//! Ordered manual saves and exports. Dialogs and writes share the same queue,
//! so an older snapshot can never replace a newer one after it has committed.
use super::*;
use std::collections::VecDeque;

pub(super) struct SaveRequest {
    tab: u64,
    revision: u64,
    kind: FileKind,
    save_as: bool,
    document: Document,
}

#[derive(Default)]
pub(super) struct Saves {
    active: bool,
    queue: VecDeque<SaveRequest>,
}

impl Saves {
    pub fn pending(&self) -> bool {
        self.active || !self.queue.is_empty()
    }
    pub fn queued_for(&self, tab: u64) -> bool {
        self.queue.iter().any(|request| request.tab == tab)
    }
    fn enqueue(&mut self, request: SaveRequest) -> Result<(), String> {
        // Repeated Ctrl+S only needs the newest queued snapshot for that tab.
        if !request.save_as && request.kind == FileKind::Native {
            self.queue
                .retain(|q| !(q.tab == request.tab && q.kind == FileKind::Native && !q.save_as));
        }
        if self.queue.len() >= 16 {
            return Err("Save queue is full. Wait for the pending saves to finish.".into());
        }
        self.queue.push_back(request);
        Ok(())
    }
}

impl App {
    pub(super) fn queue_save(&mut self, kind: FileKind, save_as: bool) -> Task<Message> {
        self.menu = false;
        self.exports = false;
        let request = SaveRequest {
            tab: self.tab_id(),
            revision: self.revision,
            kind,
            save_as,
            document: self.document(),
        };
        if let Err(error) = self.saves.enqueue(request) {
            self.pending_close = None;
            self.notify(error);
            return Task::none();
        }
        self.next_save()
    }

    pub(super) fn next_save(&mut self) -> Task<Message> {
        if self.saves.active {
            return Task::none();
        }
        let Some(request) = self.saves.queue.pop_front() else {
            return self.start_workspace_save();
        };
        self.saves.active = true;
        let existing = if !request.save_as && request.kind == FileKind::Native {
            if request.tab == self.tab_id() {
                self.file.clone()
            } else {
                self.tab_file(request.tab)
            }
        } else {
            None
        };
        let SaveRequest {
            tab,
            revision,
            kind,
            document,
            ..
        } = request;
        Task::perform(
            async move {
                let (extension, label) = match kind {
                    FileKind::Native => ("numpad", "NumPad document"),
                    FileKind::Text => ("txt", "Text tape"),
                    FileKind::Pdf => ("pdf", "PDF document"),
                    FileKind::Excel => ("xlsx", "Excel workbook"),
                };
                let path = if let Some(path) = existing {
                    path
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
                let destination = path.clone();
                tokio::task::spawn_blocking(move || match kind {
                    FileKind::Native => storage::save(&path, &document),
                    FileKind::Text => storage::atomic_write(
                        &path,
                        storage::text_export(&document.text, &document.preferences.format)
                            .as_bytes(),
                    ),
                    FileKind::Pdf => {
                        storage::export_pdf(&path, &document.text, &document.preferences.format)
                    }
                    FileKind::Excel => {
                        storage::export_xlsx(&path, &document.text, &document.preferences.format)
                    }
                })
                .await
                .map_err(|e| format!("Save worker failed: {e}"))??;
                Ok(Some((destination, kind)))
            },
            move |result| Message::Saved(tab, revision, result),
        )
    }

    pub(super) fn saved(
        &mut self,
        tab: u64,
        revision: u64,
        result: Result<Option<(PathBuf, FileKind)>, String>,
    ) -> Task<Message> {
        self.saves.active = false;
        match result {
            Ok(Some((path, kind))) => {
                self.notify(format!(
                    "Saved {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                ));
                if kind == FileKind::Native && self.finish_save(tab, revision, path) {
                    self.exit_request = Some(tabs::ExitRequest::LastTab);
                }
            }
            Err(error) => {
                self.pending_close = None;
                self.exit_request = None;
                self.notify(error);
            }
            Ok(None) => {
                self.pending_close = None;
                self.exit_request = None;
            }
        }
        self.next_save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(tab: u64, revision: u64) -> SaveRequest {
        SaveRequest {
            tab,
            revision,
            kind: FileKind::Native,
            save_as: false,
            document: Document::new(revision.to_string(), Preferences::default(), "0".into()),
        }
    }
    #[test]
    fn repeated_save_keeps_newest_snapshot_in_request_order() {
        let mut saves = Saves::default();
        saves.enqueue(request(1, 1)).unwrap();
        saves.enqueue(request(2, 1)).unwrap();
        saves.enqueue(request(1, 2)).unwrap();
        assert_eq!(saves.queue.len(), 2);
        assert_eq!(saves.queue.pop_front().unwrap().tab, 2);
        let latest = saves.queue.pop_front().unwrap();
        assert_eq!(latest.revision, 2);
        assert_eq!(latest.document.text, "2");
    }
    #[test]
    fn close_waits_for_manual_save_then_flushes_session() {
        let mut app = dialogs::tests::app();
        app.saves.active = true;
        app.exit_request = Some(tabs::ExitRequest::LastTab);
        let _ = app.start_workspace_save();
        assert!(!app.autosave_pending);
        let _ = app.saved(
            app.tab_id(),
            app.revision,
            Ok(Some((PathBuf::from("tape.numpad"), FileKind::Native))),
        );
        assert!(app.autosave_pending);
    }
}
