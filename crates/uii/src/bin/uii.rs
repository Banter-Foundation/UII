//! stdin/stdout call point. No editor window.
//!
//! `uii import <format>` reads that format and writes the document as JSON.
//! `uii export <format>` reads the document as JSON and writes that format.

use std::io::{self, Read, Write};
use std::process::ExitCode;

use uii::{call, Call, CallOutput, Document, Format};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let direction = args.next().unwrap_or_default();
    let format_name = args.next().unwrap_or_default();
    if direction != "import" && direction != "export" {
        eprintln!("usage: uii import|export <format>");
        return ExitCode::from(2);
    }
    let format = match Format::from_name(&format_name) {
        Ok(format) => format,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    };
    let mut input = String::new();
    if let Err(err) = io::stdin().read_to_string(&mut input) {
        eprintln!("{err}");
        return ExitCode::from(1);
    }
    let result = if direction == "import" {
        call(Call::Import {
            format,
            source: input,
        })
    } else {
        let document: Document = match serde_json::from_str(&input) {
            Ok(document) => document,
            Err(err) => {
                eprintln!("document json: {err}");
                return ExitCode::from(2);
            }
        };
        call(Call::Export { format, document })
    };
    match result {
        Ok(CallOutput::Document(document)) => match serde_json::to_string_pretty(&document) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("{err}");
                ExitCode::from(1)
            }
        },
        Ok(CallOutput::Source(source)) => {
            let mut stdout = io::stdout().lock();
            if writeln!(stdout, "{source}").is_err() {
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}
