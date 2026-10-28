pub(super) fn capture(game: &mut crate::game::Game<'_>) {
    if game.remote.is_some() || game.world_root.as_os_str().is_empty() { return; }
    let mut recipes = std::collections::BTreeMap::new();
    let mut tags = std::collections::BTreeMap::new();
    let mut materials = std::collections::BTreeMap::new();
    let mut material_tags = std::collections::BTreeMap::new();
    fn merge(
        pack: &horizon_client::datapack::DataPack,
        recipes: &mut std::collections::BTreeMap<String, Vec<u8>>,
        tags: &mut std::collections::BTreeMap<String, Vec<Vec<u8>>>,
        materials: &mut std::collections::BTreeMap<String, Vec<Vec<u8>>>,
        material_tags: &mut std::collections::BTreeMap<String, Vec<Vec<u8>>>,
    ) {
        let files = pack.__horizon_mixins_tconstruct_recipe_files(&["recipes", "tags", "tinkering"]);
        crate::game::tconstruct_recipes::merge_pack(recipes, files.iter().cloned(), |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
        crate::game::tconstruct_source_tags::merge_pack_tags(tags, files.iter().cloned(), |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
        crate::game::tconstruct_materials::merge_pack(materials, material_tags, files, |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
    }
    if !game.cmds.vanilla_disabled {
        let builtin = horizon_client::datapack::DataPack::builtin(&game.options.resources);
        merge(&builtin, &mut recipes, &mut tags, &mut materials, &mut material_tags);
        let vanilla = horizon_client::datapack::DataPack::builtin(&game.options.resources.join("minecraft-original-pack"));
        merge(&vanilla, &mut recipes, &mut tags, &mut materials, &mut material_tags);
    }
    let forge = horizon_client::datapack::DataPack::builtin(&game.options.resources.join("forge-original-pack"));
    merge(&forge, &mut recipes, &mut tags, &mut materials, &mut material_tags);
    let original = horizon_client::datapack::DataPack::builtin(&game.options.resources.join("tconstruct-original-pack"));
    merge(&original, &mut recipes, &mut tags, &mut materials, &mut material_tags);
    for id in &game.fns.enabled {
        if let Some(pack) = game.fns.packs.iter().find(|pack| &pack.id == id) {
            merge(pack, &mut recipes, &mut tags, &mut materials, &mut material_tags);
        }
    }
    let resolved_tags = game.res.rest.get().map(|rest| {
        let registry = rest.items.registry_order.iter().map(|id| id.to_string()).collect();
        crate::game::tconstruct_source_tags::resolve_registry(&tags, &registry)
    });
    let config = game.world_root.parent().and_then(std::path::Path::parent)
        .ok_or_else(|| "cannot locate the game root for the source config".to_owned())
        .and_then(crate::game::tconstruct_source_config::load);
    *game.world.chunks.__horizon_mixins_tconstruct_source_config_snapshot_mut() = Some(config);
    *game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags_mut() = resolved_tags;
    *game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot_mut() = Some(recipes);
    *game.world.chunks.__horizon_mixins_tconstruct_recipe_tag_snapshot_mut() = Some(tags);
    *game.world.chunks.__horizon_mixins_tconstruct_material_snapshot_mut() = Some(materials);
    *game.world.chunks.__horizon_mixins_tconstruct_material_tag_snapshot_mut() = Some(material_tags);
}
