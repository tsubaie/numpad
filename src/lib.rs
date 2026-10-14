//! Calculation, editing and document services, independent of the desktop UI.
pub mod editor;
pub mod engine;
pub mod limits;
pub mod math;
pub mod pdf_export;
pub mod storage;

pub const INTRO: &str = "Welcome to NumPad\nTape operations run from top to bottom.\nType 10+2*3, then Enter: (10+2)*3 gives 36.\n\n +          10.00\n +           2.00\n *           3.00\n -----------------\n +          36.00\n\nAssignments use standard precedence: multiplication first.\nx = 10+2*3\n\nLeave a blank line for a separate calculation.\nPress Enter to insert a subtotal.\nUse + in the tab bar for a separate tape.\nOpen ? for worked examples and shortcuts.\n";

#[cfg(test)]
mod tests;
