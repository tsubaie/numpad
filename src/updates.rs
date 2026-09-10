//! Explicit, user-requested updates. System curl and the existing installer
//! avoid shipping another HTTP/TLS stack inside the application.
use serde::Deserialize;
use std::{
    fmt, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const RELEASE_API: &str = "https://api.github.com/repos/tsubaie/numpad/releases/latest";
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub [u64; 3]);
impl Version {
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.strip_prefix('v').unwrap_or(text);
        let parts: Vec<_> = text.split('.').collect();
        if text.len() > 64
            || parts.len() != 3
            || parts
                .iter()
                .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err("The release has an unsupported version number.".into());
        }
        let mut numbers = [0; 3];
        for (n, part) in numbers.iter_mut().zip(parts) {
            *n = part
                .parse()
                .map_err(|_| "The release version is too large.")?;
        }
        Ok(Self(numbers))
    }
}
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0[0], self.0[1], self.0[2])
    }
}
#[derive(Debug, Clone, Default)]
pub enum State {
    #[default]
    Idle,
    Checking,
    Current(Version),
    Available(Version),
    Installing(Version),
    Installed(Version),
    Failed(String),
}
#[derive(Debug, Clone)]
pub enum InstallOutcome {
    Installed,
    #[cfg(target_os = "windows")]
    RestartToInstall(PathBuf),
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}
fn release_version(bytes: &[u8]) -> Result<Version, String> {
    let release: Release = serde_json::from_slice(bytes)
        .map_err(|_| "GitHub returned an invalid release response.")?;
    if release.draft || release.prerelease {
        return Err("No stable release is available.".into());
    }
    Version::parse(&release.tag_name)
}
fn command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // No console for a version check.
    }
    command
}
pub fn check() -> Result<Version, String> {
    let output = command(if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    })
    .args([
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--connect-timeout",
        "10",
        "--max-time",
        "30",
        "--max-filesize",
        "1048576",
        "--user-agent",
        concat!("NumPad/", env!("CARGO_PKG_VERSION")),
        "--header",
        "Accept: application/vnd.github+json",
        RELEASE_API,
    ])
    .output()
    .map_err(|e| format!("Could not run curl to check for updates: {e}"))?;
    if !output.status.success() {
        return Err("Could not check GitHub. Check your connection and try again; GitHub may also be rate-limiting requests.".into());
    }
    release_version(&output.stdout)
}

fn temporary_directory() -> Result<PathBuf, String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let path = std::env::temp_dir().join(format!("numpad-update-{}-{nonce}", std::process::id()));
    #[allow(unused_mut)]
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&path)
        .map_err(|e| format!("Cannot create update directory: {e}"))?;
    Ok(path)
}
#[cfg(target_os = "linux")]
fn executable_exists(program: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|p| p.join(program).is_file()))
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn installer_kind() -> &'static str {
    if cfg!(target_os = "macos") {
        return "macos";
    }
    let exe = std::env::current_exe().unwrap_or_default();
    if directories::BaseDirs::new()
        .is_some_and(|dirs| exe == dirs.home_dir().join(".local/bin/numpad"))
    {
        "tarball"
    } else {
        "auto"
    }
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn launch_terminal(args: &[String]) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let shell_command = args
            .iter()
            .map(|s| shell_quote(s))
            .collect::<Vec<_>>()
            .join(" ");
        let status = Command::new("osascript").args(["-e","on run argv\ntell application \"Terminal\"\nactivate\ndo script (item 1 of argv)\nend tell\nend run",&shell_command]).status().map_err(|e| e.to_string())?;
        if status.success() {
            return Ok(());
        }
    }
    #[cfg(target_os = "linux")]
    for (terminal, prefix) in [
        ("xdg-terminal-exec", &[][..]),
        ("x-terminal-emulator", &["-e"][..]),
        ("kitty", &[][..]),
        ("alacritty", &["-e"][..]),
        ("foot", &[][..]),
        ("gnome-terminal", &["--wait", "--"][..]),
        ("konsole", &["--nofork", "-e"][..]),
        ("xterm", &["-e"][..]),
    ] {
        if executable_exists(terminal) {
            let child = Command::new(terminal)
                .args(prefix)
                .args(args)
                .spawn()
                .map_err(|e| format!("Could not open the update terminal: {e}"))?;
            // Reap the terminal process independently; some terminal clients
            // return immediately while others wait for the shell to finish.
            std::thread::spawn(move || {
                let _ = child.wait_with_output();
            });
            return Ok(());
        }
    }
    Err("Could not open a terminal. Use Release downloads to install the update manually.".into())
}
#[cfg(any(target_os = "macos", test))]
fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

pub fn install(version: Version) -> Result<InstallOutcome, String> {
    if version <= Version::parse(env!("CARGO_PKG_VERSION"))? {
        return Err("This release is not newer than the running version.".into());
    }
    let directory = temporary_directory()?;
    let result = install_in(version, &directory);
    // Keep failed/running helpers intact for diagnostics. Windows needs its
    // script and staged binary until after the application exits.
    if matches!(result, Ok(InstallOutcome::Installed)) {
        let _ = fs::remove_dir_all(&directory);
    }
    result
}
fn install_in(version: Version, directory: &Path) -> Result<InstallOutcome, String> {
    let status_path = directory.join("result");
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let installer = directory.join("install.sh");
        let wrapper = directory.join("run.sh");
        fs::write(&installer, include_str!("../install.sh")).map_err(|e| e.to_string())?;
        fs::write(&wrapper, include_str!("../scripts/update-unix.sh"))
            .map_err(|e| e.to_string())?;
        launch_terminal(&[
            "sh".into(),
            wrapper.to_string_lossy().into(),
            installer.to_string_lossy().into(),
            status_path.to_string_lossy().into(),
            version.to_string(),
            installer_kind().into(),
        ])?;
    }
    #[cfg(target_os = "windows")]
    {
        let helper = directory.join("update.ps1");
        fs::write(&helper, include_str!("../scripts/update-windows.ps1"))
            .map_err(|e| e.to_string())?;
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        // A visible helper reports any installation/rollback failure after
        // NumPad exits. It never kills the app or bypasses session saving.
        let child = Command::new("powershell.exe")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&helper)
            .arg("-Version")
            .arg(version.to_string())
            .arg("-AppPath")
            .arg(executable)
            .arg("-AppPid")
            .arg(std::process::id().to_string())
            .arg("-WorkDir")
            .arg(directory)
            .spawn()
            .map_err(|e| format!("Could not start the update helper: {e}"))?;
        std::thread::spawn(move || {
            let _ = child.wait_with_output();
        });
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return Err("Use Release downloads to update on this platform.".into());
    let start = Instant::now();
    loop {
        if let Ok(status) = fs::read_to_string(&status_path) {
            let status = status.trim_start_matches('\u{feff}').trim();
            return if status == "0" {
                Ok(InstallOutcome::Installed)
            } else {
                Err(format!(
                    "The update was not installed. Check the installer window for details ({status})."
                ))
            };
        }
        #[cfg(target_os = "windows")]
        if directory.join("ready").is_file() {
            return Ok(InstallOutcome::RestartToInstall(directory.to_path_buf()));
        }
        if start.elapsed() > Duration::from_secs(30)
            && !status_path.with_extension("started").exists()
        {
            #[cfg(target_os = "windows")]
            let cancel = directory.join("cancel");
            #[cfg(not(target_os = "windows"))]
            let cancel = status_path.with_extension("cancel");
            let _ = fs::write(cancel, b"startup timed out");
            return Err("The installer window did not start. Use Release downloads or check your terminal configuration.".into());
        }
        if start.elapsed() > Duration::from_secs(30 * 60) {
            return Err(
                "The installer has not finished. Check its terminal window before trying again."
                    .into(),
            );
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions_are_numeric_and_reject_shell_input() {
        assert!(Version::parse("v1.10.0").unwrap() > Version::parse("1.9.9").unwrap());
        for bad in [
            "1.2",
            "1.2.3;id",
            "1.2.3\n",
            "1.2.3-beta",
            "1..3",
            "1.2.18446744073709551616",
        ] {
            assert!(Version::parse(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn only_stable_valid_release_responses_are_accepted() {
        assert_eq!(
            release_version(br#"{"tag_name":"v1.3.2","draft":false,"prerelease":false}"#).unwrap(),
            Version([1, 3, 2])
        );
        assert!(
            release_version(br#"{"tag_name":"v1.3.2","draft":false,"prerelease":true}"#).is_err()
        );
        assert!(release_version(br#"{"message":"API rate limit exceeded"}"#).is_err());
    }
    #[test]
    fn shell_paths_are_quoted_as_literal_arguments() {
        assert_eq!(shell_quote("a'b $x"), "'a'\\''b $x'");
    }
}
