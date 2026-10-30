fn diagnostic_lan_response(conn: u32, menu_kind: &str, pos: (i32, i32, i32)) {
    if std::env::var_os("TCONSTRUCT_LAN_DIAGNOSTIC").is_some() {
        eprintln!("LAN_TRACE server-response conn={} pos={:?} menu={}", conn, pos, menu_kind);
    }
}
