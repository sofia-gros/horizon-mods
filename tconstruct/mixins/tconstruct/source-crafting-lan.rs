// Host-authoritative guest crafting sessions reuse the verified crafting menu.
use super::*;
use std::ops::ControlFlow::{self, Break, Continue};

type Pos = (i32, i32, i32);
fn station(game: &Game<'_>, pos: Pos) -> bool {
    &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name == "tconstruct:crafting_station"
}
fn items(game: &Game<'_>, pos: Pos) -> Option<Vec<Option<ItemStack>>> {
    let BlockEntity::Chest(items) = game.world.block_entities.get(&pos)? else { return None; };
    (items.len() == 9).then(|| items.clone())
}
fn session(game: &Game<'_>, conn: u32, joined: u64, window: i32) -> Option<Pos> {
    game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_sessions().get(&(conn, joined, window)).copied()
}

pub(crate) fn open(game: &mut Game<'_>, host: &mut LanHost, conn: u32, pos: Pos) -> bool {
    game.ensure_datapacks();
    game.world.block_entities.get_or_insert_with(pos, || BlockEntity::Chest(vec![None; 9]));
    let Some(grid) = items(game, pos) else { return false; };
    game.lan_crafting_closed(host, conn, None);
    game.lan_station_closed(host, conn, None);
    game.guest_container_closed(host, conn);
    let Some(gi) = host.hub.guests.iter().position(|guest| guest.conn == conn) else { return false; };
    let joined = host.hub.guests[gi].placed_at;
    let Some(window) = host.hub.open_window(conn, "minecraft:crafting", horizon_protocol::text::literal("TConstruct Crafting Station")) else { return false; };
    let mut menu = Box::new(Menu::new(Kind::Crafting));
    game.guest_menu_op(host, gi, &mut menu, |menu, env| {
        for (i, stack) in grid.into_iter().enumerate() { menu.set_slot(env, i + 1, stack); }
    });
    host.play.entry(conn).or_default().crafting.table = Some((window, 0, menu, [pos.0, pos.1, pos.2]));
    game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_sessions_mut().insert((conn, joined, window), pos);
    game.lan_send_table_content(host, conn);
    true
}

pub(crate) fn click(game: &mut Game<'_>, host: &mut LanHost, conn: u32, packet: &play::ContainerClick) -> ControlFlow<bool> {
    let Some(gi) = host.hub.guests.iter().position(|guest| guest.conn == conn) else { return Continue(()); };
    let joined = host.hub.guests[gi].placed_at;
    let Some(pos) = session(game, conn, joined, packet.container) else { return Continue(()); };
    let Some((window, state, mut menu, stored_pos)) = host.play.get_mut(&conn).and_then(|play| play.crafting.table.take_if(|(window, ..)| *window == packet.container)) else { return Break(true); };
    if station(game, pos) && !host.hub.guests[gi].spectator() && host.hub.guests[gi].within_block_range([pos.0, pos.1, pos.2], 4.0) {
        if let Some(grid) = items(game, pos) {
            game.guest_menu_op(host, gi, &mut menu, |menu, env| {
                for (i, stack) in grid.into_iter().enumerate() { menu.set_slot(env, i + 1, stack); }
                let _ = menu.click(env, i32::from(packet.slot), i32::from(packet.button), input(packet.input));
            });
            if let Some(BlockEntity::Chest(items)) = game.world.block_entities.get_mut(&pos) {
                if items.len() == 9 { for (i, slot) in items.iter_mut().enumerate() { *slot = menu.slot(i + 1).cloned(); } }
            }
            game.world.modified.insert((pos.0 >> 4, pos.2 >> 4));
        }
    }
    host.play.entry(conn).or_default().crafting.table = Some((window, state, menu, stored_pos));
    game.lan_send_table_content(host, conn);
    Break(true)
}

// Empty the temporary menu copy before vanilla close/leave returns its slots.
pub(crate) fn close(game: &mut Game<'_>, host: &mut LanHost, conn: u32, joined: u64, window: Option<i32>) {
    let Some((actual, _, menu, _)) = host.play.get_mut(&conn).and_then(|p| p.crafting.table.as_mut()) else { return; };
    if window.is_some_and(|window| window != *actual) { return; }
    let key = (conn, joined, *actual);
    let Some(pos) = game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_sessions_mut().remove(&key) else { return; };
    // The block is updated after each transaction. It may have been removed
    // since the last click; in either case the temporary copies are not drops.
    let env = game.menu_env();
    for i in 1..10 { menu.set_slot(&env, i, None); }
    let _ = pos;
}
