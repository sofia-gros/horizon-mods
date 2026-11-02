fn tconstruct_crafting_lan_recipe_guard(
    game: &mut crate::game::Game<'_>,
    host: &mut LanHost,
    conn: u32,
    packet: &horizon_protocol::packets::recipes::PlaceRecipe,
) -> std::ops::ControlFlow<()> {
    use horizon_client::menu::place::PostPlace;
    use std::ops::ControlFlow::{Break, Continue};
    let Some(gi) = host.hub.guests.iter().position(|guest| guest.conn == conn) else { return Continue(()); };
    let joined = host.hub.guests[gi].placed_at;
    let tracked = game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_sessions()
        .get(&(conn, joined, packet.container)).copied();
    let current = host.play.get(&conn).and_then(|play| play.crafting.table.as_ref())
        .filter(|(window, _, _, _)| *window == packet.container).map(|(_, _, _, pos)| (pos[0], pos[1], pos[2]));
    let station = current.is_some_and(|pos| &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name == "tconstruct:crafting_station");
    if tracked.is_none() && !station { return Continue(()); }
    let Some(pos) = tracked.filter(|pos| Some(*pos) == current) else { game.lan_send_table_content(host, conn); return Break(()); };
    let guest = &host.hub.guests[gi];
    if guest.spectator() || !guest.within_block_range([pos.0, pos.1, pos.2], 4.0) || !station { game.lan_send_table_content(host, conn); return Break(()); }
    let Some(id) = world_stations!(game).recipe_index.recipe_of_display(packet.recipe).map(str::to_owned) else { return Break(()); };
    if !guest.recipe_book.knows(&id) || world_recipes!(game).placement(&id).is_none_or(|placement| placement.impossible()) { return Break(()); }
    let (player, name, creative) = (guest.uuid, guest.name.clone(), guest.creative());
    game.native_player_recipe_book_click(player, &name, &id, packet.use_max);
    let Some(horizon_client::containers::BlockEntity::Chest(grid)) = game.world.block_entities.get(&pos) else { return Break(()); };
    if grid.len() != 9 || &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name != "tconstruct:crafting_station" { return Break(()); }
    let grid = grid.clone();
    let Some((window, state, mut menu, saved_pos)) = host.play.get_mut(&conn).and_then(|play| play.crafting.table.take_if(|(window, ..)| *window == packet.container)) else { return Break(()); };
    if menu.slot_count() != 46 {
        host.play.entry(conn).or_default().crafting.table = Some((window, state, menu, saved_pos));
        game.lan_send_table_content(host, conn);
        return Break(());
    }
    let original_inventory = host.hub.guests[gi].inventory.clone();
    let original_carried = host.hub.guests[gi].carried.clone();
    let mut placed = game.guest_menu_op(host, gi, &mut menu, |menu, env| {
        for (index, item) in grid.iter().enumerate() { menu.set_slot(env, index + 1, item.clone()); }
        let player: Vec<_> = (0..36).map(|index| menu.player_item(index).cloned()).collect();
        let carried = menu.carried.clone();
        let before: Vec<_> = grid.iter().chain(&player).flatten().cloned().collect();
        let dropped = menu.dropped.len(); let drop_text = menu.drops.len();
        let outcome = menu.place_recipe(env, &id, packet.use_max, creative, None);
        let after_grid: Vec<_> = (1..10).map(|slot| menu.slot(slot).cloned()).collect();
        let after: Vec<_> = after_grid.iter().flatten().cloned().chain((0..36).filter_map(|index| menu.player_item(index).cloned())).collect();
        let count = |items: &[ItemStack], item: &ItemStack| -> u64 {
            items.iter().filter(|other| other.same_item_same_components(item)).map(|other| u64::from(other.count)).sum()
        };
        let valid = menu.dropped.len() == dropped && before.iter().all(|item| count(&before, item) == count(&after, item))
            && after.iter().all(|item| count(&before, item) == count(&after, item));
        if !valid {
            for (index, item) in grid.iter().enumerate() { menu.set_slot(env, index + 1, item.clone()); }
            for (index, item) in player.into_iter().enumerate() { menu.set_player_item(index, item); }
            menu.carried = carried; menu.dropped.truncate(dropped); menu.drops.truncate(drop_text);
            return None;
        }
        Some((outcome, after_grid))
    });
    if let Some((_, new_grid)) = placed.as_ref() {
        let valid_block = &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name == "tconstruct:crafting_station";
        if let Some(horizon_client::containers::BlockEntity::Chest(items)) = game.world.block_entities.get_mut(&pos).filter(|_| valid_block) {
            if items.len() == 9 {
                *items = new_grid.clone();
                game.world.modified.insert((pos.0 >> 4, pos.2 >> 4));
            } else { placed = None; }
        } else { placed = None; }
        if placed.is_none() {
            host.hub.guests[gi].inventory = original_inventory;
            host.hub.guests[gi].carried = original_carried;
            let env = game.menu_env();
            for index in 1..10 { menu.set_slot(&env, index, None); }
        }
    }
    host.play.entry(conn).or_default().crafting.table = Some((window, state, menu, saved_pos));
    game.lan_send_table_content(host, conn);
    if matches!(placed, Some((PostPlace::Ghost, _))) { host.hub.send_ghost_recipe(conn, packet.container, &id); }
    Break(())
}
