fn prepare_demo() {
    if let Err(error) = crate::pluto_creative_demo::prepare() {
        eprintln!("Pluto Creative demo: {error}");
        std::process::exit(1);
    }
}
