fn main() {
    if let Err(err) = uii::run() {
        eprintln!("UII: {err}");
        std::process::exit(1);
    }
}
