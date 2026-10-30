fn diagnostic_lan_screen(p: &horizon_protocol::packets::play::OpenScreen) {
    if std::env::var_os("TCONSTRUCT_LAN_DIAGNOSTIC").is_some() {
        eprintln!("LAN_TRACE client-screen window={} menu={}", p.container, p.menu);
    }
}
