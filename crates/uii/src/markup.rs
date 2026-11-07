//! React, Vue, Svelte, and Angular are the HTML/CSS document wrapped in that
//! framework's component file. The tree is still parsed as HTML.

use crate::codec::type_name;
use crate::error::Error;
use crate::html;
use crate::model::Document;

pub(crate) fn export_react(document: &Document) -> Result<String, Error> {
    let html = html::export(document)?;
    let name = type_name(&document.name);
    Ok(format!(
        "import React from \"react\";\n\nexport function {name}() {{\n  return null;\n}}\n\nexport const uiiMarkup = `/* uii-markup-start */\n{html}/* uii-markup-end */`;\n"
    ))
}

pub(crate) fn export_vue(document: &Document) -> Result<String, Error> {
    let html = html::export(document)?;
    let name = type_name(&document.name);
    Ok(format!(
        "<template>\n{html}</template>\n<script>\nexport default {{ name: \"{name}\" }}\n</script>\n"
    ))
}

pub(crate) fn export_svelte(document: &Document) -> Result<String, Error> {
    let html = html::export(document)?;
    Ok(format!("<!-- uii-svelte -->\n{html}"))
}

pub(crate) fn export_angular(document: &Document) -> Result<String, Error> {
    let html = html::export(document)?;
    let name = type_name(&document.name);
    let selector = format!("app-{}", name.to_ascii_lowercase());
    Ok(format!(
        "import {{ Component }} from \"@angular/core\";\n\n@Component({{\n  selector: \"{selector}\",\n  template: `/* uii-markup-start */\n{html}/* uii-markup-end */`,\n  styles: [\"\"],\n}})\nexport class {name}Component {{}}\n"
    ))
}

pub(crate) fn import_react(source: &str) -> Result<Document, Error> {
    if !source.contains("export function") {
        return Err(Error::new("React file is missing `export function`"));
    }
    let html = between(source, "/* uii-markup-start */", "/* uii-markup-end */")?;
    html::import(html)
}

pub(crate) fn import_vue(source: &str) -> Result<Document, Error> {
    if !source.contains("<template") {
        return Err(Error::new("Vue file is missing `<template`"));
    }
    let html = between(source, "<template>", "</template>")
        .or_else(|_| between(source, "<template>\n", "</template>"))?;
    html::import(html)
}

pub(crate) fn import_svelte(source: &str) -> Result<Document, Error> {
    if !source.contains("uii-svelte") {
        return Err(Error::new("Svelte file is missing the uii-svelte marker"));
    }
    html::import(source)
}

pub(crate) fn import_angular(source: &str) -> Result<Document, Error> {
    if !source.contains("@Component") {
        return Err(Error::new("Angular file is missing `@Component`"));
    }
    let html = between(source, "/* uii-markup-start */", "/* uii-markup-end */")?;
    html::import(html)
}

fn between<'a>(source: &'a str, start: &str, end: &str) -> Result<&'a str, Error> {
    let Some(from) = source.find(start) else {
        return Err(Error::new(format!("missing `{start}`")));
    };
    let rest = &source[from + start.len()..];
    let Some(to) = rest.find(end) else {
        return Err(Error::new(format!("missing `{end}`")));
    };
    Ok(&rest[..to])
}
