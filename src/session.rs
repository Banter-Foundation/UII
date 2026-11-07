//! Drawer session: browser state, row queries, and the files under the data directory.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::drawer::{DrawerCommand, DrawerState};
use crate::library::{AssetId, AssetKind, AssetRecord, ComponentId, Library, StoreError};

pub const DOCK_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionError {
    Library(StoreError),
    Dock(StoreError),
    Io(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Library(err) => write!(f, "library.json: {err}"),
            Self::Dock(err) => write!(f, "dock.json: {err}"),
            Self::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for SessionError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataDirOs {
    Windows,
    Macos,
    Other,
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

pub fn data_dir() -> PathBuf {
    let override_dir = std::env::var("UII_DATA").ok();
    let xdg = std::env::var("XDG_DATA_HOME").ok();
    resolve_data_dir(
        native_os(),
        override_dir.as_deref(),
        xdg.as_deref(),
        &native_base(),
    )
}

pub fn library_path(data_dir: &Path) -> PathBuf {
    data_dir.join("library.json")
}

pub fn dock_path(data_dir: &Path) -> PathBuf {
    data_dir.join("dock.json")
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BrowserState {
    pub asset_query: String,
    pub component_query: String,
    pub selected_folder: Option<AssetId>,
    pub selected_asset: Option<AssetId>,
    pub selected_component: Option<ComponentId>,
    pub expanded: BTreeSet<AssetId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetRow {
    pub id: AssetId,
    pub depth: u32,
    pub kind: AssetKind,
}

pub struct DockHost {
    pub drawer: DrawerState,
    pub browser: BrowserState,
    pub library: Library,
    pub status: String,
    asset_name: String,
    component_name: String,
    shortcut: crate::drawer::ShortcutEdge,
    resizing: bool,
    dirty: bool,
    data_dir: PathBuf,
}

impl DockHost {
    pub fn new(library: Library) -> Self {
        Self::from_parts(
            library,
            DrawerState::default(),
            BrowserState::default(),
            PathBuf::new(),
        )
    }

    pub(crate) fn from_parts(
        library: Library,
        drawer: DrawerState,
        browser: BrowserState,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            drawer,
            browser,
            library,
            status: String::new(),
            asset_name: String::new(),
            component_name: String::new(),
            shortcut: crate::drawer::ShortcutEdge::default(),
            resizing: false,
            dirty: false,
            data_dir,
        }
    }

    pub(crate) fn asset_name_mut(&mut self) -> &mut String {
        &mut self.asset_name
    }

    pub(crate) fn component_name_mut(&mut self) -> &mut String {
        &mut self.component_name
    }

    pub(crate) fn asset_name(&self) -> &str {
        &self.asset_name
    }

    pub(crate) fn component_name(&self) -> &str {
        &self.component_name
    }

    pub(crate) fn clear_asset_name(&mut self) {
        self.asset_name.clear();
    }

    pub(crate) fn clear_component_name(&mut self) {
        self.component_name.clear();
    }

    pub(crate) fn shortcut_mut(&mut self) -> &mut crate::drawer::ShortcutEdge {
        &mut self.shortcut
    }

    pub(crate) fn resizing(&self) -> bool {
        self.resizing
    }

    pub(crate) fn set_resizing(&mut self, resizing: bool) {
        self.resizing = resizing;
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(crate) fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub(crate) fn persist(&mut self) {
        if !self.dirty || self.data_dir.as_os_str().is_empty() {
            return;
        }
        match save_session(&self.data_dir, self) {
            Ok(()) => self.dirty = false,
            Err(err) => self.status = err.to_string(),
        }
    }
}

pub fn asset_rows(library: &Library, query: &str, expanded: &BTreeSet<AssetId>) -> Vec<AssetRow> {
    let mut rows = Vec::new();
    walk_assets(library, None, 0, query, expanded, &mut rows);
    rows
}

pub fn folder_contents(library: &Library, folder: Option<AssetId>, query: &str) -> Vec<AssetId> {
    library
        .children(folder)
        .into_iter()
        .filter(|id| library.asset_visible(*id, query))
        .collect()
}

pub fn component_rows(library: &Library, query: &str) -> Vec<ComponentId> {
    let mut ids: Vec<ComponentId> = library
        .components()
        .iter()
        .map(|component| component.id)
        .filter(|id| library.component_matches(*id, query))
        .collect();
    ids.sort_by(|left, right| library.cmp_components(*left, *right));
    ids
}

pub fn insertion_folder(library: &Library, browser: &BrowserState) -> Option<AssetId> {
    if let Some(asset_id) = browser.selected_asset {
        if let Some(asset) = library.asset(asset_id) {
            if asset.kind == AssetKind::Asset {
                return asset.parent;
            }
        }
    }
    if let Some(folder) = browser.selected_folder {
        if library.is_folder(folder) {
            return Some(folder);
        }
    }
    None
}

pub fn expand_for_new_folder(
    expanded: &mut BTreeSet<AssetId>,
    parent: Option<AssetId>,
    created: AssetId,
) {
    if let Some(parent) = parent {
        expanded.insert(parent);
    }
    expanded.insert(created);
}

pub fn prune_browser(browser: &mut BrowserState, library: &Library) {
    browser.expanded.retain(|id| library.is_folder(*id));
    if browser
        .selected_folder
        .is_some_and(|id| !library.is_folder(id))
    {
        browser.selected_folder = None;
    }
    if browser.selected_asset.is_some_and(|id| {
        !matches!(
            library.asset(id),
            Some(AssetRecord {
                kind: AssetKind::Asset,
                ..
            })
        )
    }) {
        browser.selected_asset = None;
    }
    if browser
        .selected_component
        .is_some_and(|id| library.component(id).is_none())
    {
        browser.selected_component = None;
    }
}

pub fn open_session(
    library_json: Option<&str>,
    dock_json: Option<&str>,
) -> Result<DockHost, SessionError> {
    let library = match library_json {
        None => Library::new(),
        Some(text) => Library::from_json(text).map_err(SessionError::Library)?,
    };
    let (drawer, mut browser) = match dock_json {
        None => (DrawerState::default(), BrowserState::default()),
        Some(text) => parse_dock(text).map_err(SessionError::Dock)?,
    };
    prune_browser(&mut browser, &library);
    Ok(DockHost::from_parts(
        library,
        drawer,
        browser,
        PathBuf::new(),
    ))
}

pub fn load_session(data_dir: &Path) -> Result<DockHost, SessionError> {
    let library_json = read_optional(&library_path(data_dir))?;
    let dock_json = read_optional(&dock_path(data_dir))?;
    let mut host = open_session(library_json.as_deref(), dock_json.as_deref())?;
    host.data_dir = data_dir.to_path_buf();
    Ok(host)
}

pub fn save_session(data_dir: &Path, host: &DockHost) -> Result<(), SessionError> {
    fs::create_dir_all(data_dir).map_err(|err| SessionError::Io(err.to_string()))?;
    let library = host.library.to_json().map_err(SessionError::Library)?;
    let dock = dock_json(host).map_err(SessionError::Dock)?;
    fs::write(library_path(data_dir), library).map_err(|err| SessionError::Io(err.to_string()))?;
    fs::write(dock_path(data_dir), dock).map_err(|err| SessionError::Io(err.to_string()))?;
    Ok(())
}

fn walk_assets(
    library: &Library,
    parent: Option<AssetId>,
    depth: u32,
    query: &str,
    expanded: &BTreeSet<AssetId>,
    rows: &mut Vec<AssetRow>,
) {
    let force_open = !query.trim().is_empty();
    for id in library.children(parent) {
        if !library.asset_visible(id, query) {
            continue;
        }
        let Some(record) = library.asset(id) else {
            continue;
        };
        rows.push(AssetRow {
            id,
            depth,
            kind: record.kind,
        });
        if record.kind == AssetKind::Folder && (force_open || expanded.contains(&id)) {
            walk_assets(library, Some(id), depth + 1, query, expanded, rows);
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

fn read_optional(path: &Path) -> Result<Option<String>, SessionError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(SessionError::Io(err.to_string())),
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DockFile {
    version: u32,
    open: bool,
    pinned: bool,
    body_height: f32,
    assets_fraction: f32,
    asset_query: String,
    component_query: String,
    selected_folder: Option<u64>,
    selected_asset: Option<u64>,
    selected_component: Option<u64>,
    expanded: Vec<u64>,
}

fn parse_dock(text: &str) -> Result<(DrawerState, BrowserState), StoreError> {
    let file: DockFile = serde_json::from_str(text).map_err(|_| StoreError::Json)?;
    if file.version != DOCK_VERSION {
        return Err(StoreError::Version);
    }
    if !file.body_height.is_finite() || !file.assets_fraction.is_finite() {
        return Err(StoreError::Value);
    }
    let mut drawer = DrawerState::default();
    drawer.apply(DrawerCommand::SetOpen(file.open));
    drawer.apply(DrawerCommand::SetPinned(file.pinned));
    drawer.apply(DrawerCommand::SetBodyHeight(file.body_height));
    drawer.apply(DrawerCommand::SetAssetsFraction(file.assets_fraction));
    let browser = BrowserState {
        asset_query: file.asset_query,
        component_query: file.component_query,
        selected_folder: file.selected_folder.map(AssetId),
        selected_asset: file.selected_asset.map(AssetId),
        selected_component: file.selected_component.map(ComponentId),
        expanded: file.expanded.into_iter().map(AssetId).collect(),
    };
    Ok((drawer, browser))
}

fn dock_json(host: &DockHost) -> Result<String, StoreError> {
    let file = DockFile {
        version: DOCK_VERSION,
        open: host.drawer.open,
        pinned: host.drawer.pinned,
        body_height: host.drawer.body_height,
        assets_fraction: host.drawer.assets_fraction,
        asset_query: host.browser.asset_query.clone(),
        component_query: host.browser.component_query.clone(),
        selected_folder: host.browser.selected_folder.map(|id| id.0),
        selected_asset: host.browser.selected_asset.map(|id| id.0),
        selected_component: host.browser.selected_component.map(|id| id.0),
        expanded: host.browser.expanded.iter().map(|id| id.0).collect(),
    };
    serde_json::to_string_pretty(&file).map_err(|_| StoreError::Json)
}
