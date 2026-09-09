use crate::{
    engine::{self, Format, Kind},
    math::Number,
};
use directories::ProjectDirs;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub format: Format,
    pub dark: bool,
    pub ruled: bool,
    pub mono: bool,
    pub zoom: f32,
    #[serde(skip_serializing)]
    pub custom: [CustomKey; 2],
    pub tax: Option<TaxSettings>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomKey {
    pub label: String,
    pub formula: String,
    pub calculate: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            format: Format::default(),
            dark: false,
            ruled: true,
            mono: true,
            zoom: 1.0,
            tax: None,
            custom: [
                CustomKey {
                    label: "+ VAT (15%)".into(),
                    formula: "+15%".into(),
                    calculate: true,
                },
                CustomKey {
                    label: "− VAT (15%)".into(),
                    formula: "/1.15".into(),
                    calculate: true,
                },
            ],
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxSettings {
    pub label: String,
    pub rate: String,
    pub calculate: bool,
}
impl Default for TaxSettings {
    fn default() -> Self {
        Self {
            label: "VAT".into(),
            rate: "15".into(),
            calculate: true,
        }
    }
}
impl TaxSettings {
    pub fn rate_label(&self, comma: bool) -> String {
        let rate = self
            .validate()
            .map(|v| v.normalized().to_plain_string())
            .unwrap_or_else(|_| "?".into());
        if comma { rate.replace('.', ",") } else { rate }
    }
    pub fn validate(&self) -> Result<Number, String> {
        if self.label.trim().is_empty() {
            return Err("Enter a tax name".into());
        }
        let n = engine::number(&self.rate.replace(',', "."), &Format::default())?;
        if !(Number::from(0)..=Number::from(1000)).contains(&n) {
            return Err("Tax rate must be between 0 and 1000 percent".into());
        }
        Ok(n)
    }
    pub fn formula(&self, remove: bool, f: &Format) -> Result<String, String> {
        let rate = self.validate()?;
        let value = if remove {
            crate::math::binary(
                &Number::from(1),
                &crate::math::div(&rate, &Number::from(100))?,
                '+',
            )?
        } else {
            rate
        };
        let value = value.normalized().to_plain_string();
        let value = if f.comma {
            value.replace('.', ",")
        } else {
            value
        };
        Ok(if remove {
            format!("/{value}")
        } else {
            format!("+{value}%")
        })
    }
}
impl Preferences {
    pub fn tax_settings(&self) -> TaxSettings {
        self.tax.clone().unwrap_or_else(|| {
            let old = &self.custom[0];
            let name = old
                .label
                .trim_start_matches(['+', '-', '−', ' '])
                .split('(')
                .next()
                .unwrap_or("VAT")
                .trim();
            let candidate = TaxSettings {
                label: if name.is_empty() {
                    "VAT".into()
                } else {
                    name.into()
                },
                rate: old
                    .formula
                    .trim()
                    .trim_start_matches('+')
                    .trim_end_matches('%')
                    .to_string(),
                calculate: old.calculate,
            };
            if candidate.validate().is_ok() {
                candidate
            } else {
                TaxSettings::default()
            }
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub version: u32,
    pub text: String,
    #[serde(default)]
    pub preferences: Preferences,
    #[serde(default)]
    pub memory: String,
}
impl Document {
    pub fn new(text: String, mut preferences: Preferences, memory: String) -> Self {
        preferences.tax = Some(preferences.tax_settings());
        Self {
            version: 2,
            text,
            preferences,
            memory,
        }
    }
}
pub fn data_directory() -> PathBuf {
    if let Some(dir) = std::env::var_os("NUMPAD_DATA_DIR") {
        return PathBuf::from(dir);
    }
    ProjectDirs::from("app", "NumPad", "NumPad")
        .map(|p| p.data_local_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".numpad"))
}
pub fn session_path() -> PathBuf {
    data_directory().join("session.numpad")
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create folder: {e}"))?;
    }
    let temporary = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    let mut file = fs::File::create(&temporary).map_err(|e| format!("Cannot save file: {e}"))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| format!("Cannot write file: {e}"))?;
    drop(file);
    fs::rename(&temporary, path).map_err(|e| format!("Cannot finish saving file: {e}"))
}
pub fn save(path: &Path, doc: &Document) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(doc).map_err(|e| e.to_string())?;
    atomic_write(path, &bytes)
}
pub fn load(path: &Path) -> Result<Document, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.len() > 2_000_000 {
        return Err("File exceeds the 2 MB document limit".into());
    }
    let text = fs::read_to_string(path).map_err(|e| format!("Cannot read file: {e}"))?;
    if path
        .extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("txt"))
    {
        return Ok(Document::new(text, Preferences::default(), "0".into()));
    }
    let mut doc: Document =
        serde_json::from_str(&text).map_err(|e| format!("Not a valid NumPad document: {e}"))?;
    if !matches!(doc.version, 1 | 2) {
        return Err(format!("Unsupported document version {}", doc.version));
    }
    if doc.text.len() > 1_000_000 {
        return Err("Document exceeds the text limit".into());
    }
    doc.preferences.zoom = doc.preferences.zoom.clamp(0.6, 2.0);
    doc.preferences.format.digits = doc.preferences.format.digits.min(12);
    Ok(doc)
}
pub fn text_export(text: &str, f: &Format) -> String {
    engine::calculate(text, f)
        .lines
        .iter()
        .map(|l| engine::render(l, f))
        .collect::<Vec<_>>()
        .join("\n")
}
pub fn export_xlsx(path: &Path, text: &str, f: &Format) -> Result<(), String> {
    use rust_xlsxwriter::{Color, Format as XFormat, Workbook};
    let mut book = Workbook::new();
    let sheet = book.add_worksheet();
    sheet.set_name("Calculation").map_err(|e| e.to_string())?;
    let header = XFormat::new()
        .set_bold()
        .set_background_color(Color::RGB(0x203A2E))
        .set_font_color(Color::White);
    let total = XFormat::new()
        .set_bold()
        .set_font_color(Color::RGB(0x166345))
        .set_num_format("#,##0.00;[Red]-#,##0.00");
    let numeric = XFormat::new().set_num_format("#,##0.00;[Red]-#,##0.00");
    let columns = [
        "Operator",
        "Value / variable",
        "Comment",
        "Calculated value",
        "Error",
    ];
    for (i, h) in columns.iter().enumerate() {
        sheet
            .write_string_with_format(0, i as u16, *h, &header)
            .map_err(|e| e.to_string())?;
    }
    sheet.set_column_width(0, 12).map_err(|e| e.to_string())?;
    sheet.set_column_width(1, 24).map_err(|e| e.to_string())?;
    sheet.set_column_width(2, 64).map_err(|e| e.to_string())?;
    sheet.set_column_width(3, 24).map_err(|e| e.to_string())?;
    sheet.set_column_width(4, 48).map_err(|e| e.to_string())?;
    let tape = engine::calculate(text, f);
    for (i, l) in tape.lines.iter().enumerate() {
        let row = (i + 1) as u32;
        let style = if l.kind == Kind::Total {
            &total
        } else {
            &numeric
        };
        if matches!(l.kind, Kind::Value | Kind::Total) {
            sheet
                .write_string(
                    row,
                    0,
                    if l.kind == Kind::Total {
                        "=".into()
                    } else {
                        l.op.to_string()
                    },
                )
                .map_err(|e| e.to_string())?;
            let value = if l.kind == Kind::Total {
                l.value
                    .as_ref()
                    .map(|v| engine::format(v, f))
                    .unwrap_or_default()
            } else {
                format!("{}{}", l.operand, if l.percent { "%" } else { "" })
            };
            sheet
                .write_string_with_format(row, 1, &value, style)
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(
                    row,
                    2,
                    format!(
                        "{}{}",
                        if l.name.is_empty() {
                            String::new()
                        } else {
                            format!("{}: ", l.name)
                        },
                        l.comment
                    ),
                )
                .map_err(|e| e.to_string())?;
            if let Some(v) = l.amount.as_ref().or(l.value.as_ref()) {
                // Excel is limited to 15 significant decimal digits; preserve larger values as text.
                let exact = v.normalized().to_plain_string();
                if exact.chars().filter(char::is_ascii_digit).count() > 15 {
                    sheet
                        .write_string_with_format(row, 3, &exact, style)
                        .map_err(|e| e.to_string())?;
                } else if let Some(n) = v.to_f64() {
                    sheet
                        .write_number_with_format(row, 3, n, style)
                        .map_err(|e| e.to_string())?;
                }
            }
        } else {
            sheet
                .write_string(row, 2, &l.raw)
                .map_err(|e| e.to_string())?;
        }
        if let Some(error) = &l.error {
            sheet
                .write_string(row, 4, error)
                .map_err(|e| e.to_string())?;
        }
    }
    sheet.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;
    atomic_write(path, &book.save_to_buffer().map_err(|e| e.to_string())?)
}
pub fn export_pdf(path: &Path, text: &str, f: &Format) -> Result<(), String> {
    use printpdf::*;
    let mut pdf = PdfDocument::new("NumPad calculation");
    // Use an installed font; no system font is redistributed with the application.
    let font_paths = [
        "C:/Windows/Fonts/consola.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
    ];
    let custom_font = font_paths
        .iter()
        .find_map(|p| fs::read(p).ok())
        .and_then(|bytes| ParsedFont::from_bytes(&bytes, 0, &mut Vec::new()))
        .map(|font| pdf.add_font(&font));
    let rendered = text_export(text, f);
    if custom_font.is_none() && !rendered.is_ascii() {
        return Err("PDF export needs an installed Consolas, DejaVu Sans Mono or Courier New font for Unicode text. Text and Excel export remain available.".into());
    }
    let mut rows = vec![];
    for row in rendered.lines() {
        let chars: Vec<char> = row.chars().collect();
        if chars.is_empty() {
            rows.push(String::new());
        } else {
            for chunk in chars.chunks(92) {
                rows.push(chunk.iter().collect::<String>());
            }
        }
    }
    if rows.is_empty() {
        rows.push(String::new());
    }
    let mut pages = vec![];
    for (page_index, chunk) in rows.chunks(54).enumerate() {
        let mut ops = vec![
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point::new(Mm(16.0), Mm(281.0)),
            },
            Op::SetFontSizeBuiltinFont {
                size: Pt(17.0),
                font: BuiltinFont::HelveticaBold,
            },
            Op::WriteTextBuiltinFont {
                items: vec![TextItem::Text("NumPad".into())],
                font: BuiltinFont::HelveticaBold,
            },
            Op::EndTextSection,
        ];
        for (i, line) in chunk.iter().enumerate() {
            ops.push(Op::StartTextSection);
            ops.push(Op::SetTextCursor {
                pos: Point::new(Mm(16.0), Mm(266.0 - i as f32 * 4.3)),
            });
            if let Some(font) = &custom_font {
                ops.push(Op::SetFontSize {
                    size: Pt(9.0),
                    font: font.clone(),
                });
                ops.push(Op::WriteText {
                    items: vec![TextItem::Text(line.clone())],
                    font: font.clone(),
                });
            } else {
                ops.push(Op::SetFontSizeBuiltinFont {
                    size: Pt(9.0),
                    font: BuiltinFont::Courier,
                });
                ops.push(Op::WriteTextBuiltinFont {
                    items: vec![TextItem::Text(line.clone())],
                    font: BuiltinFont::Courier,
                });
            }
            ops.push(Op::EndTextSection);
        }
        ops.extend([
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point::new(Mm(16.0), Mm(14.0)),
            },
            Op::SetFontSizeBuiltinFont {
                size: Pt(8.0),
                font: BuiltinFont::Helvetica,
            },
            Op::WriteTextBuiltinFont {
                items: vec![TextItem::Text(format!(
                    "Page {}  |  Exported from NumPad",
                    page_index + 1
                ))],
                font: BuiltinFont::Helvetica,
            },
            Op::EndTextSection,
        ]);
        pages.push(PdfPage::new(Mm(210.0), Mm(297.0), ops));
    }
    let bytes = pdf
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut Vec::new());
    atomic_write(path, &bytes)
}
pub fn memory_value(text: &str) -> Number {
    engine::number(text, &Format::default()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_round_trip() {
        let d = Document::new(
            "+100\n+15%\n---\n+115".into(),
            Preferences::default(),
            "20".into(),
        );
        let bytes = serde_json::to_vec(&d).unwrap();
        let restored: Document = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(d.text, restored.text);
        assert_eq!(restored.memory, "20");
    }
    #[test]
    fn legacy_palette_is_ignored_without_losing_theme_or_tape() {
        let json = r#"{"version":1,"text":"100\n+15%","preferences":{"palette":"High Contrast","dark":true},"memory":"16"}"#;
        let restored: Document = serde_json::from_str(json).unwrap();
        assert!(restored.preferences.dark);
        assert_eq!(restored.memory, "16");
        assert_eq!(restored.text, "100\n+15%");
        assert!(
            !serde_json::to_string(&restored)
                .unwrap()
                .contains("palette")
        );
    }
}
#[cfg(test)]
mod tax_tests {
    use super::*;
    #[test]
    fn visible_tax_rate_comes_from_settings() {
        assert_eq!(
            Preferences::default().tax_settings().rate_label(false),
            "15"
        );
        let tax = TaxSettings {
            rate: "14.50".into(),
            ..TaxSettings::default()
        };
        assert_eq!(tax.rate_label(false), "14.5");
        assert_eq!(tax.rate_label(true), "14,5");
        let prefs: Preferences = serde_json::from_str(r#"{"language":"Deutsch"}"#).unwrap();
        assert!(!serde_json::to_string(&prefs).unwrap().contains("language"));
    }
    #[test]
    fn both_tax_directions_share_one_rate() {
        let key = TaxSettings {
            label: "VAT".into(),
            rate: "14".into(),
            calculate: true,
        };
        let f = Format::default();
        assert_eq!(key.formula(false, &f).unwrap(), "+14%");
        assert_eq!(key.formula(true, &f).unwrap(), "/1.14");
        let text = format!(
            "100\n{}\n{}\n---\n+0",
            key.formula(false, &f).unwrap(),
            key.formula(true, &f).unwrap()
        );
        assert_eq!(
            engine::calculate(&text, &f).grand.normalized().to_string(),
            "100"
        );
        assert_eq!(
            key.formula(true, &Format { comma: true, ..f }).unwrap(),
            "/1,14"
        );
    }
    #[test]
    fn legacy_tax_migrates_from_add_key_and_strips_rate_from_name() {
        let mut prefs = Preferences::default();
        prefs.custom[0].label = "+ VAT (15%)1".into();
        prefs.custom[0].formula = "+14%".into();
        let tax = prefs.tax_settings();
        assert_eq!(tax.label, "VAT");
        assert_eq!(tax.rate, "14");
        assert_eq!(tax.formula(true, &Format::default()).unwrap(), "/1.14");
    }
    #[test]
    fn tax_rates_are_validated() {
        for rate in ["-1", "1001", "oops", ""] {
            assert!(
                TaxSettings {
                    rate: rate.into(),
                    ..TaxSettings::default()
                }
                .validate()
                .is_err()
            );
        }
    }
}
