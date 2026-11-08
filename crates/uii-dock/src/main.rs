fn main() {
    if let Err(err) = uii_dock::run() {
        eprintln!("UII: {err}");
        std::process::exit(1);
    }
}
