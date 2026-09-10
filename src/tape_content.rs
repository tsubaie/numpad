//! Keeps the widget in sync without replacing its whole buffer on each edit.
use iced::widget::text_editor::{self, Action, Edit};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Default)]
pub struct TapeContent {
    content: text_editor::Content,
    text: String,
}
impl TapeContent {
    pub fn with_text(text: &str) -> Self {
        Self {
            content: text_editor::Content::with_text(text),
            text: text.into(),
        }
    }
    pub fn sync(&mut self, next: &str) {
        if self.text == next {
            return;
        }
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
