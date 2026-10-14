//! Real X11 keyboard/mouse tests. Run with xvfb-run; no desktop session is touched.
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct App {
    child: Child,
    dir: PathBuf,
    window: String,
}
impl Drop for App {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn x(args: &[&str]) -> String {
    let out = Command::new("xdotool")
        .args(args)
        .output()
        .expect("install xdotool");
    assert!(
        out.status.success(),
        "xdotool {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().into()
}
impl App {
    fn launch(dir: PathBuf) -> Self {
        fs::create_dir_all(&dir).unwrap();
        let _ = fs::remove_file(dir.join("state.json"));
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("app.log"))
            .unwrap();
        // Exercise the real About controls without making network requests or
        // touching installed applications. Only this child receives the shim.
        let commands = dir.join("commands");
        fs::create_dir_all(&commands).unwrap();
        fs::write(commands.join("curl"), "#!/bin/sh\nprintf checked > \"$NUMPAD_E2E_DIR/update-requested\"\n[ ! -f \"$NUMPAD_E2E_DIR/update-offline\" ] || exit 22\ncat \"$NUMPAD_E2E_DIR/update-response.json\"\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(commands.join("curl"), fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::write(
            dir.join("update-response.json"),
            r#"{"tag_name":"v99.0.0","draft":false,"prerelease":false}"#,
        )
        .unwrap();
        let mut paths = vec![commands];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        let child = Command::new(env!("CARGO_BIN_EXE_numpad"))
            .env("PATH", std::env::join_paths(paths).unwrap())
            .env("NUMPAD_DATA_DIR", dir.join("data"))
            .env("NUMPAD_E2E_DIR", &dir)
            .env("XDG_CONFIG_HOME", dir.join("config"))
            .env("XDG_DATA_HOME", dir.join("xdg-data"))
            .env("XDG_STATE_HOME", dir.join("state"))
            .env("XDG_CACHE_HOME", dir.join("cache"))
            .env_remove("WAYLAND_DISPLAY")
            .env("WINIT_UNIX_BACKEND", "x11")
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let mut app = Self {
            child,
            dir,
            window: String::new(),
        };
        app.wait("startup", |s| {
            s["targets"].as_array().is_some_and(|a| !a.is_empty())
        });
        app.window = x(&[
            "search",
            "--pid",
            &app.child.id().to_string(),
            "--name",
            "NumPad",
        ])
        .lines()
        .last()
        .unwrap()
        .into();
        x(&["windowfocus", "--sync", &app.window]);
        app
    }
    fn state(&self) -> Value {
        fs::read(self.dir.join("state.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or(Value::Null)
    }
    fn wait(&mut self, label: &str, predicate: impl Fn(&Value) -> bool) -> Value {
        let start = Instant::now();
        loop {
            let s = self.state();
            if predicate(&s) {
                return s;
            }
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "app exited during {label}; see {}",
                self.dir.display()
            );
            assert!(
                start.elapsed() < Duration::from_secs(15),
                "timed out: {label}; state={s}; artifacts={}",
                self.dir.display()
            );
            thread::sleep(Duration::from_millis(50));
        }
    }
    fn key(&self, key: &str) {
        x(&["key", "--clearmodifiers", key]);
    }
    fn capture(&mut self, label: &str) {
        let path = self.dir.join("screen.png");
        let _ = fs::remove_file(&path);
        fs::write(self.dir.join("capture"), "").unwrap();
        self.wait("screenshot", |_| path.exists());
        fs::rename(path, self.dir.join(format!("{label}.png"))).unwrap();
    }
    fn capture_documentation(&mut self, label: &str) {
        // Read the displayed X11 window: the renderer's internal screenshot path
        // can omit cached glyphs when capturing consecutive partial redraws.
        for _ in 0..2 {
            let frame = self.state()["frame"].clone();
            self.wait("settled documentation frame", |s| s["frame"] != frame);
        }
        let status = Command::new("import")
            .args(["-window", &self.window])
            .arg(self.dir.join(format!("{label}.png")))
            .status()
            .expect("install ImageMagick for documentation captures");
        assert!(status.success(), "X11 window capture failed");
    }
    fn type_text(&self, text: &str) {
        x(&["type", "--clearmodifiers", "--delay", "12", "--", text]);
    }
    fn click(&mut self, name: &str) {
        self.click_target(name, true);
    }
    fn click_text(&mut self, line: usize, column: f64, count: usize) {
        self.settle();
        let s = self.state();
        let viewport = s["targets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == "tape-viewport")
            .unwrap();
        let zoom = s["zoom"].as_f64().unwrap();
        let px = viewport["x"].as_f64().unwrap() + 12. + column * 9.6 * zoom;
        let py = viewport["y"].as_f64().unwrap() + 12. + (line as f64 + 0.5) * 28. * zoom;
        x(&[
            "mousemove",
            "--window",
            &self.window,
            &format!("{px:.0}"),
            &format!("{py:.0}"),
        ]);
        // Give each press a real duration so the native event loop observes
        // distinct clicks even while software rendering a frame.
        for _ in 0..count {
            x(&[
                "mousedown",
                "1",
                "sleep",
                "0.04",
                "mouseup",
                "1",
                "sleep",
                "0.12",
            ]);
        }
    }
    fn settle(&mut self) {
        // A queued probe may still describe the previous widget tree. Require
        // two subsequent observations before using geometry or sending input.
        for _ in 0..2 {
            let frame = self.state()["frame"].clone();
            self.wait("settled UI frame", |s| s["frame"] != frame);
        }
    }
    fn click_target(&mut self, name: &str, stays_open: bool) {
        self.settle();
        let s = self.wait(name, |s| {
            s["targets"]
                .as_array()
                .is_some_and(|a| a.iter().any(|t| t["name"] == name))
        });
        let t = s["targets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap();
        let px = t["x"].as_f64().unwrap() + t["width"].as_f64().unwrap() / 2.;
        let py = t["y"].as_f64().unwrap() + t["height"].as_f64().unwrap() / 2.;
        x(&[
            "mousemove",
            "--window",
            &self.window,
            &format!("{px:.0}"),
            &format!("{py:.0}"),
        ]);
        self.settle();
        x(&["mousedown", "1"]);
        self.settle();
        x(&["mouseup", "1"]);
        if stays_open {
            self.settle();
        }
    }
    fn wait_exit(&mut self) {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "last tab must exit cleanly");
                return;
            }
            assert!(
                start.elapsed() < Duration::from_secs(15),
                "last tab did not close the app"
            );
            thread::sleep(Duration::from_millis(50));
        }
    }
    fn replace(&self, text: &str) {
        self.key("ctrl+a");
        self.type_text(text);
    }
}

fn main() {
    assert!(
        std::env::var_os("DISPLAY").is_some(),
        "Run under xvfb-run -a cargo test --features e2e --test e2e"
    );
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let dir = std::env::current_dir()
        .unwrap()
        .join(format!("test-results/e2e/{stamp}-{}", std::process::id()));
    println!("UI test artifacts: {}", dir.display());
    if std::env::var_os("NUMPAD_DOC_SCREENSHOTS").is_some() {
        documentation_screenshots(dir);
        return;
    }
    let mut app = App::launch(dir.clone());
    let s = app.state();
    let target = |name: &str| {
        s["targets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap()
    };
    let tab = target("active-tab");
    let tape = target("tape-surface");
    assert!(
        (tab["y"].as_f64().unwrap() + tab["height"].as_f64().unwrap()
            - tape["y"].as_f64().unwrap())
        .abs()
            < 1.,
        "tab must meet tape without a gap"
    );
    assert!(
        tab["x"].as_f64().unwrap() + tab["width"].as_f64().unwrap()
            <= tape["x"].as_f64().unwrap() + tape["width"].as_f64().unwrap()
    );
    println!("PASS connected tab geometry");
    app.capture("connected-tabs");
    let screen = image::open(app.dir.join("connected-tabs.png"))
        .unwrap()
        .into_rgba8();
    let px = (tab["x"].as_f64().unwrap() + tab["width"].as_f64().unwrap() / 2.) as u32;
    let seam = tape["y"].as_f64().unwrap().round() as u32;
    assert_eq!(
        screen.get_pixel(px, seam - 2),
        screen.get_pixel(px, seam + 2),
        "active tab and paper must share a seamless background"
    );

    app.key("ctrl+t");
    app.wait("new tab", |s| s["tabs"] == 2);
    app.type_text("alpha bravo charlie");
    app.click_text(0, 8.5, 2);
    app.wait("double click selects a word", |s| s["selection"] == "bravo");
    app.key("ctrl+c");
    app.type_text("delta");
    app.wait("typing replaces the selected word", |s| {
        s["text"] == "alpha delta charlie"
    });
    app.key("ctrl+End");
    app.key("ctrl+v");
    app.wait("word selection copied to clipboard", |s| {
        s["text"] == "alpha delta charliebravo"
    });
    app.click_text(0, 8.5, 3);
    app.wait("triple click selects the line", |s| {
        s["selection"] == "alpha delta charliebravo"
    });
    app.key("BackSpace");
    app.wait("delete selected line", |s| s["text"] == "");
    app.type_text("1+2");
    app.key("Return");
    app.key("ctrl+Home");
    app.key("Down");
    app.key("Down");
    app.key("Down");
    app.key("End");
    let at_total = app.wait("caret at subtotal end", |s| {
        s["caret"]["line"] == 3
            && s["text"]
                .as_str()
                .and_then(|t| t.lines().nth(3))
                .is_some_and(|line| s["caret"]["column"].as_u64() == Some(line.len() as u64))
    });
    let before_space = at_total["caret"]["column"].as_u64().unwrap();
    app.type_text("   ");
    app.wait("subtotal spaces immediately move the caret", |s| {
        s["caret"]["column"].as_u64() == Some(before_space + 3)
            && s["text"]
                .as_str()
                .unwrap_or("")
                .lines()
                .nth(3)
                .is_some_and(|l| l.ends_with("   "))
    });
    app.type_text("subtotal note");
    app.key("Return");
    app.key("Return");
    app.type_text("Try selecting these words");
    app.wait("comments ready for capture", |s| {
        s["text"]
            .as_str()
            .is_some_and(|t| t.contains("subtotal note") && t.contains("Try selecting these words"))
    });
    app.capture_documentation("comments-and-subtotal");
    println!("PASS word/line selection, copy/replace/delete, and immediate subtotal spacing");
    app.replace("x = 10+2*3");
    app.wait("typing", |s| {
        s["text"].as_str().is_some_and(|t| t.contains("x = 10+2*3"))
    });
    app.key("ctrl+z");
    app.wait("undo", |s| {
        !s["text"].as_str().unwrap_or("").contains("x = 10+2*3")
    });
    app.key("ctrl+shift+z");
    app.wait("redo", |s| {
        s["text"].as_str().unwrap_or("").contains("x = 10+2*3")
    });
    app.replace("40+2");
    app.key("Return");
    app.wait("calculation", |s| s["grand"] == "42");
    app.capture("calculation");
    app.click("copy-grand");
    app.wait("copy completion", |s| s["copied"] == "Some(Grand)");
    app.click("tape-surface");
    app.key("ctrl+a");
    app.key("ctrl+v");
    app.wait("real clipboard paste", |s| {
        s["text"].as_str().unwrap_or("").trim() == "42.00"
    });
    println!("PASS keyboard editing, undo/redo and clipboard");

    app.click("menu-toggle");
    app.wait("menu open", |s| s["menu"] == true);
    app.click("tape-surface");
    app.wait("outside click", |s| s["menu"] == false);
    app.click("menu-toggle");
    app.click("Settings");
    app.wait("settings", |s| s["modal"] == true);
    app.click("About");
    app.wait("about content", |s| {
        s["targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "Created by tsubaie ↗")
    });
    assert!(
        !dir.join("update-requested").exists(),
        "No automatic update request"
    );
    app.click("Check for updates");
    app.wait("new release available", |s| {
        s["targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "Install update")
    });
    app.capture_documentation("about-updates");
    fs::write(
        dir.join("update-response.json"),
        format!(
            r#"{{"tag_name":"v{}","draft":false,"prerelease":false}}"#,
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();
    app.click("Check for updates");
    app.wait("already current", |s| {
        s["targets"].as_array().unwrap().iter().any(|t| {
            t["name"]
                .as_str()
                .unwrap_or("")
                .starts_with("You're up to date.")
        })
    });
    fs::write(dir.join("update-offline"), "").unwrap();
    app.click("Check for updates");
    app.wait("offline update feedback", |s| {
        s["targets"].as_array().unwrap().iter().any(|t| {
            t["name"]
                .as_str()
                .unwrap_or("")
                .starts_with("Could not check GitHub.")
        })
    });
    println!("PASS on-demand update checks, version comparison, and offline feedback");
    app.click("Cancel");
    app.wait("cancel", |s| s["modal"] == false);
    println!("PASS menu dismissal and settings navigation");

    app.click("menu-toggle");
    app.click("Settings");
    app.click("Dark");
    app.click("Cancel");
    app.wait("theme cancel", |s| {
        s["modal"] == false && s["theme"] == "System"
    });
    app.click("menu-toggle");
    app.click("Settings");
    app.click("Dark");
    app.click("Save changes");
    app.wait("theme save", |s| {
        s["modal"] == false && s["theme"] == "Dark"
    });
    app.capture("dark-tabs");
    let original = app.state()["text"].as_str().unwrap().to_owned();
    app.click("tape-surface");
    let long_note = "Long comment stays inside the tape. ".repeat(5);
    app.replace(&long_note);
    app.wait("long note", |s| s["text"] == long_note);
    for (width, height) in [(700, 580), (464, 1112), (420, 580), (900, 1000)] {
        x(&[
            "windowsize",
            &app.window,
            &width.to_string(),
            &height.to_string(),
        ]);
        app.wait("window resize delivered", |s| {
            s["window"]["width"].as_f64() == Some(width as f64)
                && s["window"]["height"].as_f64() == Some(height as f64)
        });
        app.settle();
        let state = app.state();
        let target = |name: &str| {
            state["targets"]
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t["name"] == name)
                .unwrap()
        };
        let tape = target("tape-surface");
        let controls = target("calculator-controls");
        let viewport = target("tape-viewport");
        if width < 860 {
            assert!(
                (tape["width"].as_f64().unwrap() - (width as f64 - 32.)).abs() < 1.,
                "narrow tape must use the full body width"
            );
            assert!(
                controls["y"].as_f64().unwrap()
                    >= tape["y"].as_f64().unwrap() + tape["height"].as_f64().unwrap(),
                "controls must sit below the tape"
            );
        } else {
            assert!(
                controls["x"].as_f64().unwrap()
                    >= tape["x"].as_f64().unwrap() + tape["width"].as_f64().unwrap(),
                "wide controls must sit beside the tape"
            );
        }
        assert!(
            viewport["height"].as_f64().unwrap() >= 100.,
            "tape must remain usable"
        );
        let label = format!("layout-{width}-{height}");
        app.capture_documentation(&label);
        let screen = image::open(app.dir.join(format!("{label}.png")))
            .unwrap()
            .into_rgb8();
        let mut green_pixels = 0;
        for (x, y, pixel) in screen.enumerate_pixels() {
            let [r, g, b] = pixel.0;
            if g > 150 && g > r.saturating_add(30) && g > b.saturating_add(30) {
                green_pixels += 1;
                assert!(
                    x as f64 >= viewport["x"].as_f64().unwrap()
                        && (x as f64)
                            < viewport["x"].as_f64().unwrap() + viewport["width"].as_f64().unwrap()
                        && y as f64 >= viewport["y"].as_f64().unwrap()
                        && (y as f64)
                            < viewport["y"].as_f64().unwrap()
                                + viewport["height"].as_f64().unwrap(),
                    "comment escaped tape at {x},{y} in {label}"
                );
            }
        }
        assert!(green_pixels > 100, "comment must be visibly rendered");
    }
    app.click("tape-surface");
    app.replace(&original);
    app.wait("restore tape after layout check", |s| s["text"] == original);
    x(&["windowsize", &app.window, "1180", "820"]);
    app.wait("restore layout", |s| {
        s["targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "tape-surface" && t["width"].as_f64().unwrap() > 700.)
    });
    println!("PASS themes, narrow/tall/short layouts, and rendered text clipping");

    app.click("tape-surface");
    app.key("ctrl+plus");
    app.wait("zoom in", |s| s["zoom"].as_f64().unwrap_or(0.) > 1.);
    app.key("ctrl+0");
    app.wait("reset zoom", |s| s["zoom"] == 1.);
    app.click("tape-surface");
    x(&["keydown", "ctrl"]);
    app.wait("control held", |s| s["ctrl"] == true);
    x(&["click", "4"]);
    app.wait("mouse wheel zoom", |s| {
        s["zoom"].as_f64().unwrap_or(0.) > 1.
    });
    x(&["keyup", "ctrl"]);
    app.wait("control released", |s| s["ctrl"] == false);
    app.key("ctrl+0");
    app.wait("wheel zoom reset", |s| s["zoom"] == 1.);
    app.replace("A long tape note");
    // xdotool type does not translate embedded newlines into Return events.
    x(&["key", "--repeat", "65", "--delay", "12", "Return"]);
    app.key("ctrl+End");
    app.wait("long tape scroll", |s| {
        s["scroll"].as_f64().unwrap_or(0.) > 500.
    });
    app.capture("long-tape");
    app.replace("42.00");
    app.wait("restore test text", |s| s["text"] == "42.00");
    app.key("ctrl+Tab");
    app.wait("switch tape", |s| s["active"] == 0);
    app.key("ctrl+Tab");
    app.wait("restore tape", |s| {
        s["active"] == 1 && s["text"].as_str().unwrap_or("").trim() == "42.00"
    });
    println!("PASS zoom and independent tabs");

    // Wait for the real autosave, then simulate an ungraceful process exit.
    app.wait("persisted workspace", |_| {
        fs::read_to_string(dir.join("data/workspace.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .is_some_and(|s| {
                s["active"] == 1
                    && s["tabs"][1]["document"]["text"]
                        .as_str()
                        .unwrap_or("")
                        .trim()
                        == "42.00"
            })
    });
    drop(app);
    let mut app = App::launch(dir.clone());
    app.wait("crash recovery", |s| {
        s["tabs"] == 2 && s["active"] == 1 && s["text"].as_str().unwrap_or("").trim() == "42.00"
    });
    println!("PASS crash recovery");
    app.key("ctrl+w");
    app.wait("dirty close prompt", |s| s["modal"] == true);
    app.click("Cancel");
    app.wait("close canceled", |s| s["modal"] == false && s["tabs"] == 2);
    app.key("ctrl+w");
    app.click("Discard");
    app.wait("one tab left", |s| s["tabs"] == 1);
    app.click_target("close-active-tab", false);
    app.wait_exit();
    drop(app);
    let mut app = App::launch(dir);
    app.wait("fresh tape after last close", |s| {
        s["tabs"] == 1 && s["text"] == ""
    });
    app.type_text("7");
    app.wait("unsaved last tape", |s| s["text"] == "7");
    app.key("ctrl+w");
    app.wait("last unsaved prompt", |s| s["modal"] == true);
    app.click("Cancel");
    app.wait("last close canceled", |s| {
        s["modal"] == false && s["text"] == "7"
    });
    app.key("ctrl+w");
    app.click_target("Discard", false);
    app.wait_exit();
    println!("PASS last-tab exit and unsaved-change protection; all UI scenarios passed");
}

// Reproducible screenshots of real widgets with fictional, checked-in data.
// Capture only: this does not replace the regression suite above.
fn documentation_screenshots(dir: PathBuf) {
    fs::create_dir_all(dir.join("data")).unwrap();
    fs::write(
        dir.join("data/workspace.json"),
        include_str!("../docs/demo-workspace.json"),
    )
    .unwrap();
    let mut app = App::launch(dir);
    // Resize once so font shaping and the final layout have settled.
    x(&["windowsize", &app.window, "1180", "840"]);
    app.wait("demo layout", |s| {
        s["targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "tape-surface" && t["height"].as_f64().unwrap() > 660.)
    });
    app.click("tape-surface");
    app.key("ctrl+Home");
    x(&["key", "--repeat", "13", "--delay", "12", "Down"]);
    app.wait("demo total", |s| {
        s["grand"] == "2875" && s["result"] == "2875"
    });
    app.capture_documentation("numpad-dark");
    app.click("menu-toggle");
    app.click("Settings");
    app.capture_documentation("numpad-settings");
    app.click("About");
    app.wait("about content", |s| {
        s["targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "Created by @tsubaie")
    });
    app.capture_documentation("numpad-about");
    app.click("Appearance");
    app.click("Light");
    app.click("Save changes");
    app.wait("light theme", |s| {
        s["theme"] == "Light" && s["modal"] == false
    });
    app.capture_documentation("numpad-light");
    app.key("F1");
    app.wait("guide", |s| s["modal"] == true);
    app.capture_documentation("numpad-guide");
    println!(
        "Documentation screenshots captured in {}",
        app.dir.display()
    );
}
