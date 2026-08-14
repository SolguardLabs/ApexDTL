fn main() {
    if let Err(error) = apex_dtl::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
