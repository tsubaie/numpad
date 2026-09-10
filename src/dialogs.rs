use super::*;
use iced::widget::column;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsTab {
    Appearance,
    Numbers,
    Tax,
    About,
}

#[derive(Debug, Clone, Copy)]
pub enum AboutLink {
    Repository,
    Author,
    Issues,
    Releases,
}

pub(super) fn open_link(link: AboutLink) -> Task<Message> {
    let url = match link {
        AboutLink::Repository => "https://github.com/tsubaie/numpad",
        AboutLink::Author => "https://www.ta.sa",
        AboutLink::Releases => "https://github.com/tsubaie/numpad/releases/latest",
        AboutLink::Issues => "https://github.com/tsubaie/numpad/issues",
    };
    Task::perform(
        async move {
            let (sender, receiver) = iced::futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                #[cfg(target_os = "windows")]
                let result = std::process::Command::new("cmd")
                    .args(["/C", "start", "", url])
                    .status();
                #[cfg(target_os = "macos")]
                let result = std::process::Command::new("open").arg(url).status();
                #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                let result = std::process::Command::new("xdg-open").arg(url).status();
                let result = result
                    .map_err(|e| format!("Could not open browser: {e}"))
                    .and_then(|status| {
                        if status.success() {
                            Ok(())
                        } else {
                            Err("Could not open the link in your browser".into())
                        }
                    });
                let _ = sender.send(result);
            });
            receiver
                .await
                .unwrap_or_else(|_| Err("Could not open browser".into()))
        },
        Message::LinkOpened,
    )
}

pub struct SettingsDraft {
    pub tab: SettingsTab,
    pub prefs: Preferences,
    pub tax: TaxSettings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuideTab {
    GettingStarted,
    Tape,
    TaxMemory,
    Shortcuts,
}

impl App {
    pub(super) fn open_settings(&mut self, tab: SettingsTab) {
        self.modal = Some(Modal::Settings(Box::new(SettingsDraft {
            tab,
            prefs: self.prefs.clone(),
            tax: self.prefs.tax_settings(),
        })));
    }

    fn tab<'a>(&self, label: &'a str, active: bool, message: Message) -> Element<'a, Message> {
        let p = self.colors();
        button(text(label).size(13))
            .padding([10, 12])
            .on_press(message)
            .style(move |_, status| {
                key_style(
                    if active { p.selected } else { p.card },
                    if active { p.accent } else { p.text },
                    status,
                )
            })
            .into()
    }

    fn guide_card<'a>(
        &self,
        title: &'a str,
        example: &'a str,
        explanation: &'a str,
    ) -> Element<'a, Message> {
        let p = self.colors();
        self.card(
            container(
                column![
                    text(title).size(16).color(p.accent),
                    text(example).font(Font::MONOSPACE).size(15),
                    text(explanation).size(14),
                ]
                .spacing(10),
            )
            .padding(16),
            p.background,
        )
        .width(Fill)
        .into()
    }

    fn updates_view(&self) -> Element<'_, Message> {
        use updates::State;
        let p = self.colors();
        let status = match &self.update_state {
            State::Idle => "Check GitHub for the latest stable release. Nothing is downloaded until you request it.".into(),
            State::Checking => "Checking for updates…".into(),
            State::Current(version) => format!("You're up to date. Latest stable release: {version}."),
            State::Available(version) => format!("NumPad {version} is available."),
            State::Installing(version) => format!("Installing NumPad {version}. Follow the installer window; it may ask for administrator approval."),
            State::Installed(version) => format!("NumPad {version} installed. Close and reopen NumPad to use the update."),
            State::Failed(error) => error.clone(),
        };
        let mut controls = row![
            button(
                text(if matches!(self.update_state, State::Checking) {
                    "Checking…"
                } else {
                    "Check for updates"
                })
                .size(13)
            )
            .padding([8, 12])
            .style(move |_, status| {
                let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                let disabled = status == button::Status::Disabled;
                button::Style {
                    background: Some(if hovered { p.selected } else { p.card }.into()),
                    text_color: if disabled { p.muted } else { p.text },
                    border: Border {
                        color: if hovered { p.accent } else { p.rule },
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }
            })
            .on_press_maybe(
                (!matches!(self.update_state, State::Checking | State::Installing(_)))
                    .then_some(Message::CheckUpdates)
            )
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center);
        if matches!(self.update_state, State::Available(_)) {
            controls = controls.push(self.small("Install update", Message::InstallUpdate));
        }
        controls = controls.push(
            button(text("Release downloads ↗").size(12).color(p.muted))
                .padding([8, 4])
                .style(button::text)
                .on_press(Message::OpenLink(AboutLink::Releases)),
        );
        self.card(container(column![text("Updates").size(16).color(p.accent), text(status).size(13), controls,
            text(if cfg!(target_os="windows") {"Installation verifies the release checksum, saves your session, then closes and restarts NumPad. The installation folder must be writable."} else {"Install update opens the checksum-verifying installer in a terminal. Your documents stay in place. Restart NumPad after installation."}).size(12).color(p.muted)
        ].spacing(10)).padding(14),p.background).into()
    }

    fn settings_content<'a>(&'a self, draft: &'a SettingsDraft) -> Element<'a, Message> {
        let p = self.colors();
        match draft.tab {
            SettingsTab::About => column![
                row![
                    widget::image(widget::image::Handle::from_bytes(APP_ICON)).width(48).height(48),
                    column![text("NumPad").size(24), text(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(13).color(p.muted)].spacing(4),
                ].spacing(14).align_y(alignment::Vertical::Center),
                text("Your numbers. Your notes. One clear tape.").size(17).color(p.accent),
                text("A native calculator with editable calculation tapes. Keep numbers and notes together, revisit earlier values, and organize your work in separate tabs.").size(14),
                text("Calculations and session recovery stay on your device. Save portable .numpad documents or export your work to text, PDF, and Excel.").size(14),
                self.updates_view(),
                button(text("Created by tsubaie ↗").size(16).color(p.accent))
                    .padding([4, 0])
                    .style(button::text)
                    .on_press(Message::OpenLink(AboutLink::Author)),
                row![self.small("View project on GitHub ↗", Message::OpenLink(AboutLink::Repository)), self.small("Report an issue ↗", Message::OpenLink(AboutLink::Issues))].spacing(8),
                text("Open source · MIT license\nBuilt with Rust and Iced\n© 2026 NumPad contributors").size(13).color(p.muted),
                text(self.toast.as_ref().map(|(notice, _)| notice.as_str()).filter(|notice| notice.starts_with("Could not open")).unwrap_or("")).size(13).color(p.negative),
            ].spacing(18).into(),
            SettingsTab::Appearance => column![
                text("Make the tape comfortable to read").size(18),
                text("Theme").size(14).color(p.accent),
                row![
                    self.tab("System", draft.prefs.theme_mode == storage::ThemeMode::System, Message::ThemeMode(storage::ThemeMode::System)),
                    self.tab("Light", draft.prefs.theme_mode == storage::ThemeMode::Light, Message::ThemeMode(storage::ThemeMode::Light)),
                    self.tab("Dark", draft.prefs.theme_mode == storage::ThemeMode::Dark, Message::ThemeMode(storage::ThemeMode::Dark)),
                ].spacing(8),
                text("System follows your desktop automatically, including the active Omarchy color palette. Light and Dark override it for NumPad.")
                    .size(13)
                    .color(p.muted),
                text("Tape").size(14).color(p.accent),
                checkbox(draft.prefs.ruled)
                    .label("Show paper lines")
                    .on_toggle(|_| Message::Ruled),
                checkbox(draft.prefs.mono)
                    .label("Use a monospaced font")
                    .on_toggle(|_| Message::Mono),
                text("Monospaced characters keep numbers and annotations aligned.")
                    .size(13)
                    .color(p.muted),
            ]
            .spacing(16)
            .into(),
            SettingsTab::Numbers => column![
                text("Number formatting").size(18),
                checkbox(draft.prefs.format.comma)
                    .label("Use a decimal comma")
                    .on_toggle(Message::Comma),
                self.guide_card(
                    "Preview",
                    if draft.prefs.format.comma {
                        "1.234,56"
                    } else {
                        "1,234.56"
                    },
                    "Grouping uses three digits. Saving updates the tape to your selected format."
                ),
                text(
                    "This changes how numbers are displayed and entered, not the calculation rules."
                )
                .size(14)
                .color(p.muted),
            ]
            .spacing(16)
            .into(),
            SettingsTab::Tax => {
                let error = draft.tax.validate().err().unwrap_or_default();
                column![
                    text("One rate for both tax buttons").size(18),
                    text("Tax name").size(14),
                    text_input("VAT", &draft.tax.label).on_input(Message::CustomLabel).padding(10),
                    text("Rate (%)").size(14),
                    text_input("15", &draft.tax.rate).on_input(Message::CustomFormula).padding(10),
                    text(error).size(13).color(p.negative),
                    checkbox(draft.tax.calculate).label("Insert a subtotal after applying tax").on_toggle(Message::CustomCalculate),
                    self.guide_card("Adding and removing tax", "At 15%: 100 → 115 · 115 → 100",
                        "Add tax increases a net price. Remove tax extracts the included tax from a gross price; it divides by 1 + rate."),
                    self.small("Restore tax defaults", Message::CustomDefault),
                ].spacing(12).into()
            }
        }
    }

    fn guide_content(&self, tab: GuideTab) -> Element<'_, Message> {
        let p = self.colors();
        match tab {
            GuideTab::GettingStarted => column![
                text("Your first calculation").size(20),
                text("Type numbers and operators, then press Enter or =. The tape keeps each step so you can edit it later.").size(14),
                self.guide_card("Tape · top to bottom", "10 + 2 → 12\n12 × 3 → 36", "Type 10+2*3, then Enter. Each operation uses the running total."),
                self.guide_card("Assignment · standard precedence", "x = 10+2*3\nx = 16", "Multiplication happens before addition. Use parentheses to make your intended order explicit."),
                text("Use a blank line to start a separate calculation block. Enter finishes a calculation with two or more operands; after a single value, Enter starts another row. Use + in the tab bar for a separate tape.").size(14),
                text("Try this on the tape").size(16).color(p.accent),
                self.small("Load welcome example", Message::Example(3)),
                text("Loading an example replaces the tape. Undo restores your previous work.").size(13).color(p.muted),
            ].spacing(16).into(),
            GuideTab::Tape => column![
                text("Keep the context beside the numbers").size(20),
                self.guide_card("Notes and blocks", "+ 340  Design materials", "Write a comment after a value. A blank line or a standalone note begins an independent calculation."),
                self.guide_card("Reusable values", "rate = 75\nhours = 8\n\n+ rate\n* hours", "Define values once and reuse their names. Names ignore case. Edit a definition to update dependent results."),
                self.guide_card("Subtotals", "600.00 = budget", "Append = budget to a generated total to name it. Change source operands to edit a computed total."),
                text("Assignments also support sqrt(9), abs(-5), ln(10), log(100), sin(0), and exp(1). Angles use radians.").size(14),
                row![self.small("Load named values", Message::Example(1)), self.small("Load subtotals", Message::Example(2))].spacing(8),
                text("Examples replace the tape; Undo restores it. Typing operators creates rows; pasting inserts literal text.").size(13).color(p.muted),
            ].spacing(16).into(),
            GuideTab::TaxMemory => column![
                text("Percentages, tax, and memory").size(20),
                self.guide_card("Percentages", "100 + 15% → 115", "Adding or subtracting a percentage applies it to the running amount."),
                self.guide_card("Remove included tax", "115 ÷ 1.15 → 100", "Removing 15% included tax is not subtracting 15% of the gross amount. Edit tax… opens the shared rate in Settings."),
                self.guide_card("Memory keys", "M+  Add     M−  Subtract\nMR  Recall  MC  Clear", "M+ and M− use a selected number or the value at the caret. The hint below the keys shows that value. MR inserts memory at the caret; MC resets it."),
                self.small("Load percentage example", Message::Example(0)),
                text("The example replaces the tape; Undo restores it.").size(13).color(p.muted),
            ].spacing(16).into(),
            GuideTab::Shortcuts => {
                let modifier = if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" };
                let mut shortcuts = column![text("Keyboard shortcuts").size(20)].spacing(12);
                for (key, action) in [
                    ("Enter / =".to_owned(), "Insert subtotal"),
                    (format!("{modifier}+Z / {modifier}+Shift+Z"), "Undo / Redo"),
                    (format!("{modifier}+N / {modifier}+T"), "New tape tab"),
                    (format!("{modifier}+W"), "Close current tab"),
                    (format!("{modifier}+Tab / {modifier}+Shift+Tab"), "Next / previous tab"),
                    (format!("{modifier}+O"), "Open document"),
                    (format!("{modifier}+,"), "Open settings"),
                    (format!("{modifier}+S / {modifier}+Shift+S"), "Save / Save as"),
                    (format!("{modifier}+Shift+L"), "Toggle paper lines"),
                    (format!("{modifier}+plus / minus / 0"), "Zoom in / out / reset"),
                    ("Ctrl + mouse wheel over the tape".to_owned(), "Zoom with the mouse"),
                    ("F1 / Esc".to_owned(), "Open guide / Close dialog"),
                ] {
                    shortcuts = shortcuts.push(column![text(action).size(14), text(key).size(13).font(Font::MONOSPACE).color(p.accent)].spacing(3));
                }
                shortcuts.push(text("Your session recovers automatically. Save a .numpad document to keep a separate file; export text, PDF, or Excel from the menu.").size(14)).into()
            }
        }
    }

    pub(super) fn modal_view<'a>(&'a self, modal: &'a Modal) -> Element<'a, Message> {
        if let Modal::CloseTab(id) = modal {
            return self.close_tab_view(*id);
        }
        let p = self.colors();
        let (title, tabs, content, footer): (
            &str,
            Element<'_, Message>,
            Element<'_, Message>,
            Element<'_, Message>,
        ) = match modal {
            Modal::CloseTab(_) => unreachable!("handled above"),
            Modal::Settings(draft) => {
                let mut save = button(text("Save changes").size(14))
                    .padding([10, 16])
                    .style(move |_, status| key_style(p.accent, p.on_accent, status));
                if draft.tax.validate().is_ok() {
                    save = save.on_press(Message::CustomApply);
                }
                (
                    "Settings",
                    row![
                        self.tab(
                            "Appearance",
                            draft.tab == SettingsTab::Appearance,
                            Message::SettingsTab(SettingsTab::Appearance)
                        ),
                        self.tab(
                            "Numbers",
                            draft.tab == SettingsTab::Numbers,
                            Message::SettingsTab(SettingsTab::Numbers)
                        ),
                        self.tab(
                            "Tax",
                            draft.tab == SettingsTab::Tax,
                            Message::SettingsTab(SettingsTab::Tax)
                        ),
                        self.tab(
                            "About",
                            draft.tab == SettingsTab::About,
                            Message::SettingsTab(SettingsTab::About)
                        ),
                    ]
                    .spacing(6)
                    .into(),
                    self.settings_content(draft),
                    row![
                        Space::new().width(Fill),
                        self.small("Cancel", Message::Close),
                        save
                    ]
                    .spacing(10)
                    .align_y(alignment::Vertical::Center)
                    .into(),
                )
            }
            Modal::Help(tab) => (
                "NumPad guide",
                row![
                    self.tab(
                        "Getting started",
                        *tab == GuideTab::GettingStarted,
                        Message::GuideTab(GuideTab::GettingStarted)
                    ),
                    self.tab(
                        "Tape & variables",
                        *tab == GuideTab::Tape,
                        Message::GuideTab(GuideTab::Tape)
                    ),
                    self.tab(
                        "Tax & memory",
                        *tab == GuideTab::TaxMemory,
                        Message::GuideTab(GuideTab::TaxMemory)
                    ),
                    self.tab(
                        "Shortcuts",
                        *tab == GuideTab::Shortcuts,
                        Message::GuideTab(GuideTab::Shortcuts)
                    ),
                ]
                .spacing(4)
                .into(),
                self.guide_content(*tab),
                text(concat!(
                    "NumPad ",
                    env!("CARGO_PKG_VERSION"),
                    " · Built with Rust + Iced"
                ))
                .size(12)
                .color(p.muted)
                .into(),
            ),
        };
        self.card(
            container(
                column![
                    row![
                        text(title).size(23),
                        Space::new().width(Fill),
                        button(standard_icon("close", p.text))
                            .padding(8)
                            .on_press(Message::Close)
                            .style(button::text)
                    ]
                    .align_y(alignment::Vertical::Center),
                    tabs,
                    scrollable(container(content).padding([0, 12])).height(Fill),
                    footer,
                ]
                .spacing(18),
            )
            .padding(22),
            p.card,
        )
        .width((self.width - 32.0).clamp(300.0, 660.0))
        .height((self.height - 32.0).clamp(300.0, 680.0))
        .into()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn app() -> App {
        let prefs = Preferences::default();
        App {
            editor: Editor::new("1.5".into(), &prefs.format),
            content: TapeContent::with_text("1.5"),
            prefs,
            memory: Number::default(),
            file: None,
            menu: false,
            exports: false,
            modal: None,
            dirty: false,
            autosaved: true,
            workspace_revision: 0,
            autosave_pending: false,
            exit_request: None,
            theme_watcher: system_theme::ThemeWatcher::new(),
            toast: None,
            copied: None,
            width: 1180.0,
            height: 820.0,
            scroll: 0.0,
            tape_height: 600.0,
            system_mode: iced::theme::Mode::Light,
            omarchy_colors: None,
            tabs: vec![crate::tabs::Tab { id: 0, state: None }],
            active_tab: 0,
            next_tab_id: 1,
            modified: false,
            revision: 0,
            pending_close: None,
            modifiers: keyboard::Modifiers::default(),
            update_state: updates::State::default(),
        }
    }

    #[test]
    fn checking_updates_never_starts_installation_or_changes_documents() {
        let mut app = app();
        let before = app.editor.text.clone();
        let _ = app.update(Message::CheckUpdates);
        assert!(matches!(app.update_state, updates::State::Checking));
        let _ = app.update(Message::UpdateChecked(Ok(updates::Version([99, 0, 0]))));
        assert!(matches!(app.update_state, updates::State::Available(_)));
        assert_eq!(app.editor.text, before);
        assert!(!app.dirty);
        let _ = app.update(Message::UpdateChecked(Ok(updates::Version([0, 0, 1]))));
        assert!(matches!(app.update_state, updates::State::Current(_)));
        let _ = app.update(Message::InstallUpdate);
        assert!(matches!(app.update_state, updates::State::Current(_)));
        let _ = app.update(Message::UpdateChecked(Err("Offline".into())));
        assert!(matches!(app.update_state, updates::State::Failed(_)));
    }
    #[test]
    fn settings_cancel_discards_edits_across_tabs() {
        let mut app = app();
        let _ = app.update(Message::Settings);
        let _ = app.update(Message::ThemeMode(storage::ThemeMode::Dark));
        let _ = app.update(Message::SettingsTab(SettingsTab::Numbers));
        let _ = app.update(Message::Comma(true));
        let _ = app.update(Message::Close);
        assert!(!app.prefs.dark);
        assert!(!app.prefs.format.comma);
        assert_eq!(app.editor.text, "1.5");
        assert!(!app.dirty);
    }

    #[test]
    fn zoom_shortcuts_work_without_shift() {
        for (key, amount) in [("+", 1), ("=", 1), ("-", -1), ("0", 0)] {
            assert!(
                matches!(shortcut(&keyboard::Key::Character(key.into()), keyboard::Modifiers::COMMAND), Some(Message::Zoom(delta)) if delta == amount)
            );
        }
    }

    #[test]
    fn control_wheel_zooms_without_editing_the_tape() {
        let mut app = app();
        let original = app.editor.text.clone();
        let _ = app.update(Message::ModifiersChanged(keyboard::Modifiers::CTRL));
        // Iced emits negative lines for an upward wheel movement.
        let _ = app.update(Message::Edit(Action::Scroll { lines: -1 }));
        assert!((app.prefs.zoom - 1.1).abs() < 0.001);
        let _ = app.update(Message::Edit(Action::Scroll { lines: 1 }));
        assert!((app.prefs.zoom - 1.0).abs() < 0.001);
        assert_eq!(app.editor.text, original);
        let _ = app.update(Message::ModifiersChanged(keyboard::Modifiers::default()));
        let _ = app.update(Message::Edit(Action::Scroll { lines: -1 }));
        assert!((app.prefs.zoom - 1.0).abs() < 0.001);
    }

    #[test]
    fn settings_validate_then_apply_together() {
        let mut app = app();
        let _ = app.update(Message::EditCustom);
        let _ = app.update(Message::CustomFormula("invalid".into()));
        let _ = app.update(Message::ThemeMode(storage::ThemeMode::Dark));
        let _ = app.update(Message::Comma(true));
        let _ = app.update(Message::CustomApply);
        assert!(app.modal.is_some());
        assert!(!app.prefs.dark);
        let _ = app.update(Message::CustomFormula("20".into()));
        let _ = app.update(Message::CustomApply);
        assert!(app.modal.is_none());
        assert!(app.prefs.dark);
        assert!(app.prefs.format.comma);
        assert_eq!(app.prefs.tax_settings().rate, "20");
        assert!(app.editor.text.contains("1,5"));
        assert!(app.dirty);
    }
}
