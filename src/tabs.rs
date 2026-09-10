use super::*;
use iced::widget::column;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub(super) enum ExitRequest {
    Window(window::Id),
    LastTab,
    #[cfg(target_os = "windows")]
    Restart(PathBuf),
}

pub struct Tab {
    pub id: u64,
    pub state: Option<TapeState>,
}

pub struct TapeState {
    saved_document: Arc<Document>,
    editor: Editor,
    content: TapeContent,
    prefs: Preferences,
    memory: Number,
    file: Option<PathBuf>,
    scroll: f32,
    modified: bool,
    revision: u64,
}

#[derive(Serialize, Deserialize)]
struct SavedTab {
    document: Arc<Document>,
    file: Option<PathBuf>,
    modified: bool,
}

#[derive(Serialize, Deserialize)]
struct Workspace {
    version: u32,
    active: usize,
    tabs: Vec<SavedTab>,
}

impl TapeState {
    fn new(document: Document, file: Option<PathBuf>, modified: bool) -> Self {
        let mut editor = Editor::new(document.text, &document.preferences.format);
        editor.recalculate(&document.preferences.format, false);
        let memory = storage::memory_value(&document.memory);
        Self {
            saved_document: Arc::new(Document::new(
                editor.text.clone(),
                document.preferences.clone(),
                memory.normalized().to_plain_string(),
            )),
            content: TapeContent::with_text(&editor.text),
            editor,
            prefs: document.preferences,
            memory,
            file,
            scroll: 0.0,
            modified,
            revision: 0,
        }
    }
    fn saved(&self) -> SavedTab {
        SavedTab {
            document: Arc::clone(&self.saved_document),
            file: self.file.clone(),
            modified: self.modified,
        }
    }
}

impl App {
    pub(super) fn tab_id(&self) -> u64 {
        self.tabs[self.active_tab].id
    }

    fn take_tape(&mut self) -> TapeState {
        TapeState {
            saved_document: Arc::new(self.document()),
            editor: std::mem::replace(
                &mut self.editor,
                Editor::new(String::new(), &self.prefs.format),
            ),
            content: std::mem::take(&mut self.content),
            prefs: self.prefs.clone(),
            memory: self.memory.clone(),
            file: self.file.take(),
            scroll: self.scroll,
            modified: self.modified,
            revision: self.revision,
        }
    }

    fn install_tape(&mut self, state: TapeState) {
        self.editor = state.editor;
        self.content = state.content;
        self.prefs = state.prefs;
        self.memory = state.memory;
        self.file = state.file;
        self.scroll = state.scroll;
        self.modified = state.modified;
        self.revision = state.revision;
        self.copied = None;
        self.toast = None;
        self.menu = false;
        self.exports = false;
    }

    pub(super) fn switch_tab(&mut self, id: u64) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        if index == self.active_tab {
            return;
        }
        if let Some(state) = self.tabs[index].state.take() {
            self.tabs[self.active_tab].state = Some(self.take_tape());
            self.active_tab = index;
            self.install_tape(state);
            self.dirty_workspace();
        }
    }

    pub(super) fn add_tab(&mut self, document: Document, file: Option<PathBuf>, modified: bool) {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(Tab {
            id,
            state: Some(TapeState::new(document, file, modified)),
        });
        self.switch_tab(id);
        self.dirty_workspace();
    }

    pub(super) fn open_document(&mut self, path: PathBuf, mut document: Document) {
        if self.file.as_ref() == Some(&path) {
            return;
        }
        if let Some(id) = self
            .tabs
            .iter()
            .find(|tab| {
                tab.state
                    .as_ref()
                    .is_some_and(|s| s.file.as_ref() == Some(&path))
            })
            .map(|t| t.id)
        {
            self.switch_tab(id);
            return;
        }
        document.preferences.theme_mode = self.prefs.theme_mode;
        document.preferences.dark = self.prefs.dark;
        let native = path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("numpad"));
        self.add_tab(document, native.then_some(path), !native);
    }

    pub(super) fn request_close_tab(&mut self, id: u64) -> bool {
        let modified = if id == self.tab_id() {
            self.modified
        } else {
            self.tabs
                .iter()
                .find(|t| t.id == id)
                .and_then(|t| t.state.as_ref())
                .is_some_and(|s| s.modified)
        };
        if modified {
            self.modal = Some(Modal::CloseTab(id));
            false
        } else {
            self.discard_tab(id)
        }
    }

    pub(super) fn discard_tab(&mut self, id: u64) -> bool {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return false;
        };
        if self.tabs.len() == 1 {
            return true;
        } else if index == self.active_tab {
            let next = if index > 0 { index - 1 } else { 1 };
            self.switch_tab(self.tabs[next].id);
        }
        self.tabs.remove(index);
        if index < self.active_tab {
            self.active_tab -= 1;
        }
        self.modal = None;
        self.dirty_workspace();
        false
    }

    pub(super) fn exit_last_tab(&mut self) -> Task<Message> {
        self.exit_request = Some(ExitRequest::LastTab);
        self.start_workspace_save()
    }

    pub(super) fn finish_save(&mut self, id: u64, revision: u64, path: PathBuf) -> bool {
        let unchanged = if id == self.tab_id() {
            self.file = Some(path);
            if self.revision == revision {
                self.modified = false;
            }
            self.revision == revision
        } else if let Some(state) = self
            .tabs
            .iter_mut()
            .find(|t| t.id == id)
            .and_then(|t| t.state.as_mut())
        {
            state.file = Some(path);
            if state.revision == revision {
                state.modified = false;
            }
            state.revision == revision
        } else {
            false
        };
        self.dirty_workspace();
        if self.pending_close == Some(id) {
            self.pending_close = None;
            if unchanged {
                return self.discard_tab(id);
            }
        }
        false
    }

    pub(super) fn propagate_theme(&mut self) {
        for state in self.tabs.iter_mut().filter_map(|t| t.state.as_mut()) {
            state.prefs.theme_mode = self.prefs.theme_mode;
            state.prefs.dark = self.prefs.dark;
            let document = Arc::make_mut(&mut state.saved_document);
            document.preferences.theme_mode = self.prefs.theme_mode;
            document.preferences.dark = self.prefs.dark;
        }
    }

    fn workspace(&self) -> Workspace {
        let tabs = self
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                if index == self.active_tab {
                    SavedTab {
                        document: Arc::new(self.document()),
                        file: self.file.clone(),
                        modified: self.modified,
                    }
                } else {
                    tab.state.as_ref().expect("inactive tab has state").saved()
                }
            })
            .collect();
        Workspace {
            version: 1,
            active: self.active_tab,
            tabs,
        }
    }

    pub(super) fn dirty_workspace(&mut self) {
        self.workspace_revision += 1;
        self.dirty = true;
        self.autosaved = false;
    }

    pub(super) fn start_workspace_save(&mut self) -> Task<Message> {
        if self.autosave_pending || (!self.dirty && self.exit_request.is_none()) {
            return Task::none();
        }
        let closing = self.exit_request.is_some();
        let workspace = if matches!(self.exit_request, Some(ExitRequest::LastTab)) {
            // An in-flight ordinary save finishes before this blank session is
            // queued, so discarded text cannot reappear after closing.
            Workspace {
                version: 1,
                active: 0,
                tabs: vec![SavedTab {
                    document: Arc::new(Document::new(
                        String::new(),
                        self.prefs.clone(),
                        "0".into(),
                    )),
                    file: None,
                    modified: false,
                }],
            }
        } else {
            self.workspace()
        };
        let revision = self.workspace_revision;
        let path = storage::session_path().with_file_name("workspace.json");
        #[cfg(target_os = "windows")]
        let update_directory = match &self.exit_request {
            Some(ExitRequest::Restart(directory)) => Some(directory.clone()),
            _ => None,
        };
        self.autosave_pending = true;
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let bytes = serde_json::to_vec(&workspace).map_err(|e| e.to_string())?;
                    let result = storage::atomic_write(&path, &bytes);
                    #[cfg(target_os = "windows")]
                    if let Some(directory) = update_directory {
                        if result.is_ok() {
                            // Only a durably saved session authorizes replacement.
                            storage::atomic_write(&directory.join("commit"), b"ready")?;
                        } else {
                            let _ = std::fs::write(directory.join("cancel"), b"save failed");
                        }
                    }
                    result
                })
                .await
                .map_err(|e| format!("Session save worker failed: {e}"))?
            },
            move |result| Message::WorkspaceSaved(revision, closing, result),
        )
    }

    pub(super) fn workspace_saved(
        &mut self,
        revision: u64,
        closing: bool,
        result: Result<(), String>,
    ) -> Task<Message> {
        self.autosave_pending = false;
        if let Err(error) = result {
            self.dirty = true;
            #[cfg(target_os = "windows")]
            if matches!(self.exit_request, Some(ExitRequest::Restart(_))) {
                self.update_state = updates::State::Failed("The update was canceled because your session could not be saved. The installed binary is unchanged.".into());
            }
            self.exit_request = None;
            self.notify(format!("Autosave failed: {error}"));
            return Task::none();
        }
        if revision == self.workspace_revision {
            self.dirty = false;
            self.autosaved = true;
        }
        if closing && let Some(exit) = self.exit_request.take() {
            return match exit {
                ExitRequest::Window(id) => window::close(id),
                ExitRequest::LastTab => window::latest().and_then(window::close),
                #[cfg(target_os = "windows")]
                ExitRequest::Restart(_) => window::latest().and_then(window::close),
            };
        }
        // Flush a final snapshot after an older write completes. Ordinary
        // edits are coalesced until the next tick rather than flooding disk.
        if self.exit_request.is_some() {
            return self.start_workspace_save();
        }
        Task::none()
    }

    pub(super) fn restore_workspace(&mut self) -> Result<(), String> {
        let path = storage::session_path().with_file_name("workspace.json");
        if !path.exists() {
            return Ok(());
        }
        let workspace: Workspace =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        self.restore_tabs(workspace)
    }

    fn restore_tabs(&mut self, mut workspace: Workspace) -> Result<(), String> {
        if workspace.version != 1
            || workspace.tabs.is_empty()
            || workspace.active >= workspace.tabs.len()
        {
            return Err("Invalid workspace session".into());
        }
        for tab in &mut workspace.tabs {
            if !matches!(tab.document.version, 1 | 2) || tab.document.text.len() > 1_000_000 {
                return Err("Invalid document in workspace session".into());
            }
            let document = Arc::make_mut(&mut tab.document);
            document.preferences.zoom = document.preferences.zoom.clamp(0.6, 2.0);
            document.preferences.format.digits = document.preferences.format.digits.min(12);
        }
        self.tabs = workspace
            .tabs
            .into_iter()
            .enumerate()
            .map(|(index, tab)| Tab {
                id: index as u64,
                state: Some(TapeState::new(
                    Arc::unwrap_or_clone(tab.document),
                    tab.file,
                    tab.modified,
                )),
            })
            .collect();
        self.next_tab_id = self.tabs.len() as u64;
        self.active_tab = workspace.active;
        let state = self.tabs[self.active_tab]
            .state
            .take()
            .expect("restored tab has state");
        self.install_tape(state);
        Ok(())
    }

    pub(super) fn tab_bar(&self) -> Element<'_, Message> {
        let p = self.colors();
        let mut bar = row![].spacing(6);
        for (index, tab) in self.tabs.iter().enumerate() {
            let (file, modified) = if index == self.active_tab {
                (&self.file, self.modified)
            } else {
                let state = tab.state.as_ref().expect("inactive tab has state");
                (&state.file, state.modified)
            };
            let name = file
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("Untitled {}", tab.id + 1));
            let label: String = name.chars().take(24).collect();
            let label = format!(
                "{}{}{}",
                label,
                if name.chars().count() > 24 { "…" } else { "" },
                if modified { " •" } else { "" }
            );
            bar = bar.push(
                container(
                    row![
                        button(text(label).size(13))
                            .padding([9, 10])
                            .on_press(Message::SwitchTab(tab.id))
                            .style(button::text),
                        button(
                            container(standard_icon("close", p.muted).width(13).height(13)).id(
                                if index == self.active_tab {
                                    "close-active-tab"
                                } else {
                                    "close-inactive-tab"
                                }
                            )
                        )
                        .padding(8)
                        .on_press(Message::CloseTab(tab.id))
                        .style(button::text),
                    ]
                    .align_y(alignment::Vertical::Center),
                )
                .id(if index == self.active_tab {
                    "active-tab"
                } else {
                    "inactive-tab"
                })
                .style(move |_| container::Style {
                    background: Some(
                        if index == self.active_tab {
                            p.paper
                        } else {
                            p.background
                        }
                        .into(),
                    ),
                    border: Border {
                        radius: iced::border::Radius {
                            top_left: 10.0,
                            top_right: 10.0,
                            bottom_left: 0.0,
                            bottom_right: 0.0,
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            );
        }
        row![
            scrollable(bar)
                .direction(widget::scrollable::Direction::Horizontal(
                    widget::scrollable::Scrollbar::default()
                ))
                .width(Fill),
            self.hint(
                self.small("+", Message::NewTab),
                "New tab · Ctrl+T",
                widget::tooltip::Position::Bottom
            )
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center)
        .into()
    }

    pub(super) fn close_tab_view(&self, id: u64) -> Element<'_, Message> {
        let p = self.colors();
        self.card(
            container(
                column![
                    text("Save changes before closing?").size(21),
                    text("This tape has changes that are not saved to a document.").size(14),
                    row![
                        self.small("Cancel", Message::Close),
                        self.small("Discard", Message::DiscardTab(id)),
                        self.small("Save & close", Message::SaveCloseTab(id))
                    ]
                    .spacing(12),
                ]
                .spacing(20),
            )
            .padding(24),
            p.card,
        )
        .width(510)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialogs::tests::app;

    #[test]
    fn inactive_snapshots_are_shared_and_keep_in_flight_preferences() {
        let mut app = app();
        let _ = app.update(Message::NewTab);
        let before = app.workspace();
        let next = app.workspace();
        assert!(Arc::ptr_eq(
            &before.tabs[0].document,
            &next.tabs[0].document
        ));
        app.prefs.theme_mode = storage::ThemeMode::Dark;
        app.propagate_theme();
        let after = app.workspace();
        assert_eq!(
            before.tabs[0].document.preferences.theme_mode,
            storage::ThemeMode::System
        );
        assert_eq!(
            after.tabs[0].document.preferences.theme_mode,
            storage::ThemeMode::Dark
        );
    }
    #[test]
    fn old_workspace_completion_does_not_clear_newer_changes() {
        let mut app = app();
        app.dirty_workspace();
        let old_revision = app.workspace_revision;
        app.autosave_pending = true;
        app.dirty_workspace();
        let _ = app.workspace_saved(old_revision, false, Ok(()));
        assert!(app.dirty);
        assert!(!app.autosaved);
        let _ = app.workspace_saved(app.workspace_revision, false, Ok(()));
        assert!(!app.dirty);
        assert!(app.autosaved);
    }
    #[test]
    fn close_waits_for_old_save_then_queues_final_snapshot() {
        let mut app = app();
        app.autosave_pending = true;
        let _ = app.exit_last_tab();
        assert!(matches!(app.exit_request, Some(ExitRequest::LastTab)));
        let _ = app.workspace_saved(app.workspace_revision, false, Ok(()));
        assert!(app.autosave_pending);
        assert!(app.exit_request.is_some());
        let _ = app.workspace_saved(app.workspace_revision, true, Err("disk full".into()));
        assert!(!app.autosave_pending);
        assert!(app.exit_request.is_none());
        assert!(app.dirty);
        assert_eq!(app.editor.text, "1.5");
    }
    #[test]
    fn switching_preserves_independent_editing_and_memory() {
        let mut app = app();
        let _ = app.update(Message::Key('2'));
        let original = app.editor.text.clone();
        app.memory = Number::from(42);
        let first = app.tab_id();
        let _ = app.update(Message::NewTab);
        assert!(app.editor.text.is_empty());
        assert_eq!(app.memory, Number::default());
        let _ = app.update(Message::Key('7'));
        let second = app.tab_id();
        app.switch_tab(first);
        assert_eq!(app.editor.text, original);
        assert_eq!(app.memory, Number::from(42));
        let _ = app.update(Message::Undo);
        assert_eq!(app.editor.text, "1.5");
        app.switch_tab(second);
        assert_eq!(app.editor.text, "7");
    }

    #[test]
    fn saves_finish_on_their_original_tab_and_keep_later_edits_dirty() {
        let mut app = app();
        let first = app.tab_id();
        let _ = app.update(Message::Key('2'));
        let revision = app.revision;
        let _ = app.update(Message::NewTab);
        app.finish_save(first, revision, PathBuf::from("first.numpad"));
        assert!(app.file.is_none());
        app.switch_tab(first);
        assert_eq!(app.file, Some(PathBuf::from("first.numpad")));
        assert!(!app.modified);
        let _ = app.update(Message::Key('3'));
        app.finish_save(first, revision, PathBuf::from("first.numpad"));
        assert!(app.modified);
    }

    #[test]
    fn closing_dirty_last_tab_requests_exit_without_replacement() {
        let mut app = app();
        let _ = app.update(Message::Key('2'));
        let first = app.tab_id();
        assert!(!app.request_close_tab(first));
        assert!(matches!(app.modal, Some(Modal::CloseTab(_))));
        let _ = app.update(Message::Close);
        assert_eq!(app.tabs.len(), 1);
        let next_id = app.next_tab_id;
        assert!(app.discard_tab(first));
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tab_id(), first);
        assert_eq!(app.next_tab_id, next_id);
        assert!(app.modified);
    }

    #[test]
    fn last_tab_save_closes_only_the_saved_revision() {
        let mut app = app();
        let id = app.tab_id();
        app.pending_close = Some(id);
        assert!(!app.finish_save(id, app.revision + 1, PathBuf::from("tape.numpad")));
        app.pending_close = Some(id);
        assert!(app.finish_save(id, app.revision, PathBuf::from("tape.numpad")));
        assert!(!app.modified);
        assert!(app.request_close_tab(id));
        assert!(!app.discard_tab(u64::MAX));
    }

    #[test]
    fn workspace_roundtrip_restores_tabs_files_and_active_tape() {
        let mut app = app();
        app.file = Some(PathBuf::from("first.numpad"));
        let _ = app.update(Message::NewTab);
        let _ = app.update(Message::Key('8'));
        let serialized = serde_json::to_vec(&app.workspace()).unwrap();
        let mut restored = crate::dialogs::tests::app();
        restored
            .restore_tabs(serde_json::from_slice(&serialized).unwrap())
            .unwrap();
        assert_eq!(restored.tabs.len(), 2);
        assert_eq!(restored.editor.text, "8");
        assert!(restored.modified);
        restored.switch_tab(0);
        assert_eq!(restored.editor.text, "1.5");
        assert_eq!(restored.file, Some(PathBuf::from("first.numpad")));
    }

    #[test]
    fn explicit_theme_overrides_system_palette() {
        let mut app = app();
        let dark = Colors::new(true);
        app.omarchy_colors = Some(dark);
        assert_eq!(app.colors(), dark);
        app.prefs.theme_mode = storage::ThemeMode::Light;
        assert_eq!(app.colors(), Colors::new(false));
        app.prefs.theme_mode = storage::ThemeMode::System;
        app.omarchy_colors = None;
        app.system_mode = iced::theme::Mode::Dark;
        assert_eq!(app.colors(), dark);
    }
}
