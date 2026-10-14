#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod appearance;
mod clip;
mod dialogs;
#[cfg(feature = "e2e")]
mod e2e;
mod file_tasks;
mod view;
use numpad::{editor, engine, limits, math, storage};
use view::{key_style, standard_icon};
mod system_theme;
mod tabs;
mod tape_content;
mod updates;

use appearance::Colors;
use editor::{Editor, Pos};
use engine::Kind;
use iced::widget::{
    self, Space, button, canvas, checkbox, container, row, scrollable, stack, text, text_editor,
    text_input,
};
use iced::{
    Border, Color, Element, Font,
    Length::{Fill, FillPortion},
    Size, Subscription, Task, Theme, alignment, keyboard, window,
};
use math::Number;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use storage::{Document, Preferences, TaxSettings};
use tape_content::TapeContent;
use text_editor::{Action, Binding, Edit};

use numpad::INTRO;
const APP_ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/app-icon.png"));
#[derive(Debug, Clone, Copy, PartialEq)]
enum FileKind {
    Native,
    Text,
    Pdf,
    Excel,
}
#[derive(Debug, Clone, Copy, PartialEq)]
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
    #[cfg(feature = "e2e")]
    Probe,
    #[cfg(feature = "e2e")]
    Probed(Vec<e2e::Target>),
    #[cfg(feature = "e2e")]
    Screenshot(window::Screenshot),
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
    SettingsTab(dialogs::SettingsTab),
    GuideTab(dialogs::GuideTab),
    ModifiersChanged(keyboard::Modifiers),
    CheckUpdates,
    UpdateChecked(Result<updates::Version, String>),
    InstallUpdate,
    UpdateInstalled(updates::Version, Result<updates::InstallOutcome, String>),
    OpenLink(dialogs::AboutLink),
    LinkOpened(Result<(), String>),
    NewTab,
    SwitchTab(u64),
    CloseTab(u64),
    CloseCurrentTab,
    DiscardTab(u64),
    SaveCloseTab(u64),
    CycleTab(bool),
    Copied(CopyKind),
    Close,
    Zoom(i8),
    Ruled,
    Mono,
    ThemeMode(storage::ThemeMode),
    SystemTheme(iced::theme::Mode),
    SystemPalette(Option<Colors>),
    TapeScrolled(f32, f32),
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
    Saved(u64, u64, Result<Option<(PathBuf, FileKind)>, String>),
    WorkspaceSaved(u64, bool, Result<(), String>),
    Tick,
    Resize(Size),
    Quit(window::Id),
    Example(usize),
}
enum Modal {
    CloseTab(u64),
    Help(dialogs::GuideTab),
    Settings(Box<dialogs::SettingsDraft>),
}
struct App {
    editor: Editor,
    content: TapeContent,
    prefs: Preferences,
    memory: Number,
    file: Option<PathBuf>,
    menu: bool,
    exports: bool,
    modal: Option<Modal>,
    dirty: bool,
    autosaved: bool,
    workspace_revision: u64,
    autosave_pending: bool,
    recovery_protected: bool,
    exit_request: Option<tabs::ExitRequest>,
    toast: Option<(String, Instant)>,
    copied: Option<(CopyKind, Instant)>,
    width: f32,
    height: f32,
    scroll: f32,
    tape_height: f32,
    system_mode: iced::theme::Mode,
    omarchy_colors: Option<Colors>,
    tabs: Vec<tabs::Tab>,
    active_tab: usize,
    next_tab_id: u64,
    modified: bool,
    revision: u64,
    pending_close: Option<u64>,
    saves: file_tasks::Saves,
    modifiers: keyboard::Modifiers,
    update_state: updates::State,
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
            min_size: Some(Size::new(420.0, 580.0)),
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
        appearance::initialize_fonts();
        let loaded = storage::load(&storage::session_path());
        let failure = if storage::session_path().exists() {
            loaded.as_ref().err().cloned()
        } else {
            None
        };
        let doc = loaded
            .unwrap_or_else(|_| Document::new(INTRO.into(), Preferences::default(), "0".into()));
        let mut editor = Editor::new(doc.text, &doc.preferences.format);
        editor.recalculate(&doc.preferences.format, true);
        let mut app = Self {
            content: TapeContent::with_text(&editor.text),
            editor,
            prefs: doc.preferences,
            memory: storage::memory_value(&doc.memory),
            file: None,
            menu: false,
            exports: false,
            modal: None,
            dirty: false,
            autosaved: true,
            workspace_revision: 0,
            autosave_pending: false,
            recovery_protected: false,
            exit_request: None,
            toast: failure.map(|e| (e, Instant::now())),
            copied: None,
            width: 1180.0,
            height: 820.0,
            scroll: 0.0,
            tape_height: 580.0,
            system_mode: iced::theme::Mode::Dark,
            omarchy_colors: system_theme::omarchy_colors(),
            tabs: vec![tabs::Tab { id: 0, state: None }],
            active_tab: 0,
            next_tab_id: 1,
            modified: false,
            revision: 0,
            pending_close: None,
            saves: file_tasks::Saves::default(),
            modifiers: keyboard::Modifiers::default(),
            update_state: updates::State::default(),
        };
        if let Err(error) = app.restore_workspace() {
            app.notify(format!("Could not restore tabs: {error}"));
        }
        if let Some(path) = std::env::args().nth(1).map(PathBuf::from) {
            match storage::load(&path) {
                Ok(doc) => app.open_document(path, doc),
                Err(error) => app.notify(error),
            }
        }
        (
            app,
            Task::batch([
                widget::operation::focus("tape"),
                iced::system::theme().map(Message::SystemTheme),
            ]),
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
        match self.prefs.theme_mode {
            storage::ThemeMode::System => self
                .omarchy_colors
                .unwrap_or_else(|| Colors::new(self.system_mode == iced::theme::Mode::Dark)),
            storage::ThemeMode::Light => Colors::new(false),
            storage::ThemeMode::Dark => Colors::new(true),
        }
    }
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            #[cfg(feature = "e2e")]
            iced::time::every(Duration::from_millis(150)).map(|_| Message::Probe),
            iced::system::theme_changes().map(Message::SystemTheme),
            if (self.dirty && !self.recovery_protected)
                || self.toast.is_some()
                || self.copied.is_some()
            {
                iced::time::every(Duration::from_millis(900)).map(|_| Message::Tick)
            } else {
                Subscription::none()
            },
            if self.prefs.theme_mode == storage::ThemeMode::System {
                Subscription::run(system_theme::changes).map(Message::SystemPalette)
            } else {
                Subscription::none()
            },
            window::resize_events().map(|(_, s)| Message::Resize(s)),
            window::close_requests().map(Message::Quit),
            iced::event::listen_with(|event, status, _| {
                if matches!(event, iced::Event::Window(window::Event::Unfocused)) {
                    return Some(Message::ModifiersChanged(keyboard::Modifiers::default()));
                }
                if let iced::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
                    return Some(Message::ModifiersChanged(modifiers));
                }
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
    fn edit_tape(&mut self, edit: impl FnOnce(&mut Editor, &engine::Format)) {
        let before = self.editor.text_revision;
        edit(&mut self.editor, &self.prefs.format);
        if self.editor.text_revision != before {
            self.mark_dirty();
        }
    }
    fn mark_dirty(&mut self) {
        self.modified = true;
        self.revision += 1;
        self.dirty_workspace();
        self.autosaved = false;
    }
    fn notify(&mut self, s: impl Into<String>) {
        self.toast = Some((s.into(), Instant::now()));
    }
    fn sync(&mut self) {
        self.content.sync(&self.editor.text);
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
        let row_height = 28.0 * self.prefs.zoom;
        let top = 12.0 + self.editor.caret.line as f32 * row_height;
        if top < self.scroll {
            self.scroll = (top - 12.0).max(0.0);
        } else if top + row_height > self.scroll + self.tape_height {
            self.scroll = (top + row_height + 12.0 - self.tape_height).max(0.0);
        }
    }
    fn scroll_task(&self) -> Task<Message> {
        widget::operation::scroll_to(
            "tape-scroll",
            widget::scrollable::AbsoluteOffset {
                x: 0.0,
                y: self.scroll,
            },
        )
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
        self.budget_text();
        if self.exit_request.is_some()
            && !matches!(
                &message,
                Message::WorkspaceSaved(..) | Message::Saved(..) | Message::Quit(_) | Message::Tick
            )
        {
            return Task::none();
        }
        match message {
            #[cfg(feature = "e2e")]
            Message::Probe => return e2e::probe(),
            #[cfg(feature = "e2e")]
            Message::Screenshot(shot) => {
                e2e::save_screenshot(shot);
                return Task::none();
            }
            #[cfg(feature = "e2e")]
            Message::Probed(targets) => {
                e2e::snapshot(self, targets);
                return Task::none();
            }
            Message::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers;
                return Task::none();
            }
            Message::CheckUpdates => {
                if matches!(
                    self.update_state,
                    updates::State::Checking | updates::State::Installing(_)
                ) {
                    return Task::none();
                }
                self.update_state = updates::State::Checking;
                return Task::perform(
                    async {
                        tokio::task::spawn_blocking(updates::check)
                            .await
                            .map_err(|e| format!("Update check failed: {e}"))?
                    },
                    Message::UpdateChecked,
                );
            }
            Message::UpdateChecked(result) => {
                self.update_state = match result {
                    Ok(version)
                        if updates::Version::parse(env!("CARGO_PKG_VERSION"))
                            .is_ok_and(|current| version > current) =>
                    {
                        updates::State::Available(version)
                    }
                    Ok(version) => updates::State::Current(version),
                    Err(error) => updates::State::Failed(error),
                };
                return Task::none();
            }
            Message::InstallUpdate => {
                let updates::State::Available(version) = self.update_state else {
                    return Task::none();
                };
                self.update_state = updates::State::Installing(version);
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || updates::install(version))
                            .await
                            .map_err(|e| format!("Update installer failed: {e}"))?
                    },
                    move |result| Message::UpdateInstalled(version, result),
                );
            }
            Message::UpdateInstalled(version, result) => {
                match result {
                    Ok(updates::InstallOutcome::Installed) => {
                        self.update_state = updates::State::Installed(version)
                    }
                    #[cfg(target_os = "windows")]
                    Ok(updates::InstallOutcome::RestartToInstall(directory)) => {
                        self.exit_request = Some(tabs::ExitRequest::Restart(directory));
                        return self.start_workspace_save();
                    }
                    Err(error) => self.update_state = updates::State::Failed(error),
                }
                return Task::none();
            }
            Message::OpenLink(link) => {
                return dialogs::open_link(link);
            }
            Message::LinkOpened(result) => {
                if let Err(error) = result {
                    self.notify(error);
                }
                return Task::none();
            }
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
            Message::SystemTheme(mode) => {
                self.system_mode = mode;
                return Task::none();
            }
            Message::SystemPalette(colors) => {
                self.omarchy_colors = colors;
                return Task::none();
            }
            Message::TapeScrolled(offset, height) => {
                self.scroll = offset;
                self.tape_height = height;
                return Task::none();
            }
            Message::Edit(Action::Scroll { lines }) => {
                if self.modifiers.control() || self.modifiers.command() {
                    return if lines == 0 {
                        Task::none()
                    } else {
                        self.update(Message::Zoom(-lines.signum() as i8))
                    };
                }
                let max = (self.content.line_count() as f32 * 28.0 * self.prefs.zoom + 24.0
                    - self.tape_height)
                    .max(0.0);
                self.scroll = (self.scroll + lines as f32 * 28.0 * self.prefs.zoom).clamp(0.0, max);
                return self.scroll_task();
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
                        self.edit_tape(|editor, format| match edit {
                            Edit::Insert(c) => editor.key(c, format),
                            Edit::Enter => editor.enter(format),
                            Edit::Paste(s) => editor.paste(&s, format),
                            Edit::Backspace => editor.delete(true, format),
                            Edit::Delete => editor.delete(false, format),
                            Edit::Indent => editor.key(' ', format),
                            Edit::Unindent => {}
                        });
                    }
                    other => {
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
                self.sync();
                return self.scroll_task();
            }
            Message::Key(c) => {
                self.edit_tape(|editor, format| editor.key(c, format));
            }
            Message::DoubleZero => {
                self.edit_tape(|editor, format| editor.paste("00", format));
            }
            Message::Enter => {
                self.edit_tape(|editor, format| editor.enter(format));
            }
            Message::Backspace => {
                self.edit_tape(|editor, format| editor.delete(true, format));
            }
            Message::Undo => {
                self.edit_tape(|editor, format| editor.undo(format));
            }
            Message::Redo => {
                self.edit_tape(|editor, format| editor.redo(format));
            }
            Message::Clear => {
                self.menu = false;
                self.edit_tape(|editor, format| editor.clear(format));
                self.scroll = 0.0;
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
                self.modal = Some(Modal::Help(dialogs::GuideTab::GettingStarted));
                self.menu = false;
                return Task::none();
            }
            Message::Settings => {
                self.open_settings(dialogs::SettingsTab::Appearance);
                self.menu = false;
                return Task::none();
            }
            Message::SettingsTab(tab) => {
                if let Some(Modal::Settings(draft)) = &mut self.modal {
                    draft.tab = tab;
                }
                return Task::none();
            }
            Message::GuideTab(tab) => {
                self.modal = Some(Modal::Help(tab));
                return Task::none();
            }
            Message::NewTab => {
                self.add_tab(
                    Document::new(String::new(), self.prefs.clone(), "0".into()),
                    None,
                    false,
                );
            }
            Message::SwitchTab(id) => {
                self.switch_tab(id);
                return Task::batch([widget::operation::focus("tape"), self.scroll_task()]);
            }
            Message::CloseTab(id) => {
                if self.request_close_tab(id) {
                    return self.exit_last_tab();
                }
            }
            Message::CloseCurrentTab => {
                if self.request_close_tab(self.tab_id()) {
                    return self.exit_last_tab();
                }
            }
            Message::DiscardTab(id) => {
                if self.discard_tab(id) {
                    return self.exit_last_tab();
                }
            }
            Message::SaveCloseTab(id) => {
                self.switch_tab(id);
                self.modal = None;
                self.pending_close = Some(id);
                return self.update(Message::Save(FileKind::Native, false));
            }
            Message::CycleTab(backward) => {
                let len = self.tabs.len();
                let index = (self.active_tab + if backward { len - 1 } else { 1 }) % len;
                self.switch_tab(self.tabs[index].id);
                return Task::batch([widget::operation::focus("tape"), self.scroll_task()]);
            }
            Message::Copied(kind) => {
                self.copied = Some((kind, Instant::now()));
                self.notify("Copied to clipboard");
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
                if let Some(Modal::Settings(draft)) = &mut self.modal {
                    draft.prefs.ruled = !draft.prefs.ruled;
                    return Task::none();
                }
                self.prefs.ruled = !self.prefs.ruled;
                self.mark_dirty();
            }
            Message::Mono => {
                if let Some(Modal::Settings(draft)) = &mut self.modal {
                    draft.prefs.mono = !draft.prefs.mono;
                    return Task::none();
                }
                self.prefs.mono = !self.prefs.mono;
                self.mark_dirty();
            }
            Message::Comma(value) => {
                if let Some(Modal::Settings(draft)) = &mut self.modal {
                    draft.prefs.format.comma = value;
                    return Task::none();
                }
                let old = self.prefs.format.clone();
                self.prefs.format.comma = value;
                if self.editor.format_changed(&old, &self.prefs.format) {
                    self.mark_dirty();
                } else {
                    self.prefs.format = old;
                }
            }
            Message::ThemeMode(mode) => {
                if let Some(Modal::Settings(draft)) = &mut self.modal {
                    draft.prefs.theme_mode = mode;
                    draft.prefs.dark = mode == storage::ThemeMode::Dark;
                }
                return Task::none();
            }
            Message::Copy(kind) => {
                let copied = match kind {
                    CopyKind::Result => engine::format(&self.result(), &self.prefs.format),
                    CopyKind::Grand => engine::format(&self.editor.tape.grand, &self.prefs.format),
                    CopyKind::Memory => engine::format(&self.memory, &self.prefs.format),
                    CopyKind::Tape => storage::text_export(&self.editor.text, &self.prefs.format),
                };
                self.exports = false;
                return iced::clipboard::write(copied).chain(Task::done(Message::Copied(kind)));
            }
            Message::Memory(op) => {
                let value = self.current_value();
                match op {
                    MemoryOp::Add | MemoryOp::Subtract => match math::binary(
                        &self.memory,
                        &value,
                        if matches!(op, MemoryOp::Add) {
                            '+'
                        } else {
                            '-'
                        },
                    ) {
                        Ok(value) => self.memory = value,
                        Err(error) => {
                            self.notify(error);
                            return Task::none();
                        }
                    },
                    MemoryOp::Clear => self.memory = Number::default(),
                    MemoryOp::Recall => {
                        let s = engine::format_operand(&self.memory, &self.prefs.format);
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
                self.open_settings(dialogs::SettingsTab::Tax);
                return Task::none();
            }
            Message::CustomLabel(s) => {
                if let Some(Modal::Settings(draft)) = &mut self.modal {
                    draft.tax.label = s.chars().take(16).collect();
                }
                return Task::none();
            }
            Message::CustomFormula(s) => {
                if let Some(Modal::Settings(draft)) = &mut self.modal {
                    draft.tax.rate = s;
                }
                return Task::none();
            }
            Message::CustomCalculate(v) => {
                if let Some(Modal::Settings(draft)) = &mut self.modal {
                    draft.tax.calculate = v;
                }
                return Task::none();
            }
            Message::CustomDefault => {
                if let Some(Modal::Settings(draft)) = &mut self.modal {
                    draft.tax = TaxSettings::default();
                }
                return Task::none();
            }
            Message::CustomApply => {
                if let Some(Modal::Settings(draft)) = &self.modal {
                    if draft.tax.validate().is_err() {
                        return Task::none();
                    }
                    let old = self.prefs.format.clone();
                    if old != draft.prefs.format
                        && !self.editor.format_changed(&old, &draft.prefs.format)
                    {
                        let error = self.editor.notice.clone();
                        self.notify(error);
                        return Task::none();
                    }
                    self.prefs = draft.prefs.clone();
                    self.prefs.tax = Some(draft.tax.clone());
                    self.propagate_theme();
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
                        tokio::task::spawn_blocking(move || {
                            storage::load(&path).map(|doc| Some((path, doc)))
                        })
                        .await
                        .map_err(|e| format!("Open worker failed: {e}"))?
                    },
                    |result| Message::Opened(Box::new(result)),
                );
            }
            Message::Opened(result) => match *result {
                Ok(Some((path, doc))) => {
                    self.open_document(path, doc);
                    self.notify("Document opened");
                }
                Err(e) => self.notify(e),
                _ => {}
            },
            Message::Save(kind, save_as) => return self.queue_save(kind, save_as),
            Message::Saved(tab, revision, result) => return self.saved(tab, revision, result),
            Message::WorkspaceSaved(revision, closing, result) => {
                return self.workspace_saved(revision, closing, result);
            }
            Message::Tick => {
                if self
                    .copied
                    .is_some_and(|(_, time)| time.elapsed() > Duration::from_secs(2))
                {
                    self.copied = None;
                }
                if self
                    .toast
                    .as_ref()
                    .is_some_and(|(_, time)| time.elapsed() > Duration::from_secs(6))
                {
                    self.toast = None;
                }
                return self.start_workspace_save();
            }
            Message::Resize(size) => {
                self.width = size.width;
                self.height = size.height;
                self.tape_height = (size.height - 250.0).max(100.0);
                return Task::none();
            }
            Message::Quit(id) => {
                self.exit_request = Some(tabs::ExitRequest::Window(id));
                return self.start_workspace_save();
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
                self.scroll = 0.0;
                self.mark_dirty();
                self.notify("Example loaded. Undo restores your previous tape.");
            }
        }
        self.sync();
        if self.modal.is_some() {
            Task::none()
        } else {
            Task::batch([widget::operation::focus("tape"), self.scroll_task()])
        }
    }
}
fn key_binding(event: text_editor::KeyPress) -> Option<Binding<Message>> {
    if let Some(message) = shortcut(&event.key, event.modifiers) {
        return Some(Binding::Custom(message));
    }
    Binding::from_key_press(event)
}
fn shortcut(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> Option<Message> {
    if modifiers.command() && *key == keyboard::Key::Named(keyboard::key::Named::Tab) {
        return Some(Message::CycleTab(modifiers.shift()));
    }
    if modifiers.command()
        && let keyboard::Key::Character(c) = key.as_ref()
    {
        let message = match c.to_ascii_lowercase().as_str() {
            "s" => Some(Message::Save(FileKind::Native, modifiers.shift())),
            "o" => Some(Message::Open),
            "," => Some(Message::Settings),
            "n" | "t" => Some(Message::NewTab),
            "w" => Some(Message::CloseCurrentTab),
            "z" => Some(if modifiers.shift() {
                Message::Redo
            } else {
                Message::Undo
            }),
            "y" => Some(Message::Redo),
            "l" if modifiers.shift() => Some(Message::Ruled),
            "=" | "+" => Some(Message::Zoom(1)),
            "-" | "_" => Some(Message::Zoom(-1)),
            "0" => Some(Message::Zoom(0)),
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
