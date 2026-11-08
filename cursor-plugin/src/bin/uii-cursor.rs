//! MCP server Cursor launches to read the current UII layout.
//!
//! stdout is reserved for MCP frames. Diagnostics go to stderr.

fn main() {
    if let Err(err) = uii_cursor::run_stdio() {
        eprintln!("uii-cursor: {err}");
        std::process::exit(1);
    }
}
