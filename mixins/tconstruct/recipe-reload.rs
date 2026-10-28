pub(super) fn capture(game: &mut crate::game::Game<'_>) {
    if game.remote.is_some() || game.world_root.as_os_str().is_empty() { return; }
    let builtin = horizon_client::datapack::DataPack::builtin(&game.options.resources);
    let mut recipes = std::collections::BTreeMap::new();
    let mut tags = std::collections::BTreeMap::new();
    if !game.cmds.vanilla_disabled {
        let files = builtin.__horizon_mixins_tconstruct_recipe_files(&["recipes", "tags"]);
        crate::game::tconstruct_recipes::merge_pack(&mut recipes, files.iter().cloned(), |name| builtin.__horizon_mixins_tconstruct_recipe_read(name));
        crate::game::tconstruct_source_tags::merge_pack_tags(&mut tags, files, |name| builtin.__horizon_mixins_tconstruct_recipe_read(name));
        let vanilla_1201 = horizon_client::datapack::DataPack::builtin(&game.options.resources.join("minecraft-original-pack"));
        let files = vanilla_1201.__horizon_mixins_tconstruct_recipe_files(&["recipes", "tags"]);
        crate::game::tconstruct_recipes::merge_pack(&mut recipes, files.iter().cloned(), |name| vanilla_1201.__horizon_mixins_tconstruct_recipe_read(name));
        crate::game::tconstruct_source_tags::merge_pack_tags(&mut tags, files, |name| vanilla_1201.__horizon_mixins_tconstruct_recipe_read(name));
    }
    let forge = horizon_client::datapack::DataPack::builtin(&game.options.resources.join("forge-original-pack"));
    let files = forge.__horizon_mixins_tconstruct_recipe_files(&["recipes", "tags"]);
    crate::game::tconstruct_recipes::merge_pack(&mut recipes, files.iter().cloned(), |name| forge.__horizon_mixins_tconstruct_recipe_read(name));
    crate::game::tconstruct_source_tags::merge_pack_tags(&mut tags, files, |name| forge.__horizon_mixins_tconstruct_recipe_read(name));
    let original = horizon_client::datapack::DataPack::builtin(&game.options.resources.join("tconstruct-original-pack"));
    let files = original.__horizon_mixins_tconstruct_recipe_files(&["recipes", "tags"]);
    crate::game::tconstruct_recipes::merge_pack(&mut recipes, files.iter().cloned(), |name| original.__horizon_mixins_tconstruct_recipe_read(name));
    crate::game::tconstruct_source_tags::merge_pack_tags(&mut tags, files, |name| original.__horizon_mixins_tconstruct_recipe_read(name));
    for id in &game.fns.enabled {
        if let Some(pack) = game.fns.packs.iter().find(|pack| &pack.id == id) {
            let files = pack.__horizon_mixins_tconstruct_recipe_files(&["recipes", "tags"]);
            crate::game::tconstruct_recipes::merge_pack(&mut recipes, files.iter().cloned(), |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
            crate::game::tconstruct_source_tags::merge_pack_tags(&mut tags, files, |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
        }
    }
    *game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot_mut() = Some(recipes);
    *game.world.chunks.__horizon_mixins_tconstruct_recipe_tag_snapshot_mut() = Some(tags);
}
