//! Import and export a UI document.
//!
//! The host application stays native Rust on Banter imgui (`dear-imgui-rs`
//! 0.22, GLFW, OpenGL, `ConfigFlags::DOCKING_ENABLE` before the first frame).
//! This crate does not open a window and does not draw an editor. Banter calls
//! [`call`]. The `uii` binary is that same call point on stdin/stdout.
//!
//! HTML regions become docks where the target has docking, tables become
//! tables where the target has them, and nested boxes become child windows
//! where the target has them. Raven calls follow the public C API in
//! [oscar-soirey/Raven](https://github.com/oscar-soirey/Raven). Banter's tree
//! has no Raven symbol; those calls are not invented names.

mod codec;
mod emit;
mod error;
mod html;
mod markup;
mod model;
mod validate;

pub use error::Error;
pub use model::{Axis, DockSide, Document, Format, Node, NodeKind};

/// Read `source` as `format` and return the document.
pub fn import(format: Format, source: &str) -> Result<Document, Error> {
    if source.trim().is_empty() {
        return Err(Error::new("empty document"));
    }
    let document = match format {
        Format::HtmlCss => html::import(source)?,
        Format::React => markup::import_react(source)?,
        Format::Vue => markup::import_vue(source)?,
        Format::Svelte => markup::import_svelte(source)?,
        Format::Angular => markup::import_angular(source)?,
        Format::BanterImgui
        | Format::RavenImgui
        | Format::DearImgui
        | Format::Egui
        | Format::Slint
        | Format::Qt
        | Format::WinUi
        | Format::SwiftUi
        | Format::Gtk
        | Format::JetpackCompose => codec::document_from_records(source)?,
    };
    validate::check(format, &document, source)?;
    Ok(document)
}

/// Write `document` as `format`.
pub fn export(format: Format, document: &Document) -> Result<String, Error> {
    match format {
        Format::HtmlCss => html::export(document),
        Format::React => markup::export_react(document),
        Format::Vue => markup::export_vue(document),
        Format::Svelte => markup::export_svelte(document),
        Format::Angular => markup::export_angular(document),
        Format::BanterImgui => emit::export_banter(document),
        Format::RavenImgui => emit::export_raven(document),
        Format::DearImgui => emit::export_dear(document),
        Format::Egui => emit::export_egui(document),
        Format::Slint => emit::export_slint(document),
        Format::Qt => emit::export_qt(document),
        Format::WinUi => emit::export_winui(document),
        Format::SwiftUi => emit::export_swift(document),
        Format::Gtk => emit::export_gtk(document),
        Format::JetpackCompose => emit::export_compose(document),
    }
}

/// What the host asks the call point to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Call {
    Import { format: Format, source: String },
    Export { format: Format, document: Document },
}

/// Text for an export, or a document for an import.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallOutput {
    Document(Document),
    Source(String),
}

/// The only entry the Banter imgui host needs. It does not create a GLFW window.
pub fn call(request: Call) -> Result<CallOutput, Error> {
    match request {
        Call::Import { format, source } => Ok(CallOutput::Document(import(format, &source)?)),
        Call::Export { format, document } => Ok(CallOutput::Source(export(format, &document)?)),
    }
}
