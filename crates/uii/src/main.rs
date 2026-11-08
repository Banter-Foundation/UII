//! UII native editor. The window is the app. `mcp`, `get`, and `apply`
//! talk to that window over a local socket so Cursor can read and change
//! the layout. They are not a web server. `import` and `export` convert a
//! document on stdin and stdout and do not open a window.

mod bridge;
mod platform;
mod publish;
mod view;

use std::env;
use std::io::{self, Read, Write};

use uii::{call, Call, CallOutput, Document, Format};

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("mcp") => bridge::mcp(),
        Some("get") => print_response(&bridge::ask("{\"op\":\"get\"}\n")),
        Some("apply") => {
            let mut body = String::new();
            io::stdin()
                .read_to_string(&mut body)
                .expect("read layout json");
            print_response(&bridge::ask(&body));
        }
        Some("import") => std::process::exit(import_export(true, args.next().as_deref())),
        Some("export") => std::process::exit(import_export(false, args.next().as_deref())),
        Some("--help") | Some("-h") | Some("help") => print_help(),
        Some(other) => {
            eprintln!("unknown command {other}");
            print_help();
            std::process::exit(2);
        }
        None => {
            if let Err(err) = platform::run() {
                eprintln!("uii: {err}");
                std::process::exit(1);
            }
        }
    }
}

fn import_export(importing: bool, format_name: Option<&str>) -> i32 {
    let Some(format_name) = format_name else {
        eprintln!("usage: uii import|export <format>");
        return 2;
    };
    let format = match Format::from_name(format_name) {
        Ok(format) => format,
        Err(err) => {
            eprintln!("{err}");
            return 2;
        }
    };
    let mut input = String::new();
    if let Err(err) = io::stdin().read_to_string(&mut input) {
        eprintln!("{err}");
        return 1;
    }
    let result = if importing {
        call(Call::Import {
            format,
            source: input,
        })
    } else {
        let document: Document = match serde_json::from_str(&input) {
            Ok(document) => document,
            Err(err) => {
                eprintln!("document json: {err}");
                return 2;
            }
        };
        call(Call::Export { format, document })
    };
    match result {
        Ok(CallOutput::Document(document)) => match serde_json::to_string_pretty(&document) {
            Ok(json) => {
                println!("{json}");
                0
            }
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Ok(CallOutput::Source(source)) => {
            let mut stdout = io::stdout().lock();
            if writeln!(stdout, "{source}").is_err() {
                return 1;
            }
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn print_response(response: &str) {
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{response}");
}

fn print_help() {
    println!(
        "UII is a native layout editor.\n\n\
         uii                 open the Design and Animation window\n\
         uii get             print the current layout as JSON\n\
         uii apply           read one JSON object from stdin and apply it\n\
         uii mcp             Cursor MCP server for the running window\n\
         uii import <format>  read that format from stdin and write document JSON\n\
         uii export <format>  read document JSON from stdin and write that format\n\n\
         get and apply talk to 127.0.0.1 on the port written by the open window.\n\
         That socket is not a web server. Start uii first.\n\
         import and export do not open a window."
    );
}
