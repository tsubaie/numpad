//! Keeps the widget in sync without replacing its whole buffer on each edit.
use iced::widget::text_editor::{self, Action, Edit};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Default)]
pub struct TapeContent {
    content: text_editor::Content,
    text: String,
    selection_mode: SelectionMode,
}
#[derive(Default, Clone, Copy)]
enum SelectionMode {
    #[default]
    Normal,
    Word,
    Line,
}
impl TapeContent {
    pub fn with_text(text: &str) -> Self {
        Self {
            content: text_editor::Content::with_text(text),
            text: text.into(),
            selection_mode: SelectionMode::Normal,
        }
    }
    pub fn perform(&mut self, action: Action) {
        self.selection_mode = match &action {
            Action::SelectWord => SelectionMode::Word,
            Action::SelectLine => SelectionMode::Line,
            Action::Drag(_) | Action::Select(_) => self.selection_mode,
            _ => SelectionMode::Normal,
        };
        self.content.perform(action);
    }
    pub fn cursor(&self) -> text_editor::Cursor {
        let cursor = self.content.cursor();
        let Some(anchor) = cursor.selection else {
            return cursor;
        };
        if matches!(self.selection_mode, SelectionMode::Normal) {
            return cursor;
        }
        // Iced exposes the click/drag anchors, not the expanded word/line
        // bounds. Match cosmic-text's Unicode boundaries for our edit model.
        let forward =
            (anchor.line, anchor.column) <= (cursor.position.line, cursor.position.column);
        let (mut start, mut end) = if forward {
            (anchor, cursor.position)
        } else {
            (cursor.position, anchor)
        };
        let first = self.text.split('\n').nth(start.line).unwrap_or("");
        let last = self.text.split('\n').nth(end.line).unwrap_or("");
        match self.selection_mode {
            SelectionMode::Word => {
                start.column = first
                    .unicode_word_indices()
                    .rev()
                    .map(|(i, _)| i)
                    .find(|&i| i < start.column)
                    .unwrap_or(0);
                end.column = last
                    .unicode_word_indices()
                    .map(|(i, word)| i + word.len())
                    .find(|&i| i > end.column)
                    .unwrap_or(last.len());
            }
            SelectionMode::Line => {
                start.column = 0;
                end.column = last.len();
            }
            SelectionMode::Normal => unreachable!(),
        }
        text_editor::Cursor {
            position: if forward { end } else { start },
            selection: Some(if forward { start } else { end }),
        }
    }
    pub fn move_to(&mut self, cursor: text_editor::Cursor) {
        // Preserve native word/line selection (including drag behavior) when
        // the model is merely echoing the selection back to the widget.
        if cursor == self.cursor() {
            return;
        }
        if cursor.selection.is_none() && self.content.cursor().selection.is_some() {
            // Iced 0.14 move_to(None) does not clear an existing selection.
            self.content
                .perform(Action::Move(text_editor::Motion::Left));
        }
        self.selection_mode = SelectionMode::Normal;
        self.content.move_to(cursor);
    }
    pub fn sync(&mut self, next: &str) {
        if self.text == next {
            return;
        }
        self.selection_mode = SelectionMode::Normal;
        let old_lines: Vec<_> = self.text.split('\n').collect();
        let new_lines: Vec<_> = next.split('\n').collect();
        if old_lines.len() == new_lines.len() {
            let mut offset = 0;
            let mut patches = Vec::new();
            for (line, (old, new)) in old_lines.iter().zip(&new_lines).enumerate() {
                if old != new {
                    let (start, old_end, new_end) = changed_range(old, new);
                    patches.push((line, offset, start, old_end, &new[start..new_end]));
                }
                offset += old.len() + 1;
            }
            for (line, offset, start, end, replacement) in patches.into_iter().rev() {
                self.content.move_to(text_editor::Cursor {
                    position: text_editor::Position { line, column: end },
                    selection: Some(text_editor::Position {
                        line,
                        column: start,
                    }),
                });
                self.content
                    .perform(Action::Edit(Edit::Paste(Arc::new(replacement.into()))));
                self.text
                    .replace_range(offset + start..offset + end, replacement);
            }
            return;
        }
        let (start, old_end, new_end) = changed_range(&self.text, next);
        self.content.move_to(text_editor::Cursor {
            position: position(&self.text, old_end),
            selection: Some(position(&self.text, start)),
        });
        self.content.perform(Action::Edit(Edit::Paste(Arc::new(
            next[start..new_end].into(),
        ))));
        self.text
            .replace_range(start..old_end, &next[start..new_end]);
    }
}
impl Deref for TapeContent {
    type Target = text_editor::Content;
    fn deref(&self) -> &Self::Target {
        &self.content
    }
}
impl DerefMut for TapeContent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}
fn position(text: &str, offset: usize) -> text_editor::Position {
    let prefix = &text[..offset];
    text_editor::Position {
        line: prefix.bytes().filter(|b| *b == b'\n').count(),
        column: prefix.rsplit('\n').next().unwrap_or("").len(),
    }
}
fn changed_range(old: &str, new: &str) -> (usize, usize, usize) {
    let mut start = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(start) || !new.is_char_boundary(start) {
        start -= 1;
    }
    let suffix = old[start..]
        .bytes()
        .rev()
        .zip(new[start..].bytes().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let mut old_end = old.len() - suffix;
    let mut new_end = new.len() - suffix;
    while !old.is_char_boundary(old_end) || !new.is_char_boundary(new_end) {
        old_end += 1;
        new_end += 1;
    }
    (start, old_end, new_end)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn word_selection_survives_sync_and_matches_unicode_clipboard_bounds() {
        for (text, column, expected) in [
            ("one hello world", 6, "hello"),
            ("one مرحبا world", 8, "مرحبا"),
            ("one café world", 6, "café"),
        ] {
            let mut content = TapeContent::with_text(text);
            content.move_to(text_editor::Cursor {
                position: text_editor::Position { line: 0, column },
                selection: None,
            });
            content.perform(Action::SelectWord);
            let cursor = content.cursor();
            content.sync(text);
            content.move_to(cursor);
            assert_eq!(content.selection().as_deref(), Some(expected));
            assert_eq!(
                &text[cursor.selection.unwrap().column..cursor.position.column],
                expected
            );
            // Replacing a native selection must leave a collapsed caret.
            content.sync("replacement");
            content.move_to(text_editor::Cursor {
                position: text_editor::Position {
                    line: 0,
                    column: 11,
                },
                selection: None,
            });
            assert!(content.selection().is_none());
        }
    }
    #[test]
    fn triple_click_selection_and_deselection_survive_model_sync() {
        let mut content = TapeContent::with_text("first\nwhole line\nlast");
        content.move_to(text_editor::Cursor {
            position: text_editor::Position { line: 1, column: 3 },
            selection: None,
        });
        content.perform(Action::SelectLine);
        let cursor = content.cursor();
        assert_eq!(cursor.selection.unwrap().column, 0);
        assert_eq!(cursor.position.column, 10);
        content.move_to(cursor);
        assert_eq!(content.selection().as_deref(), Some("whole line"));
        content.move_to(text_editor::Cursor {
            position: cursor.position,
            selection: None,
        });
        assert!(content.selection().is_none());
    }
    #[test]
    fn patches_unicode_insert_delete_and_multiple_rows() {
        let mut content = TapeContent::with_text("start\nمرحبا\nend");
        for next in [
            "start\nمرحبا!\nend",
            "start\nم\nend",
            "start\n\nnew\nend",
            "",
            "é",
            "ê",
            "🙂",
            "🙃",
        ] {
            content.sync(next);
            assert_eq!(content.content.text(), next);
        }
    }
    #[test]
    fn single_edit_retains_the_unchanged_tail() {
        assert_eq!(
            changed_range("one\ntwo\nthree", "one\ntwice\nthree"),
            (6, 7, 9)
        );
    }
}
