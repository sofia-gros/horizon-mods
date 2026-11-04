fn tconstruct_lan_craft_left(game: &mut crate::game::Game<'_>, guest: &mut horizon_client::lan::guest::Guest) {
    let conn = guest.conn;
    game.world.chunks.__horizon_mixins_tconstruct_lan_craft_menu_generation_mut().retain(|(id, _, _), _| *id != conn);
}
