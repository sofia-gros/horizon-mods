fn prepare_demo() {
    if let Err(error) = crate::pluto_transport_demo::prepare() {
        eprintln!("Pluto transport demo: {error}");
        std::process::exit(1);
    }
}
