fn tconstruct_lan_craft_open_return(value: (), game: &mut crate::game::Game<'_>, host: &mut LanHost, conn: u32, pos: [i32; 3]) -> () {
    let Some(guest) = host.hub.guests.iter().find(|guest| guest.conn == conn) else { return value; };
    let Some((window, _, menu, table_pos)) = host.play.get(&conn).and_then(|p| p.crafting.table.as_ref()) else { return value; };
    if *table_pos != pos || (0..10).any(|slot| menu.slot(slot).is_some()) { return value; }
    let key = (conn, guest.placed_at, *window);
    let generation = *game.world.chunks.__horizon_mixins_tconstruct_lan_craft_recipe_generation();
    game.world.chunks.__horizon_mixins_tconstruct_lan_craft_menu_generation_mut().insert(key, generation);
    value
}
