//! Folder library on disk. Assets and components are names in a tree.
//! The file is `library.json` under the UII data directory.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::library::{AssetId, AssetKind, Library, StoreError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataDirOs {
    Windows,
    Macos,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderRow {
    pub id: AssetId,
    pub depth: u32,
    pub kind: AssetKind,
    pub name: String,
}

/// `base` is `%LOCALAPPDATA%` on Windows and the user home on macOS and Linux.
pub fn resolve_data_dir(
    os: DataDirOs,
    override_dir: Option<&str>,
    xdg_data_home: Option<&str>,
    base: &Path,
) -> PathBuf {
    if let Some(dir) = nonempty(override_dir) {
        return PathBuf::from(dir);
    }
    match os {
        DataDirOs::Windows => base.join("UII"),
        DataDirOs::Macos => base.join("Library").join("Application Support").join("UII"),
        DataDirOs::Other => match nonempty(xdg_data_home) {
            Some(xdg) => PathBuf::from(xdg).join("UII"),
            None => base.join(".local").join("share").join("UII"),
        },
    }
}

pub fn default_data_dir() -> PathBuf {
    let override_dir = std::env::var("UII_DATA").ok();
    let xdg = std::env::var("XDG_DATA_HOME").ok();
    resolve_data_dir(
        native_os(),
        override_dir.as_deref(),
        xdg.as_deref(),
        &native_base(),
    )
}

pub fn library_file_path() -> PathBuf {
    default_data_dir().join("library.json")
}

pub fn folder_rows(library: &Library, query: &str, expanded: &BTreeSet<AssetId>) -> Vec<FolderRow> {
    let mut rows = Vec::new();
    walk(library, None, 0, query, expanded, &mut rows);
    rows
}

pub fn read_library(path: &Path) -> Result<Option<Library>, String> {
    match fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(Some(Library::new())),
        Ok(text) => Library::from_json(&text)
            .map(Some)
            .map_err(|err| format!("library.json: {err}")),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("library.json: {err}")),
    }
}

pub fn write_library(path: &Path, library: &Library) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|err| format!("library.json: {err}"))?;
        }
    }
    let text = library
        .to_json()
        .map_err(|err: StoreError| format!("library.json: {err}"))?;
    fs::write(path, text).map_err(|err| format!("library.json: {err}"))
}

fn walk(
    library: &Library,
    parent: Option<AssetId>,
    depth: u32,
    query: &str,
    expanded: &BTreeSet<AssetId>,
    rows: &mut Vec<FolderRow>,
) {
    let force_open = !query.trim().is_empty();
    for id in library.children(parent) {
        if !library.asset_visible(id, query) {
            continue;
        }
        let Some(record) = library.asset(id) else {
            continue;
        };
        rows.push(FolderRow {
            id,
            depth,
            kind: record.kind,
            name: record.name.clone(),
        });
        if record.kind == AssetKind::Folder && (force_open || expanded.contains(&id)) {
            walk(library, Some(id), depth + 1, query, expanded, rows);
        }
    }
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn native_os() -> DataDirOs {
    if cfg!(windows) {
        DataDirOs::Windows
    } else if cfg!(target_os = "macos") {
        DataDirOs::Macos
    } else {
        DataDirOs::Other
    }
}

fn native_base() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(local) = nonempty_env("LOCALAPPDATA") {
            return PathBuf::from(local);
        }
        if let Some(profile) = nonempty_env("USERPROFILE") {
            return PathBuf::from(profile).join("AppData").join("Local");
        }
        return PathBuf::from(".");
    }
    #[cfg(not(windows))]
    {
        if let Some(home) = nonempty_env("HOME") {
            return PathBuf::from(home);
        }
        PathBuf::from(".")
    }
}

fn nonempty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
