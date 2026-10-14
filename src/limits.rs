//! Shared bounds for typed, imported and restored documents.
pub const TEXT_BYTES: usize = 1_000_000;
pub const LINES: usize = 500;
// JSON escaping can expand each text byte to six bytes.
pub const DOCUMENT_BYTES: usize = TEXT_BYTES * 6 + 64_000;
pub const WORKSPACE_BYTES: usize = 64 * 1024 * 1024;
pub const WORKSPACE_TEXT_BYTES: usize = 8 * TEXT_BYTES;
pub const TABS: usize = 64;
pub const HISTORY_BYTES: usize = 8 * 1024 * 1024;
pub const WORKSPACE_HISTORY_BYTES: usize = 32 * 1024 * 1024;

pub fn text(text: &str) -> Result<(), String> {
    if text.len() > TEXT_BYTES {
        return Err("Document exceeds the 1 MB text limit.".into());
    }
    if text.bytes().filter(|b| *b == b'\n').count() + 1 > LINES {
        return Err("Document exceeds the 500-line limit.".into());
    }
    Ok(())
}
