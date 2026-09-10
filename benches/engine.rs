#![allow(dead_code)]
#[path = "../src/editor.rs"]
mod editor;
#[path = "../src/engine.rs"]
mod engine;
#[path = "../src/math.rs"]
mod math;
use std::{hint::black_box, time::Instant};
fn main() {
    let f = engine::Format::default();
    for (name, text, iterations) in [
        ("20 arithmetic lines", vec!["+ 123.45"; 20].join("\n"), 1000),
        (
            "500 arithmetic lines",
            vec!["+ 123.45"; 500].join("\n"),
            500,
        ),
        (
            "500 lines, 950KB notes",
            vec![format!("note {}", "x".repeat(1894)); 500].join("\n"),
            100,
        ),
        (
            "100 scientific assignments",
            (0..100)
                .map(|i| format!("v{i} = sin(1.23)"))
                .collect::<Vec<_>>()
                .join("\n"),
            30,
        ),
    ] {
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(engine::calculate(black_box(&text), &f));
        }
        println!(
            "{name}: calculate {:.3} ms",
            start.elapsed().as_secs_f64() * 1000.0 / iterations as f64
        );
        let mut e = editor::Editor::new(text, &f);
        e.caret = editor::Pos {
            line: 0,
            column: e.text.lines().next().unwrap().len(),
        };
        let start = Instant::now();
        for _ in 0..iterations.min(100) {
            e.key('x', &f);
            black_box(&e);
        }
        println!(
            "{name}: edit {:.3} ms",
            start.elapsed().as_secs_f64() * 1000.0 / iterations.min(100) as f64
        );
    }
}
