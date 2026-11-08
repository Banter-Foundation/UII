//! Layout snapshot Cursor reads. Fields are the document the native app publishes.

use std::collections::HashSet;
use std::fmt;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

/// Snapshot JSON this crate reads and writes.
pub const FORMAT_VERSION: u32 = 1;

/// Why a snapshot could not be built or read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    EmptyId,
    DuplicateId {
        id: String,
    },
    UnknownRegion {
        id: String,
    },
    UnsupportedFormat {
        found: u32,
    },
    Io(String),
    Json(String),
    NoLayout,
    /// The live plug did not accept a connection. Callers may fall back to the file.
    PlugUnreachable,
    Plug(String),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => f.write_str("region id is empty"),
            Self::DuplicateId { id } => write!(f, "duplicate region id: {id}"),
            Self::UnknownRegion { id } => write!(f, "unknown region: {id}"),
            Self::UnsupportedFormat { found } => write!(f, "unsupported layout format: {found}"),
            Self::Io(msg) => write!(f, "layout io: {msg}"),
            Self::Json(msg) => write!(f, "layout json: {msg}"),
            Self::NoLayout => f.write_str("no layout published"),
            Self::PlugUnreachable => f.write_str("plug is not reachable"),
            Self::Plug(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for LayoutError {}

impl From<std::io::Error> for LayoutError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<serde_json::Error> for LayoutError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err.to_string())
    }
}

/// Owl's description of the layout. HTML and CSS are the description, not a web UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiIntent {
    pub summary: String,
    pub html: String,
    pub css: String,
}

impl UiIntent {
    pub fn new(
        summary: impl Into<String>,
        html: impl Into<String>,
        css: impl Into<String>,
    ) -> Self {
        Self {
            summary: summary.into(),
            html: html.into(),
            css: css.into(),
        }
    }
}

/// One CSS declaration as Owl wrote it (`property: value`), not a computed web style.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Declaration {
    pub property: String,
    pub value: String,
}

/// Style declarations that apply to one region, in source order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Style {
    #[serde(default)]
    pub declarations: Vec<Declaration>,
}

/// Optional box on the canvas. Absent when the app has not placed the region yet.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// One node in the layout tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub id: String,
    pub name: String,
    /// What this region is for. Empty means the author has not said.
    pub purpose: String,
    #[serde(default)]
    pub style: Style,
    #[serde(default)]
    pub children: Vec<Region>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<Bounds>,
}

impl Region {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        purpose: impl Into<String>,
    ) -> Result<Self, LayoutError> {
        let id = id.into();
        if id.is_empty() {
            return Err(LayoutError::EmptyId);
        }
        Ok(Self {
            id,
            name: name.into(),
            purpose: purpose.into(),
            style: Style::default(),
            children: Vec::new(),
            bounds: None,
        })
    }

    pub fn with_declaration(
        mut self,
        property: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.style.declarations.push(Declaration {
            property: property.into(),
            value: value.into(),
        });
        self
    }

    pub fn with_child(mut self, child: Region) -> Self {
        self.children.push(child);
        self
    }

    pub fn with_bounds(mut self, bounds: Bounds) -> Self {
        self.bounds = Some(bounds);
        self
    }
}

/// The document Cursor reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutSnapshot {
    pub format: u32,
    pub document_name: String,
    pub intent: UiIntent,
    pub root: Region,
}

impl LayoutSnapshot {
    pub fn new(
        document_name: impl Into<String>,
        intent: UiIntent,
        root: Region,
    ) -> Result<Self, LayoutError> {
        let snap = Self {
            format: FORMAT_VERSION,
            document_name: document_name.into(),
            intent,
            root,
        };
        snap.validate()?;
        Ok(snap)
    }

    /// Format is checked before ids, so a wrong version is reported on its own.
    pub fn validate(&self) -> Result<(), LayoutError> {
        if self.format != FORMAT_VERSION {
            return Err(LayoutError::UnsupportedFormat { found: self.format });
        }
        let mut seen = HashSet::new();
        check_region(&self.root, &mut seen)
    }

    /// Preorder walk. An empty purpose is kept empty; the reading uses a placeholder.
    pub fn purposes(&self) -> Vec<RegionPurpose> {
        let mut out = Vec::new();
        walk_purposes(&self.root, &mut out);
        out
    }

    pub fn region_detail(&self, id: &str) -> Result<RegionDetail, LayoutError> {
        fn walk<'a>(
            region: &'a Region,
            ancestors: &[String],
            id: &str,
        ) -> Option<(&'a Region, Vec<String>)> {
            if region.id == id {
                return Some((region, ancestors.to_vec()));
            }
            let mut next = ancestors.to_vec();
            next.push(region.id.clone());
            for child in &region.children {
                if let Some(found) = walk(child, &next, id) {
                    return Some(found);
                }
            }
            None
        }

        let (region, ancestor_ids) = walk(&self.root, &[], id)
            .ok_or_else(|| LayoutError::UnknownRegion { id: id.to_string() })?;
        Ok(RegionDetail {
            id: region.id.clone(),
            name: region.name.clone(),
            purpose: region.purpose.clone(),
            style: region.style.clone(),
            bounds: region.bounds,
            child_ids: region
                .children
                .iter()
                .map(|child| child.id.clone())
                .collect(),
            ancestor_ids,
        })
    }

    /// Plain-language walk of structure, styles, and purpose. Derived only from this snapshot.
    pub fn reading(&self) -> String {
        let name = if self.document_name.is_empty() {
            "(untitled)"
        } else {
            self.document_name.as_str()
        };
        let intent = if self.intent.summary.is_empty() {
            "(not set)"
        } else {
            self.intent.summary.as_str()
        };
        let mut out = String::new();
        let _ = writeln!(out, "Document: {name}");
        let _ = writeln!(out, "Intent: {intent}");
        let _ = writeln!(out);
        write_region(&mut out, &self.root, 0);
        out
    }
}

fn check_region(region: &Region, seen: &mut HashSet<String>) -> Result<(), LayoutError> {
    if region.id.is_empty() {
        return Err(LayoutError::EmptyId);
    }
    if !seen.insert(region.id.clone()) {
        return Err(LayoutError::DuplicateId {
            id: region.id.clone(),
        });
    }
    for child in &region.children {
        check_region(child, seen)?;
    }
    Ok(())
}

fn walk_purposes(region: &Region, out: &mut Vec<RegionPurpose>) {
    out.push(RegionPurpose {
        id: region.id.clone(),
        name: region.name.clone(),
        purpose: region.purpose.clone(),
    });
    for child in &region.children {
        walk_purposes(child, out);
    }
}

fn write_region(out: &mut String, region: &Region, depth: usize) {
    let pad = "  ".repeat(depth);
    let purpose = if region.purpose.is_empty() {
        "purpose not set"
    } else {
        region.purpose.as_str()
    };
    let _ = writeln!(out, "{pad}- {} \"{}\" — {purpose}", region.id, region.name);
    let style_pad = "  ".repeat(depth + 1);
    if region.style.declarations.is_empty() {
        let _ = writeln!(out, "{style_pad}style: (none)");
    } else {
        let body = region
            .style
            .declarations
            .iter()
            .map(|decl| format!("{}: {}", decl.property, decl.value))
            .collect::<Vec<_>>()
            .join("; ");
        let _ = writeln!(out, "{style_pad}style: {body}");
    }
    for child in &region.children {
        write_region(out, child, depth + 1);
    }
}

/// One region in the purpose walk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionPurpose {
    pub id: String,
    pub name: String,
    pub purpose: String,
}

/// One region, its style, and where it sits in the tree.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegionDetail {
    pub id: String,
    pub name: String,
    pub purpose: String,
    pub style: Style,
    /// Null when the region has no box yet.
    pub bounds: Option<Bounds>,
    pub child_ids: Vec<String>,
    pub ancestor_ids: Vec<String>,
}
