//! A small text-only PDF backend. No HTML, SVG, image decoding, or PDF importing.
use flate2::{Compression, write::ZlibEncoder};
use pdf_writer::{
    Content, Finish, Name, Pdf, Rect, Ref, Str, TextStr,
    types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
};

const BODY: Ref = Ref::new(3);
const HEADING: Ref = Ref::new(4);
const FOOTER: Ref = Ref::new(5);
const FONT_NAME: Name<'static> = Name(b"NUMPAD+Tape");
const CID_INFO: SystemInfo<'static> = SystemInfo {
    registry: Str(b"Adobe"),
    ordering: Str(b"Identity"),
    supplement: 0,
};

fn compressed(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())
}
fn stream(pdf: &mut Pdf, id: Ref, data: &[u8]) -> Result<(), String> {
    let bytes = compressed(data)?;
    pdf.stream(id, &bytes)
        .filter(pdf_writer::Filter::FlateDecode);
    Ok(())
}

fn embed_font(pdf: &mut Pdf, data: &[u8], text: &str) -> Result<BTreeMap<char, u16>, String> {
    let face =
        ttf_parser::Face::parse(data, 0).map_err(|e| format!("Cannot parse PDF font: {e:?}"))?;
    if face.tables().glyf.is_none() {
        return Err("PDF export requires a TrueType outline font.".into());
    }
    let chars: BTreeSet<_> = text.chars().filter(|c| *c != '\n' && *c != '\r').collect();
    if chars.len() >= u16::MAX as usize {
        return Err("Too many distinct characters for a PDF font.".into());
    }
    let mut mapper = subsetter::GlyphRemapper::new();
    let mut encoding = BTreeMap::new();
    let mut glyph_map = vec![0, 0]; // CID 0 is .notdef.
    let mut widths = vec![0.0];
    let mut cmap = UnicodeCmap::new(Name(b"NumPad-Unicode"), CID_INFO);
    let scale = 1000.0 / f32::from(face.units_per_em());
    for (i, c) in chars.into_iter().enumerate() {
        let glyph = face.glyph_index(c).ok_or_else(|| {
            format!(
                "The installed PDF font has no glyph for U+{:04X}. Try text or Excel export.",
                c as u32
            )
        })?;
        let cid = (i + 1) as u16;
        let remapped = mapper.remap(glyph.0);
        glyph_map.extend(remapped.to_be_bytes());
        widths.push(f32::from(face.glyph_hor_advance(glyph).unwrap_or(0)) * scale);
        cmap.pair(cid, c);
        encoding.insert(c, cid);
    }
    let subset =
        subsetter::subset(data, 0, &mapper).map_err(|e| format!("Cannot subset PDF font: {e}"))?;
    let cid = Ref::new(6);
    let descriptor = Ref::new(7);
    let font_file = Ref::new(8);
    let unicode = Ref::new(9);
    let mapping = Ref::new(10);
    pdf.type0_font(BODY)
        .base_font(FONT_NAME)
        .encoding_predefined(Name(b"Identity-H"))
        .descendant_font(cid)
        .to_unicode(unicode);
    let mut font = pdf.cid_font(cid);
    font.subtype(CidFontType::Type2)
        .base_font(FONT_NAME)
        .system_info(CID_INFO)
        .font_descriptor(descriptor)
        .cid_to_gid_map_stream(mapping);
    font.widths().consecutive(0, widths);
    font.finish();
    let bbox = face.global_bounding_box();
    pdf.font_descriptor(descriptor)
        .name(FONT_NAME)
        .flags(FontFlags::SYMBOLIC | FontFlags::FIXED_PITCH)
        .bbox(Rect::new(
            f32::from(bbox.x_min) * scale,
            f32::from(bbox.y_min) * scale,
            f32::from(bbox.x_max) * scale,
            f32::from(bbox.y_max) * scale,
        ))
        .italic_angle(face.italic_angle())
        .ascent(f32::from(face.ascender()) * scale)
        .descent(f32::from(face.descender()) * scale)
        .cap_height(f32::from(face.capital_height().unwrap_or(face.ascender())) * scale)
        .stem_v(80.0)
        .font_file2(font_file);
    let compressed_font = compressed(&subset)?;
    pdf.stream(font_file, &compressed_font)
        .filter(pdf_writer::Filter::FlateDecode)
        .pair(Name(b"Length1"), subset.len() as i32);
    stream(pdf, unicode, &cmap.finish())?;
    stream(pdf, mapping, &glyph_map)?;
    Ok(encoding)
}

pub fn render(text: &str, font: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let text = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\t', "    ");
    let mut pdf = Pdf::new();
    pdf.catalog(Ref::new(1)).pages(Ref::new(2));
    pdf.document_info(Ref::new(11))
        .title(TextStr("NumPad calculation"))
        .producer(TextStr("NumPad"));
    let encoding = if let Some(font) = font {
        Some(embed_font(&mut pdf, font, &text)?)
    } else {
        if !text.is_ascii() {
            return Err("PDF export needs an installed Consolas, DejaVu Sans Mono or Courier New font for Unicode text. Text and Excel export remain available.".into());
        }
        pdf.type1_font(BODY).base_font(Name(b"Courier"));
        None
    };
    pdf.type1_font(HEADING).base_font(Name(b"Helvetica-Bold"));
    pdf.type1_font(FOOTER).base_font(Name(b"Helvetica"));
    let mut rows = Vec::new();
    for row in text.lines() {
        let mut line = String::new();
        let mut count = 0;
        for c in row.chars() {
            if count == 92 {
                rows.push(std::mem::take(&mut line));
                count = 0;
            }
            line.push(c);
            count += 1;
        }
        rows.push(line);
    }
    if rows.is_empty() {
        rows.push(String::new());
    }
    let page_ids: Vec<_> = (0..rows.len().div_ceil(54))
        .map(|i| Ref::new(12 + i as i32 * 2))
        .collect();
    pdf.pages(Ref::new(2))
        .kids(page_ids.iter().copied())
        .count(page_ids.len() as i32);
    let mm = |value: f32| value * 72.0 / 25.4;
    for (i, (id, rows)) in page_ids.into_iter().zip(rows.chunks(54)).enumerate() {
        let content_id = Ref::new(id.get() + 1);
        let mut page = pdf.page(id);
        page.parent(Ref::new(2))
            .media_box(Rect::new(0., 0., mm(210.), mm(297.)))
            .contents(content_id);
        page.resources()
            .fonts()
            .pair(Name(b"Body"), BODY)
            .pair(Name(b"Heading"), HEADING)
            .pair(Name(b"Footer"), FOOTER);
        page.finish();
        let mut content = Content::new();
        let mut line = |font, size, y, bytes: &[u8]| {
            content
                .begin_text()
                .set_font(font, size)
                .next_line(mm(16.), mm(y))
                .show(Str(bytes))
                .end_text();
        };
        line(Name(b"Heading"), 17., 281., b"NumPad");
        for (j, text) in rows.iter().enumerate() {
            let bytes = if let Some(encoding) = &encoding {
                text.chars()
                    .flat_map(|c| encoding[&c].to_be_bytes())
                    .collect::<Vec<_>>()
            } else {
                text.as_bytes().to_vec()
            };
            line(Name(b"Body"), 9., 266. - j as f32 * 4.3, &bytes);
        }
        line(
            Name(b"Footer"),
            8.,
            14.,
            format!("Page {}  |  Exported from NumPad", i + 1).as_bytes(),
        );
        stream(&mut pdf, content_id, &content.finish())?;
    }
    Ok(pdf.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_line_endings_have_identical_pdf_layout() {
        let expected = render("one\ntwo\nthree", None).unwrap();
        assert_eq!(render("one\rtwo\r\nthree", None).unwrap(), expected);
    }
    #[test]
    fn ascii_fallback_is_valid_and_unicode_requires_a_font() {
        assert!(render("100\n+20", None).unwrap().starts_with(b"%PDF-"));
        assert!(render("café", None).unwrap_err().contains("Unicode"));
        assert!(render("", None).is_ok());
        assert!(render("text", Some(b"invalid font")).is_err());
    }
}
