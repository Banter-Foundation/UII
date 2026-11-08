//! User-created assets and components. A new library is empty.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};

pub const LIBRARY_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ComponentId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    Folder,
    Asset,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRecord {
    pub id: AssetId,
    pub parent: Option<AssetId>,
    pub name: String,
    pub kind: AssetKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentRecord {
    pub id: ComponentId,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryError {
    EmptyName,
    InvalidName,
    DuplicateName,
    Missing,
    ParentNotFolder,
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptyName => "Enter a name.",
            Self::InvalidName => "Names cannot include slashes or line breaks.",
            Self::DuplicateName => "That name is already used here.",
            Self::Missing => "That item no longer exists.",
            Self::ParentNotFolder => "Create items inside a folder.",
        })
    }
}

impl std::error::Error for LibraryError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreError {
    Json,
    Version,
    Id,
    Cycle,
    Parent,
    DuplicateId,
    DuplicateName,
    Name,
    Value,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Json => "The file is not valid JSON.",
            Self::Version => "The file version is not supported.",
            Self::Id => "The file has an invalid id.",
            Self::Cycle => "The asset tree contains a cycle.",
            Self::Parent => "An asset points at a missing parent or a non-folder.",
            Self::DuplicateId => "The file repeats an id.",
            Self::DuplicateName => "The file repeats a name.",
            Self::Name => "The file has an empty or invalid name.",
            Self::Value => "The file has a value this version cannot store.",
        })
    }
}

impl std::error::Error for StoreError {}

#[derive(Clone, Debug)]
pub struct Library {
    assets: Vec<AssetRecord>,
    components: Vec<ComponentRecord>,
    next_asset: u64,
    next_component: u64,
}

impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}

impl Library {
    pub fn new() -> Self {
        Self {
            assets: Vec::new(),
            components: Vec::new(),
            next_asset: 1,
            next_component: 1,
        }
    }

    pub fn asset_count(&self) -> usize {
        self.assets.len()
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    pub fn assets(&self) -> &[AssetRecord] {
        &self.assets
    }

    pub fn components(&self) -> &[ComponentRecord] {
        &self.components
    }

    pub fn asset(&self, id: AssetId) -> Option<&AssetRecord> {
        self.assets.iter().find(|asset| asset.id == id)
    }

    pub fn component(&self, id: ComponentId) -> Option<&ComponentRecord> {
        self.components.iter().find(|component| component.id == id)
    }

    pub fn is_folder(&self, id: AssetId) -> bool {
        matches!(
            self.asset(id),
            Some(AssetRecord {
                kind: AssetKind::Folder,
                ..
            })
        )
    }

    pub fn children(&self, parent: Option<AssetId>) -> Vec<AssetId> {
        let mut ids: Vec<AssetId> = self
            .assets
            .iter()
            .filter(|asset| asset.parent == parent)
            .map(|asset| asset.id)
            .collect();
        ids.sort_by(|left, right| self.cmp_assets(*left, *right));
        ids
    }

    pub fn asset_visible(&self, id: AssetId, query: &str) -> bool {
        let Some(record) = self.asset(id) else {
            return false;
        };
        let query = query.trim();
        if query.is_empty() {
            return true;
        }
        if contains_ignore_case(&record.name, query) {
            return true;
        }
        if record.kind == AssetKind::Folder {
            return self
                .children(Some(id))
                .into_iter()
                .any(|child| self.asset_visible(child, query));
        }
        false
    }

    pub fn component_matches(&self, id: ComponentId, query: &str) -> bool {
        let Some(record) = self.component(id) else {
            return false;
        };
        let query = query.trim();
        query.is_empty() || contains_ignore_case(&record.name, query)
    }

    pub fn create_folder(
        &mut self,
        parent: Option<AssetId>,
        name: &str,
    ) -> Result<AssetId, LibraryError> {
        self.create_asset_kind(parent, name, AssetKind::Folder)
    }

    pub fn create_asset(
        &mut self,
        parent: Option<AssetId>,
        name: &str,
    ) -> Result<AssetId, LibraryError> {
        self.create_asset_kind(parent, name, AssetKind::Asset)
    }

    pub fn rename_asset(&mut self, id: AssetId, name: &str) -> Result<(), LibraryError> {
        let name = validate_name(name)?;
        if self.asset(id).is_none() {
            return Err(LibraryError::Missing);
        }
        self.ensure_unique_asset(&name, Some(id))?;
        let asset = self
            .assets
            .iter_mut()
            .find(|asset| asset.id == id)
            .ok_or(LibraryError::Missing)?;
        asset.name = name;
        Ok(())
    }

    pub fn delete_asset(&mut self, id: AssetId) -> Result<(), LibraryError> {
        if self.asset(id).is_none() {
            return Err(LibraryError::Missing);
        }
        let mut remove = HashSet::new();
        remove.insert(id);
        let mut grew = true;
        while grew {
            grew = false;
            for asset in &self.assets {
                if let Some(parent) = asset.parent {
                    if remove.contains(&parent) && remove.insert(asset.id) {
                        grew = true;
                    }
                }
            }
        }
        self.assets.retain(|asset| !remove.contains(&asset.id));
        Ok(())
    }

    pub fn create_component(&mut self, name: &str) -> Result<ComponentId, LibraryError> {
        let name = validate_name(name)?;
        self.ensure_unique_component(&name, None)?;
        let id = ComponentId(self.next_component);
        self.next_component += 1;
        self.components.push(ComponentRecord { id, name });
        Ok(id)
    }

    pub fn rename_component(&mut self, id: ComponentId, name: &str) -> Result<(), LibraryError> {
        let name = validate_name(name)?;
        if self.component(id).is_none() {
            return Err(LibraryError::Missing);
        }
        self.ensure_unique_component(&name, Some(id))?;
        let component = self
            .components
            .iter_mut()
            .find(|component| component.id == id)
            .ok_or(LibraryError::Missing)?;
        component.name = name;
        Ok(())
    }

    pub fn delete_component(&mut self, id: ComponentId) -> Result<(), LibraryError> {
        let before = self.components.len();
        self.components.retain(|component| component.id != id);
        if self.components.len() == before {
            Err(LibraryError::Missing)
        } else {
            Ok(())
        }
    }

    pub fn to_json(&self) -> Result<String, StoreError> {
        let file = LibraryFile {
            version: LIBRARY_VERSION,
            next_asset: self.next_asset,
            next_component: self.next_component,
            assets: self.assets.clone(),
            components: self.components.clone(),
        };
        serde_json::to_string_pretty(&file).map_err(|_| StoreError::Json)
    }

    pub fn from_json(text: &str) -> Result<Self, StoreError> {
        let file: LibraryFile = serde_json::from_str(text).map_err(|_| StoreError::Json)?;
        if file.version != LIBRARY_VERSION {
            return Err(StoreError::Version);
        }
        validate_library(&file)?;
        Ok(Self {
            assets: file.assets,
            components: file.components,
            next_asset: file.next_asset,
            next_component: file.next_component,
        })
    }

    fn create_asset_kind(
        &mut self,
        parent: Option<AssetId>,
        name: &str,
        kind: AssetKind,
    ) -> Result<AssetId, LibraryError> {
        let name = validate_name(name)?;
        self.ensure_parent_folder(parent)?;
        self.ensure_unique_asset(&name, None)?;
        let id = AssetId(self.next_asset);
        self.next_asset += 1;
        self.assets.push(AssetRecord {
            id,
            parent,
            name,
            kind,
        });
        Ok(id)
    }

    fn ensure_parent_folder(&self, parent: Option<AssetId>) -> Result<(), LibraryError> {
        let Some(parent) = parent else {
            return Ok(());
        };
        match self.asset(parent) {
            Some(asset) if asset.kind == AssetKind::Folder => Ok(()),
            Some(_) => Err(LibraryError::ParentNotFolder),
            None => Err(LibraryError::Missing),
        }
    }

    fn ensure_unique_asset(&self, name: &str, ignore: Option<AssetId>) -> Result<(), LibraryError> {
        if self
            .assets
            .iter()
            .any(|asset| ignore != Some(asset.id) && asset.name == name)
        {
            Err(LibraryError::DuplicateName)
        } else {
            Ok(())
        }
    }

    fn ensure_unique_component(
        &self,
        name: &str,
        ignore: Option<ComponentId>,
    ) -> Result<(), LibraryError> {
        if self
            .components
            .iter()
            .any(|component| ignore != Some(component.id) && component.name == name)
        {
            Err(LibraryError::DuplicateName)
        } else {
            Ok(())
        }
    }

    fn cmp_assets(&self, left: AssetId, right: AssetId) -> std::cmp::Ordering {
        let left = self.asset(left).expect("asset id");
        let right = self.asset(right).expect("asset id");
        folder_rank(left.kind)
            .cmp(&folder_rank(right.kind))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    }

    pub(crate) fn cmp_components(
        &self,
        left: ComponentId,
        right: ComponentId,
    ) -> std::cmp::Ordering {
        let left = self.component(left).expect("component id");
        let right = self.component(right).expect("component id");
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    }
}

fn folder_rank(kind: AssetKind) -> u8 {
    match kind {
        AssetKind::Folder => 0,
        AssetKind::Asset => 1,
    }
}

fn contains_ignore_case(name: &str, query: &str) -> bool {
    name.to_lowercase().contains(&query.to_lowercase())
}

pub(crate) fn validate_name(name: &str) -> Result<String, LibraryError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(LibraryError::EmptyName);
    }
    if trimmed
        .chars()
        .any(|ch| ch == '/' || ch == '\\' || ch.is_control())
    {
        return Err(LibraryError::InvalidName);
    }
    Ok(trimmed.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LibraryFile {
    version: u32,
    next_asset: u64,
    next_component: u64,
    assets: Vec<AssetRecord>,
    components: Vec<ComponentRecord>,
}

fn validate_library(file: &LibraryFile) -> Result<(), StoreError> {
    if file.next_asset == 0 || file.next_component == 0 {
        return Err(StoreError::Id);
    }
    let mut asset_ids = HashSet::new();
    let mut names = HashSet::new();
    let mut max_asset = 0u64;
    for asset in &file.assets {
        if asset.id.0 == 0 {
            return Err(StoreError::Id);
        }
        if !asset_ids.insert(asset.id) {
            return Err(StoreError::DuplicateId);
        }
        let name = validate_name(&asset.name).map_err(|_| StoreError::Name)?;
        if asset.name != name {
            return Err(StoreError::Name);
        }
        if !names.insert(name) {
            return Err(StoreError::DuplicateName);
        }
        max_asset = max_asset.max(asset.id.0);
    }
    if file.next_asset <= max_asset {
        return Err(StoreError::Id);
    }

    let by_id: HashMap<AssetId, &AssetRecord> =
        file.assets.iter().map(|asset| (asset.id, asset)).collect();
    for asset in &file.assets {
        if let Some(parent) = asset.parent {
            match by_id.get(&parent) {
                Some(parent_record) if parent_record.kind == AssetKind::Folder => {}
                _ => return Err(StoreError::Parent),
            }
        }
    }
    if has_cycle(&by_id) {
        return Err(StoreError::Cycle);
    }

    let mut component_ids = HashSet::new();
    let mut component_names = HashSet::new();
    let mut max_component = 0u64;
    for component in &file.components {
        if component.id.0 == 0 {
            return Err(StoreError::Id);
        }
        if !component_ids.insert(component.id) {
            return Err(StoreError::DuplicateId);
        }
        let name = validate_name(&component.name).map_err(|_| StoreError::Name)?;
        if component.name != name {
            return Err(StoreError::Name);
        }
        if !component_names.insert(name) {
            return Err(StoreError::DuplicateName);
        }
        max_component = max_component.max(component.id.0);
    }
    if file.next_component <= max_component {
        return Err(StoreError::Id);
    }
    Ok(())
}

fn has_cycle(by_id: &HashMap<AssetId, &AssetRecord>) -> bool {
    let mut state: HashMap<AssetId, u8> = HashMap::new();
    by_id.keys().any(|id| visit(*id, by_id, &mut state))
}

fn visit(
    id: AssetId,
    by_id: &HashMap<AssetId, &AssetRecord>,
    state: &mut HashMap<AssetId, u8>,
) -> bool {
    match state.get(&id).copied() {
        Some(1) => return true,
        Some(2) => return false,
        _ => {}
    }
    state.insert(id, 1);
    if let Some(parent) = by_id.get(&id).and_then(|record| record.parent) {
        if visit(parent, by_id, state) {
            return true;
        }
    }
    state.insert(id, 2);
    false
}
