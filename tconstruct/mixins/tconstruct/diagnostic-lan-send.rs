fn diagnostic_lan_send(packet: &horizon_protocol::packets::play::Serverbound) {
    if std::env::var_os("TCONSTRUCT_LAN_DIAGNOSTIC").is_some() {
        if let horizon_protocol::packets::play::Serverbound::UseItemOn(p) = packet {
            eprintln!("LAN_TRACE send pos={:?} hand={} seq={}", p.pos, p.hand, p.sequence);
        }
    }
}
