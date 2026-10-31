fn smeltery_track_state(game: &mut crate::game::Game<'_>, packet: &horizon_protocol::packets::play::Clientbound) {
    use horizon_protocol::packets::play::Clientbound as C;
    if !crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) { return; }
    let Some(pos) = crate::game::tconstruct_source_smeltery_ui::position(game) else { return; };
    let Some((window, _)) = game.remote.as_ref().and_then(|remote| remote.menu.as_ref()) else { return; };
    let (container, state) = match packet {
        C::ContainerSetContent(p) => (p.container, p.state),
        C::ContainerSetSlot(p) => (p.container, p.state),
        _ => return,
    };
    if container == *window {
        game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_menu_state_mut().insert(pos, (container, state));
    }
}
