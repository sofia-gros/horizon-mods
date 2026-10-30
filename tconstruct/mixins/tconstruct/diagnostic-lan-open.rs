fn diagnostic_lan_open(conn: u32, pos: (i32, i32, i32)) {
    if std::env::var_os("TCONSTRUCT_LAN_DIAGNOSTIC").is_some() {
        eprintln!("LAN_TRACE server-open conn={} pos={:?}", conn, pos);
    }
}
