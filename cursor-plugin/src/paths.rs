//! Where the desktop app leaves the current layout.
//!
//! Banter stores its data under `%LOCALAPPDATA%\Banter` on Windows,
//! `~/Library/Application Support/Banter` on macOS, and `$XDG_DATA_HOME/Banter`
//! (or `~/.local/share/Banter`) on Linux. UII uses the same roots with `UII`
//! in place of `Banter`.

use std::path::{Path, PathBuf};

pub fn snapshot_path_from(
    platform: &str,
    home: &Path,
    local_app_data: Option<&Path>,
    xdg_data_home: Option<&Path>,
) -> PathBuf {
    match platform {
        "windows" => {
            let base = local_app_data
                .map(Path::to_path_buf)
                .unwrap_or_else(|| home.join("AppData/Local"));
            base.join("UII").join("layout.json")
        }
        "macos" => home.join("Library/Application Support/UII/layout.json"),
        _ => {
            let base = xdg_data_home
                .map(Path::to_path_buf)
                .unwrap_or_else(|| home.join(".local/share"));
            base.join("UII").join("layout.json")
        }
    }
}

pub fn default_snapshot_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    snapshot_path_from(
        std::env::consts::OS,
        &home,
        local.as_deref(),
        xdg.as_deref(),
    )
}
