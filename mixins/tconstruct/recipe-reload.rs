pub(super) fn capture(game: &mut crate::game::Game<'_>) {
    if game.remote.is_some() || game.world_root.as_os_str().is_empty() { return; }
    let builtin = horizon_client::datapack::DataPack::builtin(&game.options.resources);
    let mut recipes = std::collections::BTreeMap::new();
    if !game.cmds.vanilla_disabled {
        crate::game::tconstruct_recipes::merge_pack(&mut recipes, builtin.__horizon_mixins_tconstruct_recipe_files(&["recipes"]), |name| builtin.__horizon_mixins_tconstruct_recipe_read(name));
    }
    for id in &game.fns.enabled {
        if let Some(pack) = game.fns.packs.iter().find(|pack| &pack.id == id) {
            crate::game::tconstruct_recipes::merge_pack(&mut recipes, pack.__horizon_mixins_tconstruct_recipe_files(&["recipes"]), |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
        }
    }
    *game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot_mut() = Some(recipes);
}
