#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod appearance;
mod editor;
mod engine;
mod math;
mod storage;
#[cfg(test)]
mod tests;

use appearance::Colors;
use editor::{Editor, Pos};
use engine::Kind;
use iced::widget::{
    self, Space, button, canvas, checkbox, column, container, row, scrollable, stack, text,
    text_editor, text_input,
};
use iced::{
    Border, Color, Element, Font,
    Length::{Fill, FillPortion},
    Size, Subscription, Task, Theme, alignment, keyboard, window,
};
use math::Number;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use storage::{Document, Preferences, TaxSettings};
use text_editor::{Action, Binding, Edit};

const INTRO: &str = "Welcome to NumPad\nType a calculation. Keep the story beside the numbers.\n\n +         255.50  Change this number\n -          66.00  Expenses\n +          33.70  Comments stay on the tape\n -----------------\n +          223.20\n\n";
const APP_ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/app-icon.png"));
#[derive(Debug, Clone, Copy, PartialEq)]
enum FileKind {
    Native,
    Text,
    Pdf,
    Excel,
}
#[derive(Debug, Clone, Copy)]
enum CopyKind {
    Result,
    Grand,
    Memory,
    Tape,
}
#[derive(Debug, Clone, Copy)]
enum MemoryOp {
    Add,
    Subtract,
    Recall,
    Clear,
}
#[derive(Debug, Clone)]
enum Message {
    Edit(Action),
    GlobalKey(keyboard::Key, keyboard::Modifiers),
    Key(char),
    DoubleZero,
    Enter,
    Backspace,
    Undo,
    Redo,
    Clear,
    Menu,
    ExportMenu,
    Help,
    Settings,
    Close,
    Zoom(i8),
    Ruled,
    Mono,
    Dark,
    Comma(bool),
    Copy(CopyKind),
    Memory(MemoryOp),
    Custom(usize),
    EditCustom,
    CustomLabel(String),
    CustomFormula(String),
    CustomCalculate(bool),
    CustomDefault,
    CustomApply,
    Open,
    Save(FileKind, bool),
    Opened(Box<Result<Option<(PathBuf, Document)>, String>>),
    Saved(Result<Option<(PathBuf, FileKind)>, String>),
    Tick,
    Resize(Size),
    Quit(window::Id),
    Example(usize),
}
enum Modal {
    Help,
    Settings,
    Custom(TaxSettings),
}
struct App {
    editor: Editor,
    content: text_editor::Content,
    prefs: Preferences,
    memory: Number,
    file: Option<PathBuf>,
    menu: bool,
    exports: bool,
    modal: Option<Modal>,
    dirty: bool,
    autosaved: bool,
    toast: Option<(String, Instant)>,
    width: f32,
    height: f32,
    scroll: usize,
}

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--export-demo") {
        let dir = PathBuf::from(args.get(2).expect("--export-demo requires a directory"));
        std::fs::create_dir_all(&dir).expect("export directory");
        let f = engine::Format::default();
        storage::export_pdf(&dir.join("sample.pdf"), INTRO, &f).expect("PDF export");
        storage::export_xlsx(&dir.join("sample.xlsx"), INTRO, &f).expect("Excel export");
        storage::save(
            &dir.join("sample.numpad"),
            &Document::new(INTRO.into(), Preferences::default(), "0".into()),
        )
        .expect("save document");
        return Ok(());
    }
    iced::application(App::boot, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .window(window::Settings {
            size: Size::new(1180.0, 820.0),
            min_size: Some(Size::new(700.0, 580.0)),
            exit_on_close_request: false,
            icon: image::load_from_memory(APP_ICON).ok().and_then(|icon| {
                let rgba = icon.into_rgba8();
                window::icon::from_rgba(rgba.to_vec(), rgba.width(), rgba.height()).ok()
            }),
            ..Default::default()
        })
        .antialiasing(true)
        .run()
}
impl App {
    fn boot() -> (Self, Task<Message>) {
        let loaded = storage::load(&storage::session_path());
        let mut doc = loaded
            .as_ref()
            .ok()
            .cloned()
            .unwrap_or_else(|| Document::new(INTRO.into(), Preferences::default(), "0".into()));
        let argument = std::env::args().nth(1).map(PathBuf::from);
        let mut file = None;
        let mut failure = None;
        if let Some(path) = argument {
            match storage::load(&path) {
                Ok(d) => {
                    doc = d;
                    file = Some(path);
                }
                Err(e) => failure = Some(e),
            }
        }
        if failure.is_none()
            && storage::session_path().exists()
            && let Err(e) = loaded
        {
            failure = Some(format!("Could not restore the session: {e}"));
        }
        let mut editor = Editor::new(doc.text, &doc.preferences.format);
        editor.recalculate(&doc.preferences.format, true);
        let content = text_editor::Content::with_text(&editor.text);
        (
            Self {
                editor,
                content,
                prefs: doc.preferences,
                memory: storage::memory_value(&doc.memory),
                file,
                menu: false,
                exports: false,
                modal: None,
                dirty: false,
                autosaved: true,
                toast: failure.map(|e| (e, Instant::now())),
                width: 1180.0,
                height: 820.0,
                scroll: 0,
            },
            widget::operation::focus("tape"),
        )
    }
    fn document(&self) -> Document {
        Document::new(
            self.editor.text.clone(),
            self.prefs.clone(),
            self.memory.normalized().to_plain_string(),
        )
    }
    fn title(&self) -> String {
        format!(
            "{} — NumPad",
            self.file
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Untitled".into())
        )
    }
    fn theme(&self) -> Theme {
        let p = self.colors();
        Theme::custom(
            "NumPad",
            iced::theme::Palette {
                background: p.background,
                text: p.text,
                primary: p.accent,
                success: p.accent,
                danger: p.negative,
                warning: p.custom,
            },
        )
    }
    fn colors(&self) -> Colors {
        Colors::new(self.prefs.dark)
    }
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::time::every(Duration::from_millis(900)).map(|_| Message::Tick),
            window::resize_events().map(|(_, s)| Message::Resize(s)),
            window::close_requests().map(Message::Quit),
            iced::event::listen_with(|event, status, _| {
                if status == iced::event::Status::Ignored
                    && let iced::Event::Keyboard(keyboard::Event::KeyPressed {
                        key, modifiers, ..
                    }) = event
                {
                    return Some(Message::GlobalKey(key, modifiers));
                }
                None
            }),
        ])
    }
    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.autosaved = false;
    }
    fn notify(&mut self, s: impl Into<String>) {
        self.toast = Some((s.into(), Instant::now()));
    }
    fn sync(&mut self) {
        if self.content.text() != self.editor.text {
            self.content.perform(Action::SelectAll);
            self.content.perform(Action::Edit(Edit::Paste(Arc::new(
                self.editor.text.clone(),
            ))));
        }
        self.content.move_to(text_editor::Cursor {
            position: text_editor::Position {
                line: self.editor.caret.line,
                column: self.editor.caret.column,
            },
            selection: self.editor.anchor.map(|p| text_editor::Position {
                line: p.line,
                column: p.column,
            }),
        });
        let visible = ((self.height - 215.0) / (28.0 * self.prefs.zoom)).max(3.0) as usize;
        if self.editor.caret.line < self.scroll {
            self.scroll = self.editor.caret.line;
        } else if self.editor.caret.line >= self.scroll + visible {
            self.scroll = self.editor.caret.line - visible + 1;
        }
    }
    fn result(&self) -> Number {
        self.editor
            .tape
            .lines
            .get(self.editor.caret.line)
            .map(|l| l.result.clone())
            .unwrap_or_default()
    }
    fn current_value(&self) -> Number {
        if let Some(selection) = self.editor.selection()
            && let Ok(value) = engine::number(selection.trim(), &self.prefs.format)
        {
            return value;
        }
        let i = self.editor.caret.line;
        self.editor
            .tape
            .lines
            .get(i)
            .and_then(|l| l.value.clone())
            .or_else(|| {
                if i > 0 {
                    self.editor
                        .tape
                        .lines
                        .get(i - 1)
                        .filter(|l| l.kind == Kind::Total)
                        .and_then(|l| l.value.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::GlobalKey(key, modifiers) => {
                if self.modal.is_some() {
                    if key == keyboard::Key::Named(keyboard::key::Named::Escape) {
                        return self.update(Message::Close);
                    }
                    return Task::none();
                }
                return shortcut(&key, modifiers)
                    .map(|msg| self.update(msg))
                    .unwrap_or_else(Task::none);
            }
            Message::Edit(action) => {
                let cursor = self.content.cursor();
                self.editor.caret = Pos {
                    line: cursor.position.line,
                    column: cursor.position.column,
                };
                self.editor.anchor = cursor.selection.map(|p| Pos {
                    line: p.line,
                    column: p.column,
                });
                match action {
                    Action::Edit(edit) => {
                        match edit {
                            Edit::Insert(c) => self.editor.key(c, &self.prefs.format),
                            Edit::Enter => self.editor.enter(&self.prefs.format),
                            Edit::Paste(s) => self.editor.paste(&s, &self.prefs.format),
                            Edit::Backspace => self.editor.delete(true, &self.prefs.format),
                            Edit::Delete => self.editor.delete(false, &self.prefs.format),
                            Edit::Indent => self.editor.key(' ', &self.prefs.format),
                            Edit::Unindent => {}
                        }
                        self.mark_dirty();
                        self.sync();
                    }
                    other => {
                        if let Action::Scroll { lines } = &other {
                            self.scroll = self.scroll.saturating_add_signed(-(*lines as isize));
                        }
                        self.content.perform(other);
                        let cursor = self.content.cursor();
                        self.editor.caret = Pos {
                            line: cursor.position.line,
                            column: cursor.position.column,
                        };
                        self.editor.anchor = cursor.selection.map(|p| Pos {
                            line: p.line,
                            column: p.column,
                        });
                    }
                }
                return Task::none();
            }
            Message::Key(c) => {
                self.editor.key(c, &self.prefs.format);
                self.mark_dirty();
            }
            Message::DoubleZero => {
                self.editor.paste("00", &self.prefs.format);
                self.mark_dirty();
            }
            Message::Enter => {
                self.editor.enter(&self.prefs.format);
                self.mark_dirty();
            }
            Message::Backspace => {
                self.editor.delete(true, &self.prefs.format);
                self.mark_dirty();
            }
            Message::Undo => {
                self.editor.undo(&self.prefs.format);
                self.mark_dirty();
            }
            Message::Redo => {
                self.editor.redo(&self.prefs.format);
                self.mark_dirty();
            }
            Message::Clear => {
                self.editor.clear(&self.prefs.format);
                self.scroll = 0;
                self.mark_dirty();
            }
            Message::Menu => {
                self.menu = !self.menu;
                self.exports = false;
                return Task::none();
            }
            Message::ExportMenu => {
                self.exports = !self.exports;
                self.menu = false;
                return Task::none();
            }
            Message::Help => {
                self.modal = Some(Modal::Help);
                self.menu = false;
                return Task::none();
            }
            Message::Settings => {
                self.modal = Some(Modal::Settings);
                self.menu = false;
                return Task::none();
            }
            Message::Close => {
                self.modal = None;
                self.menu = false;
                self.exports = false;
            }
            Message::Zoom(delta) => {
                self.prefs.zoom = if delta == 0 {
                    1.0
                } else {
                    (self.prefs.zoom + f32::from(delta) * 0.1).clamp(0.6, 2.0)
                };
                self.mark_dirty();
            }
            Message::Ruled => {
                self.prefs.ruled = !self.prefs.ruled;
                self.mark_dirty();
            }
            Message::Mono => {
                self.prefs.mono = !self.prefs.mono;
                self.mark_dirty();
            }
            Message::Comma(value) => {
                let old = self.prefs.format.clone();
                self.prefs.format.comma = value;
                self.editor.format_changed(&old, &self.prefs.format);
                self.mark_dirty();
            }
            Message::Dark => {
                self.prefs.dark = !self.prefs.dark;
                self.mark_dirty();
            }
            Message::Copy(kind) => {
                let copied = match kind {
                    CopyKind::Result => engine::format(&self.result(), &self.prefs.format),
                    CopyKind::Grand => engine::format(&self.editor.tape.grand, &self.prefs.format),
                    CopyKind::Memory => engine::format(&self.memory, &self.prefs.format),
                    CopyKind::Tape => storage::text_export(&self.editor.text, &self.prefs.format),
                };
                self.notify("Copied to clipboard");
                self.exports = false;
                return iced::clipboard::write(copied);
            }
            Message::Memory(op) => {
                let value = self.current_value();
                match op {
                    MemoryOp::Add => self.memory = math::rounded(&self.memory + value),
                    MemoryOp::Subtract => self.memory = math::rounded(&self.memory - value),
                    MemoryOp::Clear => self.memory = Number::default(),
                    MemoryOp::Recall => {
                        let s = engine::format(&self.memory, &self.prefs.format);
                        self.editor.paste(&s, &self.prefs.format);
                    }
                }
                self.mark_dirty();
            }
            Message::Custom(i) => {
                let key = self.prefs.tax_settings();
                match key.formula(i == 1, &self.prefs.format) {
                    Ok(formula) => {
                        self.editor
                            .custom(&formula, key.calculate, &self.prefs.format);
                        self.mark_dirty();
                    }
                    Err(e) => self.notify(e),
                }
            }
            Message::EditCustom => {
                self.menu = false;
                self.modal = Some(Modal::Custom(self.prefs.tax_settings()));
                return Task::none();
            }
            Message::CustomLabel(s) => {
                if let Some(Modal::Custom(key)) = &mut self.modal {
                    key.label = s.chars().take(16).collect();
                }
                return Task::none();
            }
            Message::CustomFormula(s) => {
                if let Some(Modal::Custom(key)) = &mut self.modal {
                    key.rate = s;
                }
                return Task::none();
            }
            Message::CustomCalculate(v) => {
                if let Some(Modal::Custom(key)) = &mut self.modal {
                    key.calculate = v;
                }
                return Task::none();
            }
            Message::CustomDefault => {
                if let Some(Modal::Custom(key)) = &mut self.modal {
                    *key = TaxSettings::default();
                }
                return Task::none();
            }
            Message::CustomApply => {
                if let Some(Modal::Custom(key)) = &self.modal {
                    if let Err(e) = key.validate() {
                        self.notify(e);
                        return Task::none();
                    }
                    self.prefs.tax = Some(key.clone());
                    self.modal = None;
                    self.mark_dirty();
                }
            }
            Message::Open => {
                self.menu = false;
                return Task::perform(
                    async {
                        let Some(file) = rfd::AsyncFileDialog::new()
                            .add_filter("NumPad or text", &["numpad", "txt"])
                            .pick_file()
                            .await
                        else {
                            return Ok(None);
                        };
                        let path = file.path().to_path_buf();
                        storage::load(&path).map(|doc| Some((path, doc)))
                    },
                    |result| Message::Opened(Box::new(result)),
                );
            }
            Message::Opened(result) => match *result {
                Ok(Some((path, doc))) => {
                    self.prefs = doc.preferences;
                    self.editor.load(doc.text, &self.prefs.format);
                    self.memory = storage::memory_value(&doc.memory);
                    self.file = if path.extension().is_some_and(|s| s == "numpad") {
                        Some(path)
                    } else {
                        None
                    };
                    self.scroll = 0;
                    self.mark_dirty();
                    self.notify("Document opened");
                }
                Err(e) => self.notify(e),
                _ => {}
            },
            Message::Save(kind, save_as) => {
                self.menu = false;
                self.exports = false;
                let doc = self.document();
                let existing = if !save_as && kind == FileKind::Native {
                    self.file.clone()
                } else {
                    None
                };
                return Task::perform(
                    async move {
                        let (extension, label) = match kind {
                            FileKind::Native => ("numpad", "NumPad document"),
                            FileKind::Text => ("txt", "Text tape"),
                            FileKind::Pdf => ("pdf", "PDF document"),
                            FileKind::Excel => ("xlsx", "Excel workbook"),
                        };
                        let path = if let Some(p) = existing {
                            p
                        } else {
                            let Some(file) = rfd::AsyncFileDialog::new()
                                .add_filter(label, &[extension])
                                .set_file_name(format!("Calculation.{extension}"))
                                .save_file()
                                .await
                            else {
                                return Ok(None);
                            };
                            file.path().to_path_buf()
                        };
                        match kind {
                            FileKind::Native => storage::save(&path, &doc),
                            FileKind::Text => storage::atomic_write(
                                &path,
                                storage::text_export(&doc.text, &doc.preferences.format).as_bytes(),
                            ),
                            FileKind::Pdf => {
                                storage::export_pdf(&path, &doc.text, &doc.preferences.format)
                            }
                            FileKind::Excel => {
                                storage::export_xlsx(&path, &doc.text, &doc.preferences.format)
                            }
                        }?;
                        Ok(Some((path, kind)))
                    },
                    Message::Saved,
                );
            }
            Message::Saved(result) => match result {
                Ok(Some((path, kind))) => {
                    self.notify(format!(
                        "Saved {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                    if kind == FileKind::Native {
                        self.file = Some(path);
                    }
                }
                Err(e) => self.notify(e),
                _ => {}
            },
            Message::Tick => {
                if self.dirty {
                    match storage::save(&storage::session_path(), &self.document()) {
                        Ok(()) => {
                            self.dirty = false;
                            self.autosaved = true;
                        }
                        Err(e) => {
                            self.notify(format!("Autosave failed: {e}"));
                            self.dirty = false;
                        }
                    }
                }
                if self
                    .toast
                    .as_ref()
                    .is_some_and(|(_, time)| time.elapsed() > Duration::from_secs(6))
                {
                    self.toast = None;
                }
                return Task::none();
            }
            Message::Resize(size) => {
                self.width = size.width;
                self.height = size.height;
                return Task::none();
            }
            Message::Quit(id) => {
                if let Err(e) = storage::save(&storage::session_path(), &self.document()) {
                    self.notify(format!("Could not save before closing: {e}"));
                    return Task::none();
                }
                return window::close(id);
            }
            Message::Example(i) => {
                let example = match i {
                    0 => {
                        " +       1000.00  Net price\n +         15.00%  VAT\n -----------------\n +       1150.00\n"
                    }
                    1 => {
                        "rate = 75\nhours = 8\n\n +          rate  Hourly rate\n *         hours  Hours worked\n -----------------\n +        600.00 = budget\n\n +        budget\n -         10.00%  Reserve\n -----------------\n +        540.00\n"
                    }
                    2 => {
                        " +         10.00\n +          2.00\n *          3.00\n -----------------\n +         16.00\n *          2.00\n -----------------\n +         32.00\n"
                    }
                    _ => INTRO,
                };
                self.editor.load(example.into(), &self.prefs.format);
                self.modal = None;
                self.scroll = 0;
                self.mark_dirty();
                self.notify("Example loaded. Undo restores your previous tape.");
            }
        }
        self.sync();
        if self.modal.is_some() {
            Task::none()
        } else {
            widget::operation::focus("tape")
        }
    }
    fn label(&self, key: &str) -> &str {
        match key {
            "result" => "RESULT",
            "grand" => "Grand total",
            "saved" => "Saved automatically",
            _ => "",
        }
    }
    fn view(&self) -> Element<'_, Message> {
        let p = self.colors();
        let zoom = self.prefs.zoom;
        let header = row![
            self.small("☰", Message::Menu),
            widget::image(widget::image::Handle::from_bytes(APP_ICON))
                .width(42)
                .height(42),
            column![
                text("NumPad").size(22).font(Font {
                    weight: iced::font::Weight::Bold,
                    ..Font::DEFAULT
                }),
                text("A little space for your numbers")
                    .size(11)
                    .color(p.muted)
            ]
            .spacing(2),
            Space::new().width(Fill)
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center);
        let header = container(header)
            .padding([16, 22])
            .style(move |_| container::Style {
                background: Some(p.header.into()),
                text_color: Some(p.text),
                ..Default::default()
            });
        let toolbar = row![
            self.optional("undo", Message::Undo, self.editor.can_undo()),
            self.optional("redo", Message::Redo, self.editor.can_redo()),
            self.small(if self.prefs.ruled { "≡" } else { "☷" }, Message::Ruled),
            widget::tooltip(
                self.small(
                    if self.prefs.mono {
                        "Monospace"
                    } else {
                        "Proportional"
                    },
                    Message::Mono
                ),
                "Switch tape font",
                widget::tooltip::Position::Bottom
            ),
            self.small("−", Message::Zoom(-1)),
            self.small(
                &format!("{} %", (zoom * 100.0).round() as u32),
                Message::Zoom(0)
            ),
            self.small("+", Message::Zoom(1)),
            Space::new().width(Fill),
            self.small("?", Message::Help)
        ]
        .spacing(5)
        .align_y(alignment::Vertical::Center);
        let row_styles = self
            .editor
            .tape
            .lines
            .iter()
            .map(|l| {
                if l.error.is_some() {
                    3
                } else {
                    match l.kind {
                        Kind::Total => 1,
                        Kind::Comment | Kind::Assignment => 4,
                        _ => 0,
                    }
                }
            })
            .collect();
        let paper = canvas(appearance::Paper {
            colors: p,
            ruled: self.prefs.ruled,
            height: 28.0 * zoom,
            active: self.editor.caret.line,
            scroll: self.scroll,
            rows: self
                .editor
                .tape
                .lines
                .iter()
                .map(|l| {
                    if l.error.is_some() {
                        3
                    } else if matches!(l.kind, Kind::Value | Kind::Rule | Kind::Total) {
                        1
                    } else {
                        0
                    }
                })
                .collect(),
        })
        .width(Fill)
        .height(Fill);
        let edit = text_editor(&self.content)
            .id("tape")
            .on_action(Message::Edit)
            .key_binding(key_binding)
            .font(if self.prefs.mono {
                Font::MONOSPACE
            } else {
                Font::DEFAULT
            })
            .size(16.0 * zoom)
            .line_height(iced::widget::text::LineHeight::Absolute(iced::Pixels(
                28.0 * zoom,
            )))
            .wrapping(iced::widget::text::Wrapping::None)
            .padding(12)
            .height(Fill)
            .highlight_with::<appearance::TapeHighlighter>(
                appearance::HighlightSettings {
                    rows: row_styles,
                    mono: self.prefs.mono,
                    dark: self.prefs.dark,
                },
                appearance::highlight,
            )
            .style(move |_, _| text_editor::Style {
                background: Color::TRANSPARENT.into(),
                border: Border::default(),
                placeholder: p.muted,
                value: p.text,
                selection: Color::from_rgba(p.accent.r, p.accent.g, p.accent.b, 0.22),
            });
        let status = if !self.editor.notice.is_empty() {
            self.editor.notice.clone()
        } else if let Some(error) = self
            .editor
            .tape
            .lines
            .get(self.editor.caret.line)
            .and_then(|l| l.error.as_ref())
        {
            error.clone()
        } else if let Some((toast, _)) = &self.toast {
            toast.clone()
        } else if self.autosaved {
            format!("✓  {}", self.label("saved"))
        } else {
            "Saving locally…".into()
        };
        let statusbar = row![
            text(format!("{} %", (zoom * 100.0).round() as u32)).size(11),
            text(format!(
                "Ln {}:{}",
                self.editor.caret.line + 1,
                self.editor.caret.column
            ))
            .size(11),
            Space::new().width(Fill),
            text(status).size(11)
        ]
        .spacing(16)
        .align_y(alignment::Vertical::Center);
        let tape = self
            .card(
                column![
                    container(toolbar).padding([9, 12]),
                    stack![paper, edit].height(Fill),
                    container(statusbar).padding([10, 14])
                ]
                .height(Fill),
                p.card,
            )
            .width(FillPortion(1))
            .height(Fill);
        let result = column![
            row![
                text(self.label("result")).size(11).color(p.accent),
                Space::new().width(Fill),
                text("current block").size(10).color(p.muted)
            ],
            button(
                text(engine::format(&self.result(), &self.prefs.format))
                    .size(32)
                    .font(Font {
                        weight: iced::font::Weight::Bold,
                        ..Font::DEFAULT
                    })
                    .align_x(alignment::Horizontal::Right)
                    .width(Fill)
            )
            .style(button::text)
            .on_press(Message::Copy(CopyKind::Result))
            .width(Fill),
            self.result_row(
                self.label("grand"),
                &self.editor.tape.grand,
                CopyKind::Grand
            ),
            self.result_row("M (Memory)", &self.memory, CopyKind::Memory)
        ]
        .spacing(8);
        let fill_sidebar = self.height >= 780.0;
        let sidebar_height = if fill_sidebar {
            Fill
        } else {
            iced::Length::Shrink
        };
        let sidebar = column![
            self.card(container(result).padding(18), p.card),
            self.card(
                container(self.keypad(fill_sidebar))
                    .padding(14)
                    .height(sidebar_height),
                p.card
            )
            .height(sidebar_height)
        ]
        .spacing(14)
        .width(308)
        .height(sidebar_height);
        let sidebar: Element<'_, Message> = if fill_sidebar {
            sidebar.into()
        } else {
            scrollable(sidebar).width(308).height(Fill).into()
        };
        let body = row![tape, sidebar].spacing(16).height(Fill);
        let footer = row![
            text("NumPad 1.3  ·  Local & offline").size(10),
            Space::new().width(Fill),
            text(format!(
                "{} lines  ·  {} named values",
                self.editor.tape.lines.len(),
                self.editor.tape.variables.len()
            ))
            .size(10)
        ]
        .spacing(10);
        let base = column![
            header,
            container(body).padding(16).height(Fill),
            container(footer)
                .padding([8, 18])
                .style(move |_| container::Style {
                    background: Some(p.header.into()),
                    text_color: Some(p.muted),
                    ..Default::default()
                })
        ]
        .height(Fill);
        let mut layers = stack![base];
        if self.menu || self.exports {
            let items = if self.exports {
                column![
                    self.menu_item(
                        "Save document          Ctrl+S",
                        Message::Save(FileKind::Native, false)
                    ),
                    self.menu_item("Save as…", Message::Save(FileKind::Native, true)),
                    self.menu_item("Copy tape as text", Message::Copy(CopyKind::Tape)),
                    self.menu_item("Export text…", Message::Save(FileKind::Text, true)),
                    self.menu_item("Export Excel…", Message::Save(FileKind::Excel, true)),
                    self.menu_item("Export PDF…", Message::Save(FileKind::Pdf, true)),
                    self.menu_item("Close", Message::Close)
                ]
            } else {
                column![
                    self.menu_item("New tape (undoable)     Ctrl+N", Message::Clear),
                    self.menu_item("Open…                  Ctrl+O", Message::Open),
                    self.menu_item(
                        "Save                   Ctrl+S",
                        Message::Save(FileKind::Native, false)
                    ),
                    self.menu_item("Export…", Message::ExportMenu),
                    self.menu_item("Settings", Message::Settings),
                    self.menu_item("Tax settings", Message::EditCustom),
                    self.menu_item("Guide & examples       F1", Message::Help),
                    self.menu_item("Close menu             Esc", Message::Close)
                ]
            };
            let panel = self
                .card(container(items.spacing(3)).padding(8), p.card)
                .width(290);
            layers = layers.push(
                container(widget::opaque(panel))
                    .padding(iced::Padding {
                        top: 74.0,
                        right: 18.0,
                        bottom: 0.0,
                        left: 18.0,
                    })
                    .width(Fill)
                    .align_x(alignment::Horizontal::Left),
            );
        }
        if let Some(modal) = &self.modal {
            let overlay = container(self.modal_view(modal))
                .center_x(Fill)
                .center_y(Fill)
                .style(|_| container::Style {
                    background: Some(Color::from_rgba(0.05, 0.1, 0.07, 0.5).into()),
                    ..Default::default()
                });
            layers = layers.push(widget::opaque(overlay));
        }
        layers.into()
    }
    fn keypad(&self, fill: bool) -> Element<'_, Message> {
        let p = self.colors();
        let key =
            |label: &str, msg: Message, bg: Color, fg: Color| self.key_button(label, msg, bg, fg);
        let white = p.keypad;
        let mut rows = column![
            row![
                key("AC", Message::Clear, p.custom, p.negative),
                key("⌫", Message::Backspace, p.keypad, p.muted),
                key("%", Message::Key('%'), p.keypad, p.text),
                key("÷", Message::Key('/'), p.operator, p.accent)
            ]
            .spacing(6)
        ]
        .spacing(6);
        for (labels, op) in [
            (["7", "8", "9"], '*'),
            (["4", "5", "6"], '-'),
            (["1", "2", "3"], '+'),
        ] {
            rows = rows.push(
                row![
                    key(
                        labels[0],
                        Message::Key(labels[0].chars().next().unwrap()),
                        white,
                        p.text
                    ),
                    key(
                        labels[1],
                        Message::Key(labels[1].chars().next().unwrap()),
                        white,
                        p.text
                    ),
                    key(
                        labels[2],
                        Message::Key(labels[2].chars().next().unwrap()),
                        white,
                        p.text
                    ),
                    key(
                        if op == '*' {
                            "×"
                        } else if op == '-' {
                            "−"
                        } else {
                            "+"
                        },
                        Message::Key(op),
                        p.operator,
                        p.accent
                    )
                ]
                .spacing(6),
            );
        }
        rows = rows.push(
            row![
                key("0", Message::Key('0'), white, p.text),
                key("00", Message::DoubleZero, white, p.text),
                key(
                    if self.prefs.format.comma { "," } else { "." },
                    Message::Key(if self.prefs.format.comma { ',' } else { '.' }),
                    white,
                    p.text
                ),
                key("=", Message::Enter, p.accent, p.on_accent)
            ]
            .spacing(6),
        );
        rows = rows.push(
            row![
                key("M+", Message::Memory(MemoryOp::Add), p.card, p.text),
                key("M−", Message::Memory(MemoryOp::Subtract), p.card, p.text),
                key("MR", Message::Memory(MemoryOp::Recall), p.card, p.text),
                key("MC", Message::Memory(MemoryOp::Clear), p.card, p.text)
            ]
            .spacing(6),
        );
        if fill {
            rows = rows.push(Space::new().height(Fill));
        }
        let tax = self.prefs.tax_settings();
        let rate = tax.rate_label(self.prefs.format.comma);
        rows = rows.push(
            row![
                text("TAX").size(10).color(p.muted),
                Space::new().width(Fill),
                self.small("Tax settings", Message::EditCustom)
            ]
            .align_y(alignment::Vertical::Center),
        );
        let tax_key = |label: String, i| {
            button(
                container(text(label).size(13))
                    .center_x(Fill)
                    .center_y(Fill),
            )
            .height(42)
            .width(Fill)
            .padding(0)
            .on_press(Message::Custom(i))
            .style(move |_, s| key_style(p.custom, p.text, s))
        };
        rows = rows.push(
            row![
                tax_key(format!("+ {} ({rate}%)", tax.label), 0),
                tax_key(format!("− {} ({rate}%)", tax.label), 1)
            ]
            .spacing(6),
        );
        rows = rows.push(
            text(format!(
                "M± uses {}",
                engine::format(&self.current_value(), &self.prefs.format)
            ))
            .size(10)
            .color(p.muted),
        );
        rows.height(if fill { Fill } else { iced::Length::Shrink })
            .into()
    }
    fn result_row<'a>(
        &'a self,
        label: &'a str,
        value: &Number,
        kind: CopyKind,
    ) -> Element<'a, Message> {
        let p = self.colors();
        button(
            row![
                text(label).size(12).color(p.muted),
                Space::new().width(Fill),
                text(engine::format(value, &self.prefs.format)).size(14),
                text("⧉").size(12).color(p.muted)
            ]
            .spacing(8),
        )
        .on_press(Message::Copy(kind))
        .width(Fill)
        .style(button::text)
        .padding(0)
        .into()
    }
    fn card<'a>(
        &self,
        content: impl Into<Element<'a, Message>>,
        background: Color,
    ) -> widget::Container<'a, Message> {
        let p = self.colors();
        container(content).style(move |_| container::Style {
            background: Some(background.into()),
            text_color: Some(p.text),
            border: Border {
                radius: 12.0.into(),
                color: p.rule,
                width: 1.0,
            },
            ..Default::default()
        })
    }
    fn action<'a>(&self, label: &str, msg: Message, bg: Color, fg: Color) -> Element<'a, Message> {
        button(text(label.to_owned()).size(13))
            .on_press(msg)
            .padding([8, 14])
            .style(move |_, s| key_style(bg, fg, s))
            .into()
    }
    fn key_button<'a>(
        &self,
        label: &str,
        msg: Message,
        bg: Color,
        fg: Color,
    ) -> Element<'a, Message> {
        let content: Element<'a, Message> = if label == "⌫" {
            standard_icon("backspace", fg).into()
        } else {
            text(label.to_owned()).size(19).into()
        };
        button(container(content).center_x(Fill).center_y(Fill))
            .padding(0)
            .on_press(msg)
            .height(if self.height < 740.0 { 43 } else { 49 })
            .width(Fill)
            .style(move |_, s| key_style(bg, fg, s))
            .into()
    }
    fn small<'a>(&self, label: &str, msg: Message) -> Element<'a, Message> {
        let p = self.colors();
        button(text(label.to_owned()).size(12))
            .on_press(msg)
            .padding([7, 9])
            .style(move |_, s| key_style(p.card, p.accent, s))
            .into()
    }
    fn optional<'a>(&self, label: &str, msg: Message, enabled: bool) -> Element<'a, Message> {
        let p = self.colors();
        let b = button(standard_icon(
            label,
            if enabled {
                p.accent
            } else {
                Color { a: 0.3, ..p.accent }
            },
        ))
        .padding([4, 9])
        .style(move |_, s| key_style(p.card, p.accent, s));
        if enabled {
            b.on_press(msg).into()
        } else {
            b.into()
        }
    }
    fn menu_item<'a>(&self, label: &str, msg: Message) -> Element<'a, Message> {
        button(text(label.to_owned()).size(13))
            .on_press(msg)
            .padding([10, 12])
            .width(Fill)
            .style(button::text)
            .into()
    }
    fn modal_view<'a>(&'a self, modal: &'a Modal) -> Element<'a, Message> {
        let p = self.colors();
        let(title,content):(String,Element<'a,Message>)=match modal{
            Modal::Settings=>("Preferences".into(),column![text("Appearance").size(13),text("One light theme. One cozy dark theme.").size(12).color(p.muted),checkbox(self.prefs.dark).label("Catppuccin Mocha dark theme").on_toggle(|_|Message::Dark),checkbox(self.prefs.ruled).label("Ruled paper").on_toggle(|_|Message::Ruled),checkbox(self.prefs.mono).label("Monospaced tape").on_toggle(|_|Message::Mono),checkbox(self.prefs.format.comma).label("Decimal comma (1.234,56)").on_toggle(Message::Comma),text("Documents and preferences are saved locally.\nNative .numpad files can be reopened on any supported OS.").size(12).color(p.muted)].spacing(14).into()),
            Modal::Custom(key)=>("Tax settings".into(),column![text("Tax name").size(13),text_input("VAT",&key.label).on_input(Message::CustomLabel).padding(10),text("Rate (%) — shared by both buttons").size(13),text_input("15",&key.rate).on_input(Message::CustomFormula).padding(10),text(key.validate().err().unwrap_or_default()).size(12).color(p.negative),text("Add tax increases the net price. Remove tax extracts tax already included in a gross price. Both use this same rate.").size(12).color(p.muted),checkbox(key.calculate).label("Calculate after applying tax").on_toggle(Message::CustomCalculate),row![self.small("Default",Message::CustomDefault),Space::new().width(Fill),self.small("Cancel",Message::Close),self.action("Save",Message::CustomApply,p.accent,p.on_accent)].spacing(8)].spacing(14).into()),
            Modal::Help=>("The NumPad guide".into(),scrollable(column![text("A calculator you can read back.").size(23),text("Type 10+2*3 and press Enter: the tape runs top-to-bottom, so the answer is 36. Assignments such as x = 10+2*3 keep standard precedence and give 16.").size(14),text("Calculate · Comment · Correct").size(17),text("Write a note after any number. Click an earlier value to change it: every dependent total updates. A blank line or a standalone note begins a new calculation. Enter after a single value simply starts a new row.").size(14),text("Percentages and named values").size(17),text("100+15% adds tax. 115/1.15 removes included tax. Define rate = 75, then use +rate. Names ignore letter case. Append = budget to a generated total to name it. Assignments accept arithmetic and functions such as sqrt(9), abs(-5), ln(10), log(100), sin(0) and exp(1). Angles use radians.").size(14),text("Keep calculations close").size(17),text("Ctrl+S saves a .numpad document; Ctrl+O opens one or a text tape. Export produces PDF, Excel or plain text. Your session recovers automatically after closing. M+ and M− use the value at your caret; MR inserts memory.").size(14),text("Keyboard").size(17),text("Enter / =    Calculate\nCtrl+Z / Ctrl+Y    Undo / redo\nCtrl+N / Ctrl+O / Ctrl+S    New / open / save\nCtrl+Shift+L    Toggle paper rules\nCtrl+Shift+plus/minus/0    Zoom / reset\nF1    Guide\nOn macOS, use Command instead of Ctrl.").font(Font::MONOSPACE).size(12),text("Try an example (Undo restores your tape)").size(14),row![self.small("Percentages",Message::Example(0)),self.small("Named values",Message::Example(1)),self.small("Subtotals",Message::Example(2))].spacing(8),text("NumPad 1.3 · Rust + Iced\nIndependent implementation. Behavioral reference: CalcTape Web.\nThe project docs include the reference specification and compatibility notes.").size(11).color(p.muted)].spacing(16)).height(480).into()),
        };
        self.card(
            container(
                column![
                    row![
                        text(title).size(21),
                        Space::new().width(Fill),
                        self.small("✕", Message::Close)
                    ]
                    .align_y(alignment::Vertical::Center),
                    content
                ]
                .spacing(22),
            )
            .padding(26),
            p.card,
        )
        .width(590)
        .into()
    }
}
fn key_style(bg: Color, fg: Color, status: button::Status) -> button::Style {
    let background = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        Color {
            r: bg.r * 0.92,
            g: bg.g * 0.92,
            b: bg.b * 0.92,
            a: 1.0,
        }
    } else {
        bg
    };
    button::Style {
        background: Some(background.into()),
        text_color: if status == button::Status::Disabled {
            Color { a: 0.3, ..fg }
        } else {
            fg
        },
        border: Border {
            radius: 9.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn key_binding(event: text_editor::KeyPress) -> Option<Binding<Message>> {
    if let Some(message) = shortcut(&event.key, event.modifiers) {
        return Some(Binding::Custom(message));
    }
    Binding::from_key_press(event)
}
fn shortcut(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> Option<Message> {
    if modifiers.command()
        && let keyboard::Key::Character(c) = key.as_ref()
    {
        let message = match c.to_ascii_lowercase().as_str() {
            "s" => Some(Message::Save(FileKind::Native, modifiers.shift())),
            "o" => Some(Message::Open),
            "n" => Some(Message::Clear),
            "z" => Some(if modifiers.shift() {
                Message::Redo
            } else {
                Message::Undo
            }),
            "y" => Some(Message::Redo),
            "l" if modifiers.shift() => Some(Message::Ruled),
            "=" | "+" if modifiers.shift() => Some(Message::Zoom(1)),
            "-" if modifiers.shift() => Some(Message::Zoom(-1)),
            "0" if modifiers.shift() => Some(Message::Zoom(0)),
            _ => None,
        };
        if let Some(m) = message {
            return Some(m);
        }
    }
    match key.as_ref() {
        keyboard::Key::Named(keyboard::key::Named::F1) => Some(Message::Help),
        keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::Close),
        _ => None,
    }
}
fn standard_icon(kind: &str, color: Color) -> widget::Svg<'static> {
    let path = match kind {
        "undo" => "M3 10h11a7 7 0 0 1 7 7v3 M3 10l6-6 M3 10l6 6",
        "redo" => "M21 10H10a7 7 0 0 0-7 7v3 M21 10l-6-6 M21 10l-6 6",
        _ => "M9 4h12v16H9l-7-8z M12 9l6 6 M18 9l-6 6",
    };
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="{path}" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>"#
    );
    widget::svg(widget::svg::Handle::from_memory(svg.into_bytes()))
        .width(19)
        .height(19)
        .style(move |_, _| widget::svg::Style { color: Some(color) })
}
