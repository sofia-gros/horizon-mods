fn prepare_demo() {
    if let Err(error) = crate::entry::pluto_transport_demo::prepare() {
        eprintln!("Pluto transport demo: {error}");
        std::process::exit(1);
    }
}
