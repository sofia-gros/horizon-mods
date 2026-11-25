fn prepare_demo() {
    if let Err(error) = crate::pluto_tool_rules_demo::prepare() {
        eprintln!("Pluto tool rules demo: {error}");
        std::process::exit(1);
    }
}
