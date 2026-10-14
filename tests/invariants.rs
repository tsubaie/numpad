use numpad::{
    editor::Editor,
    engine::{self, Format},
    limits, math,
    storage::{self, Document, Preferences},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Temp(PathBuf);

#[test]
fn subtotal_spaces_comments_and_names_remain_editable() {
    let f = Format::default();
    let mut e = Editor::new(String::new(), &f);
    for c in "1+2".chars() {
        e.key(c, &f);
    }
    e.enter(&f);
    e.caret.line = e
        .tape
        .lines
        .iter()
        .position(|l| l.kind == engine::Kind::Total)
        .unwrap();
    e.caret.column = e.tape.lines[e.caret.line].raw.len();
    let line = e.caret.line;
    let end = e.caret.column;
    for i in 1..=3 {
        e.key(' ', &f);
        assert_eq!(e.caret.column, end + i);
        assert_eq!(e.text.lines().nth(line).unwrap().len(), end + i);
    }
    for c in "hello world".chars() {
        e.key(c, &f);
    }
    e.caret.column = end + 3 + 5;
    e.key('!', &f);
    assert!(e.text.contains("   hello! world"));
    e.undo(&f);
    assert!(e.text.contains("   hello world"));
    e.redo(&f);
    assert!(e.text.contains("   hello! world"));
    let comma = Format {
        comma: true,
        ..f.clone()
    };
    assert!(e.format_changed(&f, &comma));
    assert_eq!(e.tape.grand, math::Number::from(3));
    assert!(e.text.contains("   hello! world"));
}
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "numpad-invariants-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn tiny_operands_survive_typing_locale_changes_and_text_export() {
    let f = Format::default();
    let mut e = Editor::new(String::new(), &f);
    for c in "0.0000000000001*10000000000000".chars() {
        e.key(c, &f);
    }
    e.enter(&f);
    assert_eq!(e.tape.grand, math::dec("1"));
    let comma = Format {
        comma: true,
        ..f.clone()
    };
    e.format_changed(&f, &comma);
    assert_eq!(e.tape.grand, math::dec("1"));
    e.format_changed(&comma, &f);
    let exported = storage::text_export(&e.text, &f);
    assert_eq!(engine::calculate(&exported, &f).grand, math::dec("1"));
}

#[test]
fn operand_formatting_roundtrips_generated_decimal_values() {
    for comma in [false, true] {
        let f = Format { comma, digits: 2 };
        for coefficient in [-1234567, -1, 0, 1, 998877] {
            for scale in [0, 1, 2, 12, 13, 32, 60, 128, 512] {
                let number =
                    engine::number(&format!("{coefficient}e-{scale}"), &Format::default()).unwrap();
                let formatted = engine::format_operand(&number, &f);
                assert_eq!(
                    engine::number(&formatted, &f).unwrap(),
                    number,
                    "{formatted}"
                );
            }
        }
    }
}

#[test]
fn hostile_exponents_and_powers_return_errors() {
    let f = Format::default();
    for value in [
        "1e-9223372036854775807",
        "1e9223372036854775807",
        "1e-1000000",
        "1e999999999999999999999999",
        "10e-9223372036854775808",
    ] {
        assert!(engine::number(value, &f).is_err(), "{value}");
    }
    assert!(math::power(&math::dec("0.1"), &math::dec("10000")).is_err());
    assert!(math::power(&math::dec("10"), &math::dec("10000")).is_err());
    assert_eq!(
        math::power(&math::dec("2"), &math::dec("-3")).unwrap(),
        math::dec("0.125")
    );
}

#[test]
fn bounded_negative_exponentials_remain_valid() {
    for value in [-80, -100, -500] {
        let result = math::function("exp", &math::Number::from(value)).unwrap();
        assert!(result > 0);
        assert!(result < math::dec("1e-32"));
    }
}

#[test]
fn formatting_and_history_respect_remaining_workspace_budget() {
    let f = Format::default();
    let mut editor = Editor::new("12345".into(), &f);
    editor.clear(&f);
    editor.set_text_limit(4);
    editor.undo(&f);
    assert!(editor.text.is_empty());
    assert!(editor.can_undo());
    editor.set_text_limit(100);
    editor.undo(&f);
    assert_eq!(editor.text, "12345");
    editor.set_text_limit(5);
    let comma = Format {
        comma: true,
        ..f.clone()
    };
    assert!(!editor.format_changed(&f, &comma));
    assert_eq!(editor.text, "12345");
    editor.recalculate(&f, true);
    assert_eq!(editor.text, "12345");
}

#[test]
fn import_and_restore_validation_share_limits() {
    let temp = Temp::new();
    let path = temp.0.join("document.numpad");
    for doc in [
        Document::new(
            vec!["1"; limits::LINES + 1].join("\n"),
            Preferences::default(),
            "0".into(),
        ),
        Document::new(
            "1".into(),
            Preferences::default(),
            "1e-9223372036854775807".into(),
        ),
        Document::new(
            "x".repeat(limits::TEXT_BYTES + 1),
            Preferences::default(),
            "0".into(),
        ),
    ] {
        fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
        assert!(storage::load(&path).is_err());
    }
    let path = temp.0.join("document.txt");
    fs::write(&path, "x".repeat(limits::TEXT_BYTES + 1)).unwrap();
    assert!(storage::load(&path).is_err());
    fs::write(&path, vec!["1"; limits::LINES + 1].join("\n")).unwrap();
    assert!(storage::load(&path).is_err());
    fs::write(&path, "1\r\n+2\r\n").unwrap();
    assert_eq!(storage::load(&path).unwrap().text, "1\n+2\n");
}

#[test]
fn maximum_document_roundtrips_even_when_json_expands_it() {
    let temp = Temp::new();
    let path = temp.0.join("escaped.numpad");
    let text = "\t".repeat(limits::TEXT_BYTES);
    storage::save(
        &path,
        &Document::new(text.clone(), Preferences::default(), "0".into()),
    )
    .unwrap();
    assert_eq!(storage::load(&path).unwrap().text, text);
}

#[test]
fn rejected_recovery_bytes_survive_later_atomic_save() {
    let temp = Temp::new();
    let path = temp.0.join("workspace.json");
    fs::write(&path, b"broken session").unwrap();
    let backup = storage::preserve_recovery(&path).unwrap();
    storage::atomic_write(&path, b"new valid session").unwrap();
    assert_eq!(fs::read(backup).unwrap(), b"broken session");
    assert_eq!(fs::read(path).unwrap(), b"new valid session");
}

#[test]
fn scientific_cache_depends_on_value_not_variable_name() {
    for input in ["x=0\ny=sin(x)", "x=1\ny=sin(x)", "x=0\ny=sin(x)"] {
        let tape = engine::calculate(input, &Format::default());
        let x = &tape.variables["x"].1;
        let y = &tape.variables["y"].1;
        assert_eq!(*y, math::function("sin", x).unwrap());
        assert_eq!(y == &math::dec("0"), x == &math::dec("0"));
    }
}

#[test]
fn generated_utf8_edit_sequences_preserve_buffer_and_undo_invariants() {
    let f = Format::default();
    let mut e = Editor::new(String::new(), &f);
    let mut seed = 0x12345678_u64;
    let inputs = [
        "é",
        "مرحبا",
        "🙂",
        "\n",
        "1.23",
        "+",
        "note",
        "\r\n",
        "漢字",
    ];
    for _ in 0..2000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        match (seed >> 32) % 6 {
            0..=2 => e.paste(inputs[(seed as usize) % inputs.len()], &f),
            3 => e.delete(true, &f),
            4 => e.undo(&f),
            _ => e.redo(&f),
        }
        limits::text(&e.text).unwrap();
        let fresh = engine::calculate(&e.text, &f);
        assert_eq!(fresh.grand, e.tape.grand);
        assert_eq!(fresh.variables, e.tape.variables);
    }
}

#[test]
fn long_unary_sequences_do_not_recurse_per_sign() {
    let vars = Default::default();
    let expression = format!("{}1", "-".repeat(2000));
    assert_eq!(
        engine::expression(&expression, &vars, &Format::default()).unwrap(),
        math::dec("1")
    );
}

#[cfg(unix)]
#[test]
fn saves_keep_private_permissions_and_cleanup_failed_replacements() {
    use std::os::unix::fs::PermissionsExt;
    let temp = Temp::new();
    let path = temp.0.join("private.numpad");
    storage::atomic_write(&path, b"first").unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    storage::atomic_write(&path, b"second").unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    let directory = temp.0.join("directory");
    fs::create_dir(&directory).unwrap();
    assert!(storage::atomic_write(&directory, b"cannot replace a directory").is_err());
    assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 2);
}
