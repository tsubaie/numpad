use iced::advanced::text::{Highlighter, highlighter};
use iced::widget::canvas::{self, Frame, Geometry};
use iced::{Color, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use std::ops::Range;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    pub background: Color,
    pub paper: Color,
    pub card: Color,
    pub header: Color,
    pub text: Color,
    pub muted: Color,
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
    pub rows: Vec<u8>,
    pub mono: bool,
    pub colors: Colors,
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
        let style = self.settings.rows.get(self.line).copied().unwrap_or(0);
        self.line += 1;
        let h = |style| Highlight {
            style,
            mono: self.settings.mono,
            colors: self.settings.colors,
        };
        let mut spans = vec![];
        if style == 1 || style == 3 || style == 4 {
            spans.push((0..s.len(), h(style)));
        } else {
            let mut end = 0;
            let mut started = false;
            for (i, c) in s.char_indices() {
                if !started {
                    if c.is_ascii_digit() || c.is_ascii_alphabetic() {
                        started = true;
                        end = i + c.len_utf8();
                    }
                } else if c.is_whitespace() {
                    break;
                } else {
                    end = i + c.len_utf8();
                }
            }
            if end > 0 {
                spans.push((
                    0..end,
                    h(if s.trim_start().starts_with('-') {
                        2
                    } else {
                        0
                    }),
                ));
                if end < s.len() {
                    spans.push((end..s.len(), h(4)));
                }
            }
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
            4 => colors.muted,
            _ => colors.text,
        }),
        font: Some(Font {
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
        }),
    }
}
pub struct Paper {
    pub colors: Colors,
    pub ruled: bool,
    pub height: f32,
    pub active: usize,
    pub scroll: usize,
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
        for i in 0..((bounds.height / self.height) as usize + 1) {
            let y = 12.0 + i as f32 * self.height;
            let index = i + self.scroll;
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
            if self.ruled {
                f.fill_rectangle(
                    Point::new(0.0, y + self.height - 1.0),
                    Size::new(bounds.width, 1.0),
                    self.colors.rule,
                );
            }
        }
        vec![f.into_geometry()]
    }
}
