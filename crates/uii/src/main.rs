//! UII window. This binary is the shell and the Design canvas.

mod animation;
mod content_dock;
mod function;
mod inspect;
mod platform;
mod view;

fn main() {
    if let Err(err) = platform::run() {
        eprintln!("uii: {err}");
        std::process::exit(1);
    }
}
