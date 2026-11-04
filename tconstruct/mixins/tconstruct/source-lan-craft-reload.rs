fn tconstruct_lan_craft_reload(value: (), game: &mut crate::game::Game<'_>) -> () {
    fn invalidate(chunks: &mut horizon_client_render::chunk_map::ChunkMap) {
        let generation = chunks.__horizon_mixins_tconstruct_lan_craft_recipe_generation_mut();
        *generation = generation.wrapping_add(1).max(1);
        chunks.__horizon_mixins_tconstruct_lan_craft_menu_generation_mut().clear();
    }
    invalidate(&mut game.world.chunks);
    for dim in [crate::game::Dim::Overworld, crate::game::Dim::Nether, crate::game::Dim::End] {
        if let Some(world) = game.levels.parked_world_mut(dim) { invalidate(&mut world.chunks); }
    }
    value
}
