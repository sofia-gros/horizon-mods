fn tconstruct_lan_craft_close(value: bool, game: &mut crate::game::Game<'_>, conn: u32, window: Option<i32>) -> bool {
    if value && window != Some(0) {
        game.world.chunks.__horizon_mixins_tconstruct_lan_craft_menu_generation_mut().retain(|(id, _, _), _| *id != conn);
    }
    value
}
