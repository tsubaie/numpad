use crate::appearance::Colors;
use iced::Color;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
struct Fingerprint {
    target: PathBuf,
    modified: std::time::SystemTime,
    len: u64,
}
pub struct ThemeWatcher {
    paths: Vec<PathBuf>,
    fingerprints: Option<Vec<Option<Fingerprint>>>,
    colors: Option<Colors>,
}
impl ThemeWatcher {
    pub fn new() -> Self {
        let paths = if cfg!(target_os = "linux") {
            directories::BaseDirs::new()
                .map(|dirs| {
                    vec![
                        dirs.state_dir()
                            .unwrap_or(dirs.data_local_dir())
                            .join("omarchy/current/theme/colors.toml"),
                        dirs.config_dir().join("omarchy/current/theme/colors.toml"),
                    ]
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        Self {
            paths,
            fingerprints: None,
            colors: None,
        }
    }
    pub fn poll(&mut self) -> Option<Colors> {
        let fingerprints: Vec<_> = self
            .paths
            .iter()
            .map(|path| {
                // Canonical identity catches theme symlink switches even when the
                // target files happen to share a timestamp and length.
                let target = std::fs::canonicalize(path).ok()?;
                let metadata = std::fs::metadata(&target).ok()?;
                Some(Fingerprint {
                    target,
                    modified: metadata.modified().ok()?,
                    len: metadata.len(),
                })
            })
            .collect();
        if self.fingerprints.as_ref() != Some(&fingerprints) {
            self.colors = self.paths.iter().find_map(|path| {
                std::fs::read_to_string(path)
                    .ok()
                    .and_then(|text| parse(&text))
            });
            self.fingerprints = Some(fingerprints);
        }
        self.colors
    }
}
pub fn omarchy_colors() -> Option<Colors> {
    ThemeWatcher::new().poll()
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
    fn watcher_detects_creation_changes_and_removal() {
        let dir = std::env::temp_dir().join(format!("numpad-theme-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("colors.toml");
        let _ = std::fs::remove_file(&path);
        let mut watcher = ThemeWatcher {
            paths: vec![path.clone()],
            fingerprints: None,
            colors: None,
        };
        assert!(watcher.poll().is_none());
        let source = "background = \"#1e1e2e\"\nforeground = \"#cdd6f4\"\naccent = \"#89b4fa\"";
        std::fs::write(&path, source).unwrap();
        let first = watcher.poll().unwrap();
        assert_eq!(watcher.poll(), Some(first));
        std::fs::write(&path, format!("{}\n", source.replace("#89b4fa", "#aabbcc"))).unwrap();
        assert_ne!(watcher.poll(), Some(first));
        std::fs::remove_file(&path).unwrap();
        assert!(watcher.poll().is_none());
        std::fs::remove_dir(&dir).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn watcher_detects_a_theme_symlink_switch() {
        use std::os::unix::fs::symlink;
        let dir =
            std::env::temp_dir().join(format!("numpad-theme-link-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a");
        let b = dir.join("b");
        let link = dir.join("current");
        let source = "background = \"#1e1e2e\"\nforeground = \"#cdd6f4\"\naccent = \"#89b4fa\"";
        std::fs::write(&a, source).unwrap();
        std::fs::write(&b, source.replace("#89b4fa", "#aabbcc")).unwrap();
        symlink(&a, &link).unwrap();
        let mut watcher = ThemeWatcher {
            paths: vec![link.clone()],
            fingerprints: None,
            colors: None,
        };
        let first = watcher.poll();
        std::fs::remove_file(&link).unwrap();
        symlink(&b, &link).unwrap();
        assert_ne!(watcher.poll(), first);
        std::fs::remove_dir_all(&dir).unwrap();
    }
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
