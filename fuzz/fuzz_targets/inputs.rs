#![no_main]
use libfuzzer_sys::fuzz_target;
use numpad::{engine, limits, storage::Document};

fuzz_target!(|data: &[u8]| {
    // Keep each execution useful and bounded; larger files have explicit tests.
    if data.len() > 8192 { return; }
    if let Ok(text) = std::str::from_utf8(data) {
        let format = engine::Format::default();
        let _ = engine::number(text, &format);
        let _ = engine::expression(text, &Default::default(), &format);
        if limits::text(text).is_ok() { let _ = engine::calculate(text, &format); }
    }
    if let Ok(mut document) = serde_json::from_slice::<Document>(data) {
        if document.validate().is_ok() {
            let _ = engine::calculate(&document.text, &document.preferences.format);
        }
    }
});
