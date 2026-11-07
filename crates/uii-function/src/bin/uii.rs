fn main() {
    if let Err(err) = uii_function::run() {
        eprintln!("uii: {err}");
        std::process::exit(1);
    }
}
