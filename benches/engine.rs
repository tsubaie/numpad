use numpad::{editor, engine};
use std::{hint::black_box, time::Instant};
fn main() {
    let f = engine::Format::default();
    for (name, text, iterations) in [
        ("20 arithmetic lines", vec!["+ 123.45"; 20].join("\n"), 200),
        (
            "500 arithmetic lines",
            vec!["+ 123.45"; 500].join("\n"),
            200,
        ),
        (
            "500 lines, 950KB notes",
            vec![format!("note {}", "x".repeat(1894)); 500].join("\n"),
            100,
        ),
        (
            "500 scientific, repeated argument",
            (0..500)
                .map(|i| format!("v{i} = sin(1.23)"))
                .collect::<Vec<_>>()
                .join("\n"),
            100,
        ),
        (
            "500 scientific, distinct arguments",
            (0..500)
                .map(|i| format!("v{i} = sin({i}/100)"))
                .collect::<Vec<_>>()
                .join("\n"),
            30,
        ),
    ] {
        for _ in 0..5 {
            black_box(engine::calculate(&text, &f));
        }
        let mut calculations = Vec::new();
        let mut edits = Vec::new();
        for _ in 0..iterations {
            let start = Instant::now();
            black_box(engine::calculate(black_box(&text), &f));
            calculations.push(start.elapsed().as_secs_f64() * 1000.0);
            // Each sample starts at the same document, caret and history state.
            let mut e = editor::Editor::new(text.clone(), &f);
            e.caret = editor::Pos {
                line: 0,
                column: e.text.lines().next().unwrap().len(),
            };
            let start = Instant::now();
            e.key(' ', &f);
            black_box(&e);
            edits.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        for (operation, mut samples) in [("calculate", calculations), ("edit", edits)] {
            samples.sort_by(f64::total_cmp);
            println!(
                "{name}: {operation} median {:.3} ms; p95 {:.3} ms",
                samples[samples.len() / 2],
                samples[(samples.len() * 95 / 100).min(samples.len() - 1)]
            );
        }
        if name.contains("scientific") {
            let mut cold = Vec::new();
            for _ in 0..20 {
                let input = text.clone();
                let format = f.clone();
                cold.push(
                    std::thread::spawn(move || {
                        // The cache is thread-local. Exclude thread startup from
                        // timing while measuring a genuinely empty function cache.
                        let start = Instant::now();
                        black_box(engine::calculate(&input, &format));
                        start.elapsed().as_secs_f64() * 1000.0
                    })
                    .join()
                    .unwrap(),
                );
            }
            cold.sort_by(f64::total_cmp);
            println!(
                "{name}: cold cache median {:.3} ms; p95 {:.3} ms",
                cold[10], cold[19]
            );
        }
    }
}
