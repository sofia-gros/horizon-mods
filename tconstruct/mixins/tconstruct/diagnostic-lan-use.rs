fn diagnostic_lan_use(game: &crate::game::Game<'_>) {
    if std::env::var_os("TCONSTRUCT_LAN_DIAGNOSTIC").is_some() {
        eprintln!("LAN_TRACE use target={:?} entity={} using={} destroying={}", game.pick().map(|h| h.pos), game.entity_targeted(), game.using.is_some(), game.remote.as_ref().is_some_and(|r| r.destroying.is_some()));
    }
}
