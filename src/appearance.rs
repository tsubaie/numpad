use iced::advanced::text::{Highlighter, highlighter};
use iced::widget::canvas::{self, Frame, Geometry};
use iced::{Color, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use std::ops::Range;
use std::sync::OnceLock;

type CommentFace = Option<(String, iced::font::Style)>;
static COMMENT_FONTS: OnceLock<[CommentFace; 2]> = OnceLock::new();

pub fn initialize_fonts() {
    // Initialize before highlighting: the renderer holds this same font-system
    // lock while invoking the highlighter. Cache just two chosen family names.
    COMMENT_FONTS.get_or_init(|| {
        use iced::advanced::graphics::text::{cosmic_text::fontdb, font_system};
        let mut system = font_system().write().expect("font system lock");
        let db = system.raw().db();
        [false, true].map(|mono| {
            let generic = if mono {
                fontdb::Family::Monospace
            } else {
                fontdb::Family::SansSerif
            };
            let preferred = if mono {
                [
                    "Consolas",
                    "Menlo",
                    "DejaVu Sans Mono",
                    "Liberation Mono",
                    "Adwaita Mono",
                ]
            } else {
                [
                    "Arial",
                    "Helvetica",
                    "Noto Sans",
                    "DejaVu Sans",
                    "Liberation Sans",
                ]
            };
            let face = std::iter::once(generic)
                .chain(preferred.iter().map(|name| fontdb::Family::Name(name)))
                .find_map(|family| {
                    let id = db.query(&fontdb::Query {
                        families: &[family],
                        style: fontdb::Style::Italic,
                        ..Default::default()
                    })?;
                    db.face(id)
                        .filter(|face| face.style != fontdb::Style::Normal)
                })
                .or_else(|| {
                    db.faces().find(|face| {
                        face.monospaced == mono
                            && face.style != fontdb::Style::Normal
                            && face.weight == fontdb::Weight::NORMAL
                    })
                });
            face.and_then(|face| {
                face.families.first().map(|(name, _)| {
                    (
                        name.clone(),
                        if face.style == fontdb::Style::Oblique {
                            iced::font::Style::Oblique
                        } else {
                            iced::font::Style::Italic
                        },
                    )
                })
            })
        })
    });
}

fn comment_font(mono: bool) -> Font {
    let family = if mono {
        iced::font::Family::Monospace
    } else {
        iced::font::Family::SansSerif
    };
    match COMMENT_FONTS
        .get()
        .and_then(|fonts| fonts[usize::from(mono)].as_ref())
    {
        Some((name, style)) => Font {
            family: iced::font::Family::Name(name),
            style: *style,
            ..Font::DEFAULT
        },
        None => Font {
            family,
            style: iced::font::Style::Italic,
            ..Font::DEFAULT
        },
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    pub background: Color,
    pub paper: Color,
    pub card: Color,
    pub header: Color,
    pub text: Color,
    pub muted: Color,
    pub comment: Color,
    pub accent: Color,
    pub operator: Color,
    pub keypad: Color,
    pub rule: Color,
    pub selected: Color,
    pub negative: Color,
    pub custom: Color,
    pub on_accent: Color,
}
fn hex(h: u32) -> Color {
    Color::from_rgb8((h >> 16) as u8, (h >> 8) as u8, h as u8)
}
impl Colors {
    pub fn new(dark: bool) -> Self {
        // Catppuccin Mocha: https://catppuccin.com/palette/
        if dark {
            Self {
                background: hex(0x181825),
                paper: hex(0x1e1e2e),
                card: hex(0x1e1e2e),
                header: hex(0x181825),
                text: hex(0xcdd6f4),
                muted: hex(0xa6adc8),
                comment: hex(0xa6e3a1),
                accent: hex(0xcba6f7),
                operator: hex(0x45475a),
                keypad: hex(0x313244),
                rule: hex(0x313244),
                selected: hex(0x313244),
                negative: hex(0xf38ba8),
                custom: hex(0x313244),
                on_accent: hex(0x1e1e2e),
            }
        } else {
            Self {
                background: hex(0xf2f4f6),
                paper: hex(0xffffff),
                card: hex(0xffffff),
                header: hex(0xf2f4f6),
                text: hex(0x293640),
                muted: hex(0x667784),
                comment: hex(0x287a3d),
                accent: hex(0x217d80),
                operator: hex(0xe0f0f0),
                keypad: hex(0xf2f4f6),
                rule: hex(0xe8edf0),
                selected: hex(0xedf7f6),
                negative: hex(0xb44255),
                custom: hex(0xf3f0e7),
                on_accent: hex(0xffffff),
            }
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct HighlightSettings {
    pub rows: Vec<HighlightRow>,
    pub mono: bool,
    pub colors: Colors,
}
#[derive(Clone, Copy, Default, PartialEq)]
pub struct HighlightRow {
    pub style: u8,
    pub comment_start: Option<usize>,
}
impl HighlightRow {
    pub fn from_line(line: &numpad::engine::Line) -> Self {
        use numpad::engine::Kind;
        let style = if line.error.is_some() {
            3
        } else {
            match line.kind {
                Kind::Total => 1,
                Kind::Comment => 4,
                Kind::Rule => 5,
                Kind::Value if line.op == '-' => 2,
                _ => 0,
            }
        };
        let comment_start = if line.kind == Kind::Comment {
            Some(0)
        } else if !line.comment.is_empty() && line.raw.trim_end().ends_with(&line.comment) {
            Some(line.raw.trim_end().len() - line.comment.len())
        } else {
            None
        };
        Self {
            style,
            comment_start,
        }
    }
}
pub struct TapeHighlighter {
    settings: HighlightSettings,
    line: usize,
}
#[derive(Clone, Copy)]
pub struct Highlight {
    pub style: u8,
    pub mono: bool,
    pub colors: Colors,
}
impl Highlighter for TapeHighlighter {
    type Settings = HighlightSettings;
    type Highlight = Highlight;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Highlight)>;
    fn new(settings: &Self::Settings) -> Self {
        Self {
            settings: settings.clone(),
            line: 0,
        }
    }
    fn update(&mut self, s: &Self::Settings) {
        self.settings = s.clone();
        self.line = 0;
    }
    fn change_line(&mut self, line: usize) {
        self.line = line;
    }
    fn current_line(&self) -> usize {
        self.line
    }
    fn highlight_line(&mut self, s: &str) -> Self::Iterator<'_> {
        let row = self
            .settings
            .rows
            .get(self.line)
            .copied()
            .unwrap_or_default();
        self.line += 1;
        let h = |style| Highlight {
            style,
            mono: self.settings.mono,
            colors: self.settings.colors,
        };
        let mut spans = vec![];
        let start = row
            .comment_start
            .filter(|&i| i <= s.len() && s.is_char_boundary(i))
            .unwrap_or(s.len());
        if start > 0 {
            spans.push((0..start, h(row.style)));
        }
        if start < s.len() {
            spans.push((start..s.len(), h(4)));
        }
        if s.is_empty() {
            spans.push((0..0, h(row.style)));
        }
        spans.into_iter()
    }
}
pub fn highlight(h: &Highlight, _: &Theme) -> highlighter::Format<Font> {
    let colors = h.colors;
    highlighter::Format {
        color: Some(match h.style {
            1 => colors.accent,
            2 | 3 => colors.negative,
            4 => colors.comment,
            5 => colors.muted,
            _ => colors.text,
        }),
        font: Some(if h.style == 4 {
            comment_font(h.mono)
        } else {
            Font {
                family: if h.mono {
                    iced::font::Family::Monospace
                } else {
                    iced::font::Family::SansSerif
                },
                weight: if h.style == 1 {
                    iced::font::Weight::Bold
                } else {
                    iced::font::Weight::Normal
                },
                ..Font::DEFAULT
            }
        }),
    }
}

pub struct Paper {
    pub colors: Colors,
    pub ruled: bool,
    pub height: f32,
    pub active: usize,
    pub scroll: f32,
    pub rows: Vec<u8>,
}
impl<Message> canvas::Program<Message> for Paper {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        f.fill_rectangle(Point::ORIGIN, bounds.size(), self.colors.paper);
        let first = (self.scroll / self.height).floor() as usize;
        for i in 0..((bounds.height / self.height) as usize + 2) {
            let index = first + i;
            let y = 12.0 + index as f32 * self.height - self.scroll;
            let total = self.rows.get(index) == Some(&2);
            if total {
                f.fill_rectangle(
                    Point::new(0.0, y),
                    Size::new(bounds.width, self.height),
                    Color {
                        a: 0.10,
                        ..self.colors.accent
                    },
                );
            }
            if index == self.active {
                f.fill_rectangle(
                    Point::new(0.0, y),
                    Size::new(bounds.width, self.height),
                    self.colors.selected,
                );
                f.fill_rectangle(
                    Point::new(0.0, y),
                    Size::new(4.0, self.height),
                    self.colors.accent,
                );
            }
            if self.rows.get(index).copied().unwrap_or(0) > 0 {
                f.fill_rectangle(
                    Point::new(6.0, y),
                    Size::new(3.0, self.height),
                    if self.rows[index] == 3 {
                        self.colors.negative
                    } else {
                        self.colors.keypad
                    },
                );
            }
            if self.ruled || total {
                f.fill_rectangle(
                    Point::new(0.0, y + self.height - 1.0),
                    Size::new(bounds.width, if total { 2.0 } else { 1.0 }),
                    if total {
                        self.colors.accent
                    } else {
                        self.colors.rule
                    },
                );
            }
        }
        vec![f.into_geometry()]
    }
}

/// The seven UI marks use the existing canvas renderer, not an SVG decoder.
pub struct Icon {
    pub kind: &'static str,
    pub color: Color,
}
impl<Message> canvas::Program<Message> for Icon {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.scale_nonuniform(iced::Vector::new(bounds.width / 24.0, bounds.height / 24.0));
        let path = canvas::Path::new(|p| {
            let mut line = |points: &[(f32, f32)]| {
                if let Some(&(x, y)) = points.first() {
                    p.move_to(Point::new(x, y));
                    for &(x, y) in &points[1..] {
                        p.line_to(Point::new(x, y));
                    }
                }
            };
            match self.kind {
                "menu" => {
                    line(&[(4., 6.), (20., 6.)]);
                    line(&[(4., 12.), (20., 12.)]);
                    line(&[(4., 18.), (20., 18.)]);
                }
                "copy" => {
                    line(&[
                        (16., 8.),
                        (16., 5.),
                        (14., 3.),
                        (5., 3.),
                        (3., 5.),
                        (3., 14.),
                        (5., 16.),
                        (8., 16.),
                    ]);
                    p.rounded_rectangle(Point::new(8., 8.), Size::new(13., 13.), 2.0.into());
                }
                "check" => line(&[(5., 12.), (9., 16.), (19., 6.)]),
                "close" => {
                    line(&[(6., 6.), (18., 18.)]);
                    line(&[(18., 6.), (6., 18.)]);
                }
                "undo" | "redo" => {
                    let flip = self.kind == "redo";
                    let pt = |x, y| Point::new(if flip { 24.0 - x } else { x }, y);
                    p.move_to(pt(9., 4.));
                    p.line_to(pt(3., 10.));
                    p.line_to(pt(9., 16.));
                    p.move_to(pt(3., 10.));
                    p.line_to(pt(14., 10.));
                    p.bezier_curve_to(pt(18., 10.), pt(21., 13.), pt(21., 17.));
                    p.line_to(pt(21., 20.));
                }
                _ => {
                    line(&[
                        (9., 4.),
                        (21., 4.),
                        (21., 20.),
                        (9., 20.),
                        (2., 12.),
                        (9., 4.),
                    ]);
                    line(&[(12., 9.), (18., 15.)]);
                    line(&[(18., 9.), (12., 15.)]);
                }
            }
        });
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_color(self.color)
                .with_width(1.8)
                .with_line_cap(canvas::LineCap::Round)
                .with_line_join(canvas::LineJoin::Round),
        );
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comments_are_italic_green_without_restyling_expressions_or_totals() {
        initialize_fonts();
        let tape = numpad::engine::calculate(
            "100\n+15% tax note\n---\n+115 total note\n\nplain note\nx = 10+2",
            &numpad::engine::Format::default(),
        );
        for dark in [false, true] {
            let colors = Colors::new(dark);
            let mut highlighter = TapeHighlighter::new(&HighlightSettings {
                rows: tape.lines.iter().map(HighlightRow::from_line).collect(),
                mono: true,
                colors,
            });
            for line in &tape.lines {
                let spans: Vec<_> = highlighter.highlight_line(&line.raw).collect();
                for (range, h) in spans {
                    let format = highlight(&h, &Theme::Light);
                    if h.style == 4 {
                        assert!(line.raw[range].contains("note"));
                        assert_eq!(format.color, Some(colors.comment));
                        assert!(matches!(
                            format.font.unwrap().style,
                            iced::font::Style::Italic | iced::font::Style::Oblique
                        ));
                    } else {
                        assert_eq!(format.font.unwrap().style, iced::font::Style::Normal);
                    }
                }
            }
            assert_eq!(HighlightRow::from_line(&tape.lines[3]).style, 1);
            assert_eq!(HighlightRow::from_line(&tape.lines[6]).style, 0);
        }
    }
}
