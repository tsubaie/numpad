use crate::appearance::Colors;
use iced::Color;
use std::path::PathBuf;

pub fn omarchy_colors() -> Option<Colors> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let dirs = directories::BaseDirs::new()?;
    let paths = [
        dirs.state_dir()
            .unwrap_or(dirs.data_local_dir())
            .join("omarchy/current/theme/colors.toml"),
        dirs.config_dir().join("omarchy/current/theme/colors.toml"),
    ];
    paths
        .iter()
        .find_map(|path: &PathBuf| std::fs::read_to_string(path).ok().and_then(|s| parse(&s)))
}

fn blend(a: Color, b: Color, weight: f32) -> Color {
    Color::from_rgb(
        a.r + (b.r - a.r) * weight,
        a.g + (b.g - a.g) * weight,
        a.b + (b.b - a.b) * weight,
    )
}

// Omarchy's generated palette is a flat list of quoted mode and #RRGGBB values.
fn parse(source: &str) -> Option<Colors> {
    let values: std::collections::HashMap<_, _> = source
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let value = value.trim();
            let quote = value.chars().next()?;
            if quote != '"' && quote != '\'' {
                return None;
            }
            Some((key.trim(), value[1..].split(quote).next()?))
        })
        .collect();
    let color = |key| {
        let hex = values.get(key)?.strip_prefix('#')?;
        if hex.len() != 6 {
            return None;
        }
        let n = u32::from_str_radix(hex, 16).ok()?;
        Some(Color::from_rgb8((n >> 16) as u8, (n >> 8) as u8, n as u8))
    };
    let background = color("background")?;
    let foreground = color("foreground")?;
    let accent = color("accent")?;
    let dark = values.get("mode").map_or(
        background.r * 0.2126 + background.g * 0.7152 + background.b * 0.0722 < 0.5,
        |mode| *mode == "dark",
    );
    let mut p = Colors::new(dark);
    p.background = color("dark_background").unwrap_or(background);
    p.header = p.background;
    p.paper = background;
    p.card = background;
    p.text = foreground;
    p.muted = color("light_foreground").unwrap_or_else(|| blend(background, foreground, 0.7));
    p.accent = accent;
    p.keypad = color("lighter_background").unwrap_or_else(|| blend(background, foreground, 0.09));
    p.operator = blend(background, accent, 0.2);
    p.rule = blend(background, foreground, 0.12);
    p.selected = color("selection").unwrap_or_else(|| blend(background, accent, 0.18));
    p.custom = p.keypad;
    p.negative = color("red").unwrap_or(p.negative);
    p.on_accent = if accent.r * 0.2126 + accent.g * 0.7152 + accent.b * 0.0722 > 0.55 {
        Color::from_rgb8(24, 24, 32)
    } else {
        Color::WHITE
    };
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_palette_and_rejects_incomplete_or_invalid_colors() {
        let source = "mode = \"dark\"\nbackground = \"#1e1e2e\"\nforeground = \"#cdd6f4\"\naccent = \"#89b4fa\"";
        let p = parse(source).unwrap();
        assert_eq!(p.accent, Color::from_rgb8(137, 180, 250));
        assert_eq!(p.paper, Color::from_rgb8(30, 30, 46));
        assert!(parse("background = \"#123\"").is_none());
        assert!(parse(&source.replace("#89b4fa", "invalid")).is_none());
    }
}
