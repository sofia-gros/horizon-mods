fn prepare_demo() {
    if let Err(error) = crate::entry::pluto_tool_rules_demo::prepare() {
        eprintln!("Pluto tool rules demo: {error}");
        std::process::exit(1);
    }
}
