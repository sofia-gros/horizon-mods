// Inventory window zero has its own state counter. Keep the open window's
// counter separate so an inventory update cannot invalidate a menu click.
fn tconstruct_menu_window_state(game: &mut crate::game::Game<'_>, packet: &horizon_protocol::packets::play::Clientbound) {
    let update = match packet {
        C::OpenScreen(_) => Some(None),
        C::ContainerSetContent(p) if p.container != 0 && p.container == game.remote_menu_id() => Some(Some((p.container, p.state))),
        C::ContainerSetSlot(p) if p.container != 0 && p.container == game.remote_menu_id() => Some(Some((p.container, p.state))),
        _ => None,
    };
    if let Some(value) = update { *game.world.chunks.__horizon_mixins_tconstruct_menu_window_state_mut() = value; }
}
