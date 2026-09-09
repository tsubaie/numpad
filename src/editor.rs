use crate::engine::{self, Format, Kind, Tape};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pos {
    pub line: usize,
    pub column: usize,
}
#[derive(Clone)]
struct Snapshot {
    text: String,
    caret: Pos,
    anchor: Option<Pos>,
    format: Format,
}
pub struct Editor {
    pub text: String,
    pub caret: Pos,
    pub anchor: Option<Pos>,
    pub tape: Tape,
    format: Format,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    pub notice: String,
}
impl Editor {
    pub fn new(text: String, f: &Format) -> Self {
        let tape = engine::calculate(&text, f);
        Self {
            text,
            caret: Pos::default(),
            anchor: None,
            tape,
            format: f.clone(),
            undo: vec![],
            redo: vec![],
            notice: String::new(),
        }
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            caret: self.caret,
            anchor: self.anchor,
            format: self.format.clone(),
        }
    }
    fn record(&mut self) {
        self.undo.push(self.snapshot());
        if self.undo.len() > 300 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.notice.clear();
    }
    fn restore(&mut self, s: Snapshot, f: &Format) {
        self.text = if s.format == *f {
            s.text
        } else {
            transcode(&s.text, &s.format, f)
        };
        self.caret = s.caret;
        self.anchor = s.anchor;
        self.tape = engine::calculate(&self.text, f);
        self.format = f.clone();
    }
    pub fn undo(&mut self, f: &Format) {
        if let Some(s) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(s, f);
        }
    }
    pub fn redo(&mut self, f: &Format) {
        if let Some(s) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.restore(s, f);
        }
    }
    pub fn clear(&mut self, f: &Format) {
        self.record();
        self.text.clear();
        self.caret = Pos::default();
        self.anchor = None;
        self.recalculate(f, false);
    }
    pub fn load(&mut self, text: String, f: &Format) {
        self.record();
        self.text = text;
        self.caret = Pos::default();
        self.anchor = None;
        self.recalculate(f, true);
    }
    fn index(&self, p: Pos) -> usize {
        let mut start = 0;
        for (i, l) in self.text.split('\n').enumerate() {
            if i == p.line {
                let mut col = p.column.min(l.len());
                while !l.is_char_boundary(col) {
                    col -= 1;
                }
                return start + col;
            }
            start += l.len() + 1;
        }
        self.text.len()
    }
    fn position(&self, index: usize) -> Pos {
        let prefix = &self.text[..index.min(self.text.len())];
        Pos {
            line: prefix.bytes().filter(|b| *b == b'\n').count(),
            column: prefix.rsplit('\n').next().unwrap_or("").len(),
        }
    }
    fn range(&self) -> (usize, usize) {
        let p = self.index(self.caret);
        let a = self.anchor.map(|a| self.index(a)).unwrap_or(p);
        (p.min(a), p.max(a))
    }
    pub fn selection(&self) -> Option<String> {
        let (s, e) = self.range();
        if s < e {
            Some(self.text[s..e].into())
        } else {
            None
        }
    }
    fn replace(&mut self, value: &str) -> bool {
        let (s, e) = self.range();
        let mut next = String::with_capacity(self.text.len() + value.len());
        next.push_str(&self.text[..s]);
        next.push_str(value);
        next.push_str(&self.text[e..]);
        let old = self.text.split('\n').count();
        let new = next.split('\n').count();
        if new > 500 && new > old {
            self.notice = "500-line limit reached. Delete a line before adding more.".into();
            return false;
        }
        if next.len() > 1_000_000 {
            self.notice = "Document exceeds the 1 MB text limit.".into();
            return false;
        }
        self.text = next;
        self.caret = self.position(s + value.len());
        self.anchor = None;
        true
    }
    fn protected(&self) -> bool {
        if self.selection().is_some() {
            return false;
        }
        self.tape.lines.get(self.caret.line).is_some_and(|l| {
            if l.kind == Kind::Rule {
                return true;
            }
            l.kind == Kind::Total && self.caret.column < l.raw.find('=').unwrap_or(l.raw.len())
        })
    }
    pub fn key(&mut self, c: char, f: &Format) {
        if c == '='
            && self.selection().is_none()
            && self
                .tape
                .lines
                .get(self.caret.line)
                .is_some_and(|l| l.kind == Kind::Value)
        {
            self.enter(f);
            return;
        }
        if matches!(c, '+' | '-' | '*' | '/' | '^' | '×' | '÷') && self.selection().is_none() {
            let op = match c {
                '×' => '*',
                '÷' => '/',
                _ => c,
            };
            if let Some(line) = self.tape.lines.get(self.caret.line).cloned() {
                if line.kind == Kind::Value && line.value.is_some() {
                    self.record();
                    let mut rows: Vec<String> = self.text.split('\n').map(str::to_owned).collect();
                    let index = self.caret.line;
                    rows[index] = engine::render(&line, f);
                    let insert = format!("\n {op} ");
                    self.text = rows.join("\n");
                    self.caret.column = rows[index].len();
                    self.replace(&insert);
                    self.recalculate(f, false);
                    return;
                }
                if line.kind == Kind::Value && line.operand.is_empty() && line.raw.trim().len() <= 1
                {
                    self.record();
                    let mut rows: Vec<String> = self.text.split('\n').map(str::to_owned).collect();
                    // A minus after * or / is the sign of the next operand.
                    if op == '-' && matches!(line.op, '*' | '/' | '^') {
                        rows[self.caret.line].push('-');
                    } else {
                        rows[self.caret.line] = format!(" {op} ");
                    }
                    self.caret.column = rows[self.caret.line].len();
                    self.text = rows.join("\n");
                    self.recalculate(f, false);
                    return;
                }
                if line.kind == Kind::Total {
                    self.record();
                    self.caret.column = line.raw.len();
                    self.replace(&format!("\n {op} "));
                    self.recalculate(f, false);
                    return;
                }
            }
        }
        if self.protected() {
            self.notice =
                "This value is calculated. Edit an operand above, or append a comment / = name."
                    .into();
            return;
        }
        self.record();
        self.replace(&c.to_string());
        self.recalculate(f, false);
    }
    pub fn paste(&mut self, s: &str, f: &Format) {
        if self.protected() {
            self.notice = "Edit the source operands to change a computed total.".into();
            return;
        }
        self.record();
        self.replace(&s.replace("\r\n", "\n").replace('\r', "\n"));
        self.recalculate(f, false);
    }
    pub fn enter(&mut self, f: &Format) {
        self.record();
        if self.selection().is_some() {
            self.replace("\n");
            self.recalculate(f, false);
            return;
        }
        let line = self.tape.lines.get(self.caret.line).cloned();
        if let Some(l) = line {
            if l.kind == Kind::Value && l.count >= 2 {
                let mut rows: Vec<String> = self.text.split('\n').map(str::to_owned).collect();
                let index = self.caret.line;
                rows[index] = engine::render(&l, f);
                self.text = rows.join("\n");
                self.caret.column = rows[index].len();
                self.replace("\n -----------------\n +           0.00\n");
                self.recalculate(f, true);
                return;
            }
            if l.kind == Kind::Total || l.kind == Kind::Rule {
                self.caret.column = l.raw.len();
            }
        }
        self.replace("\n");
        self.recalculate(f, false);
    }
    pub fn delete(&mut self, backward: bool, f: &Format) {
        if self.protected() {
            self.notice =
                "Select the full subtotal to remove it, or edit its source operands.".into();
            return;
        }
        self.record();
        if self.selection().is_some() {
            self.replace("");
        } else {
            let i = self.index(self.caret);
            if backward && i > 0 {
                let prev = self.text[..i].char_indices().last().unwrap().0;
                self.anchor = Some(self.position(prev));
                self.replace("");
            } else if !backward && i < self.text.len() {
                let end = i + self.text[i..].chars().next().unwrap().len_utf8();
                self.anchor = Some(self.position(end));
                self.replace("");
            }
        }
        self.recalculate(f, false);
    }
    pub fn recalculate(&mut self, f: &Format, format_all: bool) {
        self.format = f.clone();
        self.tape = engine::calculate(&self.text, f);
        let mut rows: Vec<String> = self.text.split('\n').map(str::to_owned).collect();
        let mut changed = false;
        for (i, l) in self.tape.lines.iter().enumerate() {
            if l.kind == Kind::Total || (format_all && l.kind == Kind::Value) {
                let next = engine::render(l, f);
                if rows[i] != next {
                    let delta = next.len() as isize - rows[i].len() as isize;
                    if self.caret.line == i {
                        self.caret.column = self
                            .caret
                            .column
                            .saturating_add_signed(delta)
                            .min(next.len());
                    }
                    if let Some(a) = &mut self.anchor
                        && a.line == i
                    {
                        a.column = a.column.saturating_add_signed(delta).min(next.len());
                    }
                    rows[i] = next;
                    changed = true;
                }
            }
        }
        if changed {
            self.text = rows.join("\n");
            self.tape = engine::calculate(&self.text, f);
        }
    }
    pub fn format_changed(&mut self, old: &Format, new: &Format) {
        self.text = transcode(&self.text, old, new);
        self.anchor = None;
        self.recalculate(new, true);
    }
    pub fn custom(&mut self, formula: &str, close: bool, f: &Format) {
        let formula = formula.replace("\\n", "\n");
        // Each character follows calculator input rules, just like the keypad.
        let before = self.snapshot();
        let history = self.undo.len();
        for c in formula.chars() {
            if c == '\n' {
                self.enter(f);
            } else {
                self.key(c, f);
            }
        }
        if close {
            self.enter(f);
        }
        self.undo.truncate(history);
        self.undo.push(before);
    }
}

fn transcode(text: &str, old: &Format, new: &Format) -> String {
    engine::calculate(text, old)
        .lines
        .iter()
        .map(|line| {
            if line.kind == Kind::Assignment {
                let rhs = if old.comma != new.comma {
                    line.operand
                        .chars()
                        .map(|c| match c {
                            '.' => ',',
                            ',' => '.',
                            _ => c,
                        })
                        .collect()
                } else {
                    line.operand.clone()
                };
                format!("{} = {}", line.name, rhs)
            } else {
                engine::render(line, new)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn type_text(e: &mut Editor, s: &str) {
        for c in s.chars() {
            e.key(c, &Format::default());
        }
    }
    #[test]
    fn typing_splits_operators() {
        let f = Format::default();
        let mut e = Editor::new(String::new(), &f);
        type_text(&mut e, "10+2*3");
        e.enter(&f);
        assert_eq!(e.tape.grand.normalized().to_string(), "36");
        assert_eq!(e.text.lines().count(), 5);
        assert_eq!(e.caret.line, 5);
    }
    #[test]
    fn paste_is_not_keystrokes() {
        let f = Format::default();
        let mut e = Editor::new(String::new(), &f);
        e.paste("+10+2*3\n+1", &f);
        e.enter(&f);
        assert_eq!(e.tape.grand.normalized().to_string(), "11");
    }
    #[test]
    fn single_value_no_total() {
        let f = Format::default();
        let mut e = Editor::new(String::new(), &f);
        type_text(&mut e, "100");
        e.enter(&f);
        assert!(!e.text.contains("---"));
    }
    #[test]
    fn formatting_preserves_literal_operand_precision() {
        let f = Format::default();
        let mut e = Editor::new(String::new(), &f);
        type_text(&mut e, "1.2345*10000");
        e.enter(&f);
        assert_eq!(e.tape.grand.normalized().to_string(), "12345");
        assert!(e.text.contains("1.2345"));
        let comma = Format {
            comma: true,
            ..f.clone()
        };
        e.format_changed(&f, &comma);
        assert_eq!(e.tape.grand.normalized().to_string(), "12345");
        assert!(e.text.contains("1,2345"));
    }
    #[test]
    fn continuation_and_blank() {
        let f = Format::default();
        let mut e = Editor::new(String::new(), &f);
        type_text(&mut e, "10+2");
        e.enter(&f);
        type_text(&mut e, "*3");
        e.enter(&f);
        assert_eq!(e.tape.grand.normalized().to_string(), "36");
        e.enter(&f);
        type_text(&mut e, "100-20");
        e.enter(&f);
        assert_eq!(e.tape.grand.normalized().to_string(), "116");
    }
    #[test]
    fn clear_undo_redo() {
        let f = Format::default();
        let mut e = Editor::new("10\n+2".into(), &f);
        e.clear(&f);
        assert!(e.text.is_empty());
        e.undo(&f);
        assert_eq!(e.text, "10\n+2");
        e.redo(&f);
        assert!(e.text.is_empty());
    }
    #[test]
    fn unicode_backspace_safe() {
        let f = Format::default();
        let mut e = Editor::new(String::new(), &f);
        type_text(&mut e, "مرحبا");
        e.delete(true, &f);
        assert_eq!(e.text, "مرحب");
    }
    #[test]
    fn editing_recalculates_total() {
        let f = Format::default();
        let mut e = Editor::new("100\n+19%\n---\n+119".into(), &f);
        e.caret = Pos { line: 0, column: 3 };
        e.anchor = Some(Pos::default());
        e.paste("200", &f);
        assert!(e.text.contains("238.00"));
    }
    #[test]
    fn limits_reject_newlines_without_truncation() {
        let f = Format::default();
        let text = vec!["note"; 500].join("\n");
        let mut e = Editor::new(text.clone(), &f);
        e.enter(&f);
        assert_eq!(e.text, text);
        assert!(e.notice.contains("500"));
    }
    #[test]
    fn undo_after_decimal_locale_change_preserves_value() {
        let original = Format::default();
        let comma = Format {
            comma: true,
            ..original.clone()
        };
        let mut e = Editor::new(String::new(), &original);
        type_text(&mut e, "1000+20");
        e.enter(&original);
        e.format_changed(&original, &comma);
        e.undo(&comma);
        assert_eq!(
            e.tape.lines.last().unwrap().result.normalized().to_string(),
            "1020"
        );
    }
    #[test]
    fn typing_a_sum_name_preserves_pending_equals() {
        let f = Format::default();
        let mut e = Editor::new("100\n+20\n---\n+120\n".into(), &f);
        e.recalculate(&f, true);
        e.caret = Pos {
            line: 3,
            column: e.text.lines().nth(3).unwrap().len(),
        };
        type_text(&mut e, "=budget");
        assert!(e.text.contains("= budget"));
        assert!(e.tape.variables.contains_key("budget"));
        assert_eq!(e.tape.variables["budget"].1.normalized().to_string(), "120");
    }
}
