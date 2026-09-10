//! Opt-in, read-only observation of the real rendered application for UI tests.
use super::*;
use iced::Rectangle;
use iced::advanced::widget::{Id, Operation, operation::Outcome};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Target {
    name: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}
#[derive(Default)]
struct Probe(Vec<Target>);
impl Probe {
    fn record(&mut self, name: &str, r: Rectangle) {
        self.0.push(Target {
            name: name.into(),
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
        });
    }
}
impl Operation<Vec<Target>> for Probe {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Vec<Target>>)) {
        operate(self);
    }
    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        for name in [
            "close-active-tab",
            "menu-toggle",
            "active-tab",
            "tape-surface",
            "copy-grand",
            "copy-memory",
        ] {
            if id == Some(&Id::from(name)) {
                self.record(name, bounds);
            }
        }
    }
    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
        self.record(text, bounds);
    }
    fn finish(&self) -> Outcome<Vec<Target>> {
        Outcome::Some(self.0.clone())
    }
}
pub fn probe() -> Task<Message> {
    let Some(dir) = std::env::var_os("NUMPAD_E2E_DIR").map(PathBuf::from) else {
        return Task::none();
    };
    let capture = if std::fs::remove_file(dir.join("capture")).is_ok() {
        window::latest()
            .and_then(window::screenshot)
            .map(Message::Screenshot)
    } else {
        Task::none()
    };
    Task::batch([
        iced::advanced::widget::operate(Probe::default()).map(Message::Probed),
        capture,
    ])
}
pub fn save_screenshot(shot: window::Screenshot) {
    if let Some(dir) = std::env::var_os("NUMPAD_E2E_DIR").map(PathBuf::from)
        && image::save_buffer(
            dir.join("screen.pending.png"),
            &shot.rgba,
            shot.size.width,
            shot.size.height,
            image::ColorType::Rgba8,
        )
        .is_ok()
    {
        let _ = std::fs::rename(dir.join("screen.pending.png"), dir.join("screen.png"));
    }
}
pub fn snapshot(app: &App, targets: Vec<Target>) {
    let Some(dir) = std::env::var_os("NUMPAD_E2E_DIR").map(PathBuf::from) else {
        return;
    };
    let value = serde_json::json!({
        "frame": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros().to_string(),
        "text": app.editor.text, "content": app.content.text(), "copied": format!("{:?}", app.copied.map(|c| c.0)), "tabs": app.tabs.len(), "active": app.active_tab,
        "zoom": app.prefs.zoom, "menu": app.menu, "modal": app.modal.is_some(),
        "ctrl": app.modifiers.control(),
        "theme": format!("{:?}", app.prefs.theme_mode), "scroll": app.scroll,
        "result": app.result().normalized().to_plain_string(),
        "grand": app.editor.tape.grand.normalized().to_plain_string(),
        "targets": targets
    });
    // Atomic publication: the driver never reads a partial frame.
    if std::fs::write(
        dir.join("state.tmp"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .is_ok()
    {
        let _ = std::fs::rename(dir.join("state.tmp"), dir.join("state.json"));
    }
}
