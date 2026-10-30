fn diagnostic_lan_receive(host: &crate::game::lan::LanHost, conn: u32, packet: &horizon_protocol::packets::play::Serverbound) {
    if std::env::var_os("TCONSTRUCT_LAN_DIAGNOSTIC").is_some() {
        if let horizon_protocol::packets::play::Serverbound::UseItemOn(p) = packet {
            let guest = host.hub.guests.iter().find(|g| g.conn == conn);
            eprintln!("LAN_TRACE receive conn={} pos={:?} hand={} seq={} guest={:?}", conn, p.pos, p.hand, p.sequence, guest.map(|g| (g.position, g.dead, g.spectator())));
        }
    }
}
