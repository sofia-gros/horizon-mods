// A table menu persists across /reload. Refresh its cached result before the
// first subsequent click, and reject that click if its prediction is stale.
use super::*;
use std::ops::ControlFlow::{self, Break, Continue};

pub(crate) fn before_click(game: &mut Game<'_>, host: &mut LanHost, conn: u32, packet: &play::ContainerClick) -> ControlFlow<bool> {
    let Some(gi) = host.hub.guests.iter().position(|guest| guest.conn == conn) else { return Continue(()); };
    let guest = &host.hub.guests[gi];
    if guest.spectator() { return Continue(()); }
    let joined = guest.placed_at;
    let Some((window, _, menu, pos)) = host.play.get(&conn).and_then(|p| p.crafting.table.as_ref()) else { return Continue(()); };
    if *window != packet.container || menu.kind() != Kind::Crafting || !game.lan_table_valid(guest, *pos) { return Continue(()); }
    // The mod's Crafting Station has its own authoritative grid reload and
    // callback; do not change its interaction or depend on injection order.
    if &*game.res.facts().get(game.world.block(pos[0], pos[1], pos[2])).name != "minecraft:crafting_table" { return Continue(()); }
    let key = (conn, joined, *window);
    let generation = *game.world.chunks.__horizon_mixins_tconstruct_lan_craft_recipe_generation();
    if game.world.chunks.__horizon_mixins_tconstruct_lan_craft_menu_generation().get(&key).copied().unwrap_or(0) == generation { return Continue(()); }
    let Some((window, state, mut menu, pos)) = host.play.get_mut(&conn).and_then(|p| p.crafting.table.take()) else { return Continue(()); };
    let previous = menu.slot(0).cloned();
    let first = menu.slot(1).cloned();
    let env = game.menu_env();
    menu.set_slot(&env, 1, first);
    let mut result = menu.slot(0).cloned();
    let guest = &host.hub.guests[gi];
    game.native_prepare_item_craft(guest.uuid, &guest.name, Some(pos), &mut result);
    let env = game.menu_env();
    menu.set_slot(&env, 0, result.clone());
    if result != previous && menu.quick_crafting() {
        let env = game.menu_env();
        // A non-QuickCraft click while dragging only resets the host's drag state.
        let _ = menu.click(&env, -1, 0, Input::Pickup);
    }
    host.play.entry(conn).or_default().crafting.table = Some((window, state, menu, pos));
    game.world.chunks.__horizon_mixins_tconstruct_lan_craft_menu_generation_mut().insert(key, generation);
    if result != previous {
        game.lan_send_table_content(host, conn);
        return Break(true);
    }
    Continue(())
}
