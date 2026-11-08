//! Publish the open design for the Cursor plugin. HTML and CSS are the
//! description of the layout. This does not serve a page.

use uii_core::{node_purpose, Editor, NodeId};
use uii_cursor::{Bounds, LayoutError, LayoutSnapshot, Region, UiIntent};

pub fn snapshot(editor: &Editor) -> Result<LayoutSnapshot, LayoutError> {
    let (html, css) = editor.markup();
    let name = editor.document().name.clone();
    let intent = UiIntent::new(format!("Layout {name}"), html, css);
    let mut root = Region::new("document", name.clone(), "Design canvas")?;
    for id in editor.document().roots() {
        root = root.with_child(region(editor, *id));
    }
    LayoutSnapshot::new(name, intent, root)
}

fn region(editor: &Editor, id: NodeId) -> Region {
    let node = editor.document().node(id);
    let mut built = Region::new(id.0.to_string(), node.name.clone(), node_purpose(node))
        .unwrap_or_else(|_| {
            Region::new(format!("n{}", id.0), node.name.clone(), node_purpose(node))
                .expect("node id")
        });
    built = built.with_bounds(Bounds {
        x: node.rect.x,
        y: node.rect.y,
        width: node.rect.width,
        height: node.rect.height,
    });
    built = built
        .with_declaration("left", format!("{}px", node.rect.x))
        .with_declaration("top", format!("{}px", node.rect.y))
        .with_declaration("width", format!("{}px", node.rect.width))
        .with_declaration("height", format!("{}px", node.rect.height));
    if node.fill.a > 0 {
        built = built.with_declaration("background-color", uii_core::format_color(node.fill));
    }
    if matches!(
        node.kind,
        uii_core::NodeKind::Text | uii_core::NodeKind::Button
    ) {
        built = built.with_declaration("font-size", format!("{}px", node.font_size));
    }
    if node.corner_radius > 0.0 {
        built = built.with_declaration("border-radius", format!("{}px", node.corner_radius));
    }
    let children = node.children.clone();
    for child in children {
        built = built.with_child(region(editor, child));
    }
    built
}
