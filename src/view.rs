use super::*;
use iced::widget::column;

impl App {
    pub(super) fn label(&self, key: &str) -> &str {
        match key {
            "result" => "RESULT",
            "grand" => "Grand total",
            "saved" => "Saved automatically",
            _ => "",
        }
    }
    pub(super) fn view(&self) -> Element<'_, Message> {
        let p = self.colors();
        let zoom = self.prefs.zoom;
        let narrow = self.width < 860.0;
        let header = row![
            self.hint(
                button(
                    container(standard_icon("menu", p.text))
                        .id("menu-toggle")
                        .center_x(Fill)
                        .center_y(Fill)
                )
                .width(36)
                .height(36)
                .padding(0)
                .on_press(Message::Menu)
                .style(move |_, status| key_style(p.card, p.text, status)),
                "Menu",
                widget::tooltip::Position::Bottom
            ),
            widget::image(widget::image::Handle::from_bytes(APP_ICON))
                .width(42)
                .height(42),
            column![
                text("NumPad").size(22).font(Font {
                    weight: iced::font::Weight::Bold,
                    ..Font::DEFAULT
                }),
                text("Editable tape · operations run top to bottom")
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
        let mode = match self
            .editor
            .tape
            .lines
            .get(self.editor.caret.line)
            .map(|l| &l.kind)
        {
            Some(Kind::Assignment) => "Named expression · standard precedence",
            Some(Kind::Comment | Kind::Blank) => {
                "Note or blank line · starts a new calculation block"
            }
            _ => "Tape · top to bottom · paste inserts literal text",
        };
        let row_styles = self
            .editor
            .tape
            .lines
            .iter()
            .map(appearance::HighlightRow::from_line)
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
                    } else if l.kind == Kind::Total {
                        2
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
            .height((self.content.line_count() as f32 * 28.0 * zoom + 24.0).max(self.tape_height))
            .highlight_with::<appearance::TapeHighlighter>(
                appearance::HighlightSettings {
                    rows: row_styles,
                    mono: self.prefs.mono,
                    colors: p,
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
        let tape = container(
            column![
                container(toolbar).padding([9, 12]),
                container(text(mode).size(11).color(p.muted)).padding([0, 12]),
                container(
                    stack![
                        paper,
                        scrollable(crate::clip::clip(edit, 12.0))
                            .id("tape-scroll")
                            .on_scroll(|v| Message::TapeScrolled(
                                v.absolute_offset().y,
                                v.bounds().height
                            ))
                            .height(Fill)
                    ]
                    .height(Fill)
                )
                .id("tape-viewport")
                .clip(true)
                .height(Fill),
                container(statusbar).padding([10, 14])
            ]
            .height(Fill),
        )
        .id("tape-surface")
        .style(move |_| container::Style {
            background: Some(p.paper.into()),
            text_color: Some(p.text),
            border: Border {
                radius: iced::border::Radius {
                    top_left: 0.0,
                    top_right: 12.0,
                    bottom_left: 12.0,
                    bottom_right: 12.0,
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .width(FillPortion(1))
        .height(Fill);
        let tape = column![self.tab_bar(), tape]
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
        let fill_sidebar = !narrow && self.height >= 780.0;
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
        .width(if narrow {
            Fill
        } else {
            iced::Length::Fixed(308.0)
        })
        .height(sidebar_height);
        let sidebar: Element<'_, Message> = if fill_sidebar {
            sidebar.into()
        } else {
            scrollable(sidebar)
                .width(if narrow {
                    Fill
                } else {
                    iced::Length::Fixed(308.0)
                })
                .height(Fill)
                .into()
        };
        let controls = container(sidebar).id("calculator-controls");
        let body: Element<'_, Message> = if narrow {
            column![tape.height(FillPortion(3)), controls.height(FillPortion(2))]
                .spacing(12)
                .height(Fill)
                .into()
        } else {
            row![tape, controls.height(Fill)]
                .spacing(16)
                .height(Fill)
                .into()
        };
        let footer = row![
            text(concat!("NumPad ", env!("CARGO_PKG_VERSION"))).size(10),
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
                    self.menu_item("New tab                Ctrl+T", Message::NewTab),
                    self.menu_item("Open…                  Ctrl+O", Message::Open),
                    self.menu_item(
                        "Save                   Ctrl+S",
                        Message::Save(FileKind::Native, false)
                    ),
                    self.menu_item("Export…", Message::ExportMenu),
                    self.menu_item("Settings", Message::Settings),
                    self.menu_item("Guide & examples       F1", Message::Help)
                ]
            };
            let panel = self
                .card(container(items.spacing(3)).padding(8), p.card)
                .width(290);
            layers = layers.push(
                widget::mouse_area(
                    container(Space::new().width(Fill).height(Fill))
                        .width(Fill)
                        .height(Fill),
                )
                .on_press(Message::Close)
                .on_right_press(Message::Close),
            );
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
    pub(super) fn keypad(&self, fill: bool) -> Element<'_, Message> {
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
                key("M+", Message::Memory(MemoryOp::Add), p.keypad, p.text),
                key("M−", Message::Memory(MemoryOp::Subtract), p.keypad, p.text),
                key("MR", Message::Memory(MemoryOp::Recall), p.keypad, p.text),
                key("MC", Message::Memory(MemoryOp::Clear), p.keypad, p.text)
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
        if fill {
            rows = rows.push(Space::new().height(Fill));
        }
        let tax = self.prefs.tax_settings();
        let rate = tax.rate_label(self.prefs.format.comma);
        rows = rows.push(
            row![
                text("TAX").size(10).color(p.muted),
                Space::new().width(Fill),
                self.small("Edit tax…", Message::EditCustom)
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
        rows.height(if fill { Fill } else { iced::Length::Shrink })
            .into()
    }
    pub(super) fn result_row<'a>(
        &'a self,
        label: &'a str,
        value: &Number,
        kind: CopyKind,
    ) -> Element<'a, Message> {
        let p = self.colors();
        row![
            text(label).size(12).color(p.muted),
            Space::new().width(Fill),
            text(engine::format(value, &self.prefs.format)).size(14),
            self.hint(
                button(
                    container(
                        standard_icon(
                            if self.copied.is_some_and(|(copied, _)| copied == kind) {
                                "check"
                            } else {
                                "copy"
                            },
                            p.accent
                        )
                        .width(16)
                        .height(16)
                    )
                    .id(if kind == CopyKind::Grand {
                        "copy-grand"
                    } else {
                        "copy-memory"
                    })
                    .center_x(Fill)
                    .center_y(Fill)
                )
                .width(34)
                .height(34)
                .padding(0)
                .on_press(Message::Copy(kind))
                .style(move |_, status| button::Style {
                    background: Some(
                        if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                            p.selected
                        } else {
                            p.keypad
                        }
                        .into()
                    ),
                    text_color: p.accent,
                    border: Border {
                        radius: 12.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                if self.copied.is_some_and(|(copied, _)| copied == kind) {
                    "Copied".to_owned()
                } else {
                    match kind {
                        CopyKind::Memory => "Copy memory".to_owned(),
                        _ => "Copy total".to_owned(),
                    }
                },
                widget::tooltip::Position::Top
            )
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center)
        .into()
    }

    pub(super) fn hint<'a>(
        &self,
        content: impl Into<Element<'a, Message>>,
        label: impl Into<String>,
        position: widget::tooltip::Position,
    ) -> Element<'a, Message> {
        let p = self.colors();
        widget::tooltip(content, text(label.into()).size(12), position)
            .gap(8)
            .padding(9)
            .delay(Duration::from_millis(400))
            .style(move |_| container::Style {
                background: Some(p.card.into()),
                text_color: Some(p.text),
                border: Border {
                    radius: 9.0.into(),
                    width: 1.0,
                    color: p.rule,
                },
                shadow: iced::Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.16),
                    offset: iced::Vector::new(0.0, 3.0),
                    blur_radius: 10.0,
                },
                ..Default::default()
            })
            .into()
    }

    pub(super) fn card<'a>(
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
    pub(super) fn key_button<'a>(
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
        let key = button(container(content).center_x(Fill).center_y(Fill))
            .padding(0)
            .on_press(msg)
            .height(if self.height < 740.0 { 43 } else { 49 })
            .width(Fill)
            .style(move |_, s| key_style(bg, fg, s));
        let hint = match label {
            "M+" => "Add value to memory",
            "M−" => "Subtract value from memory",
            "MR" => "Insert memory at caret",
            "MC" => "Clear memory",
            _ => return key.into(),
        };
        self.hint(key, hint, widget::tooltip::Position::Top)
    }
    pub(super) fn small<'a>(&self, label: &str, msg: Message) -> Element<'a, Message> {
        let p = self.colors();
        button(text(label.to_owned()).size(12))
            .on_press(msg)
            .padding([7, 9])
            .style(move |_, s| key_style(p.card, p.accent, s))
            .into()
    }
    pub(super) fn optional<'a>(
        &self,
        label: &str,
        msg: Message,
        enabled: bool,
    ) -> Element<'a, Message> {
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
    pub(super) fn menu_item<'a>(&self, label: &str, msg: Message) -> Element<'a, Message> {
        button(text(label.to_owned()).size(13))
            .on_press(msg)
            .padding([10, 12])
            .width(Fill)
            .style(button::text)
            .into()
    }
}
pub(super) fn key_style(bg: Color, fg: Color, status: button::Status) -> button::Style {
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

pub(super) fn standard_icon(kind: &str, color: Color) -> widget::Canvas<appearance::Icon, Message> {
    let kind = match kind {
        "menu" => "menu",
        "copy" => "copy",
        "check" => "check",
        "close" => "close",
        "undo" => "undo",
        "redo" => "redo",
        _ => "backspace",
    };
    canvas(appearance::Icon { kind, color })
        .width(19)
        .height(19)
}
