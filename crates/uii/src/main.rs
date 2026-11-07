//! UII native editor. The window is the app. `mcp`, `get`, and `apply`
//! talk to that window over a local socket so Cursor can read and change
//! the layout. They are not a web server.

mod bridge;
mod platform;
mod view;

use std::env;
use std::io::{self, Read, Write};

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("mcp") => bridge::mcp(),
        Some("get") => print_response(&bridge::ask("{\"op\":\"get\"}\n")),
        Some("apply") => {
            let mut body = String::new();
            io::stdin().read_to_string(&mut body).expect("read layout json");
            print_response(&bridge::ask(&body));
        }
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
         uii mcp             Cursor MCP server for the running window\n\n\
         get and apply talk to 127.0.0.1 on the port written by the open window.\n\
         That socket is not a web server. Start uii first."
    );
}
