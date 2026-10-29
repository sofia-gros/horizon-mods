pub(super) fn capture(game: &mut crate::game::Game<'_>) {
    if game.remote.is_some() || game.world_root.as_os_str().is_empty() { return; }
    let mut recipes = std::collections::BTreeMap::new();
    let mut tags = std::collections::BTreeMap::new();
    let mut fluid_tags = std::collections::BTreeMap::new();
    let mut materials = std::collections::BTreeMap::new();
    let mut material_tags = std::collections::BTreeMap::new();
    fn merge(
        pack: &horizon_client::datapack::DataPack,
        recipes: &mut std::collections::BTreeMap<String, Vec<u8>>,
        tags: &mut std::collections::BTreeMap<String, Vec<Vec<u8>>>,
        fluid_tags: &mut std::collections::BTreeMap<String, Vec<Vec<u8>>>,
        materials: &mut std::collections::BTreeMap<String, Vec<Vec<u8>>>,
        material_tags: &mut std::collections::BTreeMap<String, Vec<Vec<u8>>>,
    ) {
        let files = pack.__horizon_mixins_tconstruct_recipe_files(&["recipes", "tags", "tinkering"]);
        crate::game::tconstruct_recipes::merge_pack(recipes, files.iter().cloned(), |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
        crate::game::tconstruct_source_tags::merge_pack_tags(tags, files.iter().cloned(), |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
        crate::game::tconstruct_source_tags::merge_pack_fluid_tags(fluid_tags, files.iter().cloned(), |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
        crate::game::tconstruct_materials::merge_pack(materials, material_tags, files, |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
    }
    if !game.cmds.vanilla_disabled {
        let builtin = horizon_client::datapack::DataPack::builtin(&game.options.resources);
        merge(&builtin, &mut recipes, &mut tags, &mut fluid_tags, &mut materials, &mut material_tags);
        let vanilla = horizon_client::datapack::DataPack::builtin(&game.options.resources.join("minecraft-original-pack"));
        merge(&vanilla, &mut recipes, &mut tags, &mut fluid_tags, &mut materials, &mut material_tags);
    }
    let forge = horizon_client::datapack::DataPack::builtin(&game.options.resources.join("forge-original-pack"));
    merge(&forge, &mut recipes, &mut tags, &mut fluid_tags, &mut materials, &mut material_tags);
    let original = horizon_client::datapack::DataPack::builtin(&game.options.resources.join("tconstruct-original-pack"));
    merge(&original, &mut recipes, &mut tags, &mut fluid_tags, &mut materials, &mut material_tags);
    for id in &game.fns.enabled {
        if let Some(pack) = game.fns.packs.iter().find(|pack| &pack.id == id) {
            merge(pack, &mut recipes, &mut tags, &mut fluid_tags, &mut materials, &mut material_tags);
        }
    }
    let resolved_tags = game.res.rest.get().map(|rest| {
        let registry = rest.items.registry_order.iter().map(|id| id.to_string()).collect();
        crate::game::tconstruct_source_tags::resolve_registry(&tags, &registry)
    });
    // Resolve against the actual Horizon built-in registry. This may be a
    // strict subset of the original mod's fluids until those are ported.
    let resolved_fluids = horizon_protocol::registry::Builtin::find(&game.options.resources)
        .ok_or_else(|| "missing built-in registry report".to_owned())
        .and_then(|path| horizon_protocol::registry::Builtin::load(&path).map_err(|error| error.to_string()))
        .and_then(|builtin| {
            let ids = builtin.get("minecraft:fluid").ok_or_else(|| "missing fluid registry".to_owned())?;
            let registry = ids.names.iter().filter(|id| !id.is_empty()).cloned().collect();
            crate::game::tconstruct_source_tags::resolve_registry(&fluid_tags, &registry)
        });
    let config = game.world_root.parent().and_then(std::path::Path::parent)
        .ok_or_else(|| "cannot locate the game root for the source config".to_owned())
        .and_then(crate::game::tconstruct_source_config::load);
    // Preserve an explicit failure when any required input is unavailable.
    // These definitions describe the current Horizon registries, which do not
    // yet contain every original TConstruct item or fluid.
    let definitions = (|| -> Result<_, String> {
        let item_tags = resolved_tags.as_ref().ok_or("item registry not ready")?.as_ref().map_err(Clone::clone)?;
        let fluid_tags = resolved_fluids.as_ref().map_err(Clone::clone)?;
        let config = config.as_ref().map_err(Clone::clone)?;
        let registries = [("minecraft:item".to_owned(), item_tags.clone()), ("minecraft:fluid".to_owned(), fluid_tags.clone())].into_iter().collect();
        let (materials, redirects) = crate::game::tconstruct_tx::source_material_registry(&materials, &registries, config, true)
            .map_err(|error| format!("material definitions: {error:?}"))?;
        let ids = materials.keys().cloned().collect();
        let material_tags = crate::game::tconstruct_source_tags::resolve_material_registry(&material_tags, &ids)?;
        let definitions = materials.into_iter().filter_map(|(id, value)| {
            match value {
                crate::game::tconstruct_tx::SourceMaterialDefinition::Material { craftable, hidden, tier, sort_order, rarity } =>
                    Some((id, (craftable, hidden, tier, sort_order, rarity))),
                _ => None,
            }
        }).collect();
        Ok((definitions, redirects, material_tags))
    })();
    // Forge 1.20.1's six vanilla tiers are known. Other Mixins may add tier
    // types, so an unknown tier is an incomplete context, never an automatic
    // rejection of the original stat.
    let stats = (|| -> Result<_, String> {
        let tier_names = ["wood", "stone", "iron", "gold", "diamond", "netherite"].into_iter()
            .map(|name| format!("minecraft:{name}")).collect();
        let tiers = crate::game::tconstruct_source_stat_types::TierRegistry { names: &tier_names, complete: false };
        let mut result = std::collections::HashMap::new();
        for (id, layers) in &materials {
            let Some((namespace, path)) = id.split_once(':') else { return Err(format!("invalid material stats {id}")); };
            let Some(path) = path.strip_prefix("tinkering/materials/stats/") else { continue; };
            let merged = crate::game::tconstruct_tx::source_merge_material_stats(layers)
                .map_err(|error| format!("material stats {id}: {error:?}"))?;
            let validated = crate::game::tconstruct_source_stat_types::validate_builtin_stats(&merged, Some(&tiers))
                .map_err(|error| format!("material stats {id}: {error}"))?;
            result.insert(format!("{namespace}:{path}"), validated);
        }
        Ok(result)
    })();
    *game.world.chunks.__horizon_mixins_tconstruct_source_config_snapshot_mut() = Some(config);
    *game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags_mut() = resolved_tags;
    *game.world.chunks.__horizon_mixins_tconstruct_fluid_tag_snapshot_mut() = Some(fluid_tags);
    *game.world.chunks.__horizon_mixins_tconstruct_resolved_fluid_tags_mut() = Some(resolved_fluids);
    *game.world.chunks.__horizon_mixins_tconstruct_resolved_material_definitions_mut() = Some(definitions);
    *game.world.chunks.__horizon_mixins_tconstruct_resolved_material_stats_mut() = Some(stats);
    *game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot_mut() = Some(recipes);
    *game.world.chunks.__horizon_mixins_tconstruct_recipe_tag_snapshot_mut() = Some(tags);
    *game.world.chunks.__horizon_mixins_tconstruct_material_snapshot_mut() = Some(materials);
    *game.world.chunks.__horizon_mixins_tconstruct_material_tag_snapshot_mut() = Some(material_tags);
    game.world.chunks.__horizon_mixins_tconstruct_builder_choices_mut().clear();
    game.world.chunks.__horizon_mixins_tconstruct_builder_preview_mut().clear();
}
