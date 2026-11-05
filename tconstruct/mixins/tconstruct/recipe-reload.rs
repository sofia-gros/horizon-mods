pub(super) fn mod_snapshot(game: &mut crate::game::Game<'_>) -> (std::collections::HashMap<String, Option<bool>>, bool) {
    let mut native = Vec::new();
    game.native_mods(&mut native);
    let mut states = std::collections::HashMap::new();
    for entry in native {
        if !entry.id.is_empty() { states.insert(entry.id.clone(), game.native_on(&entry.id).then_some(true)); }
    }
    // Unadvertised native hooks and rejected content announcements make the
    // complete set unknowable; a missing ID must remain unknown.
    (states, false)
}

pub(super) fn capture(game: &mut crate::game::Game<'_>) {
    if game.remote.is_some() || game.world_root.as_os_str().is_empty() { return; }
    let mut recipes = std::collections::BTreeMap::new();
    let mut tags = std::collections::BTreeMap::new();
    let mut fluid_tags = std::collections::BTreeMap::new();
    let mut block_tags = std::collections::BTreeMap::new();
    let mut materials = std::collections::BTreeMap::new();
    let mut material_tags = std::collections::BTreeMap::new();
    let registry: std::collections::HashSet<String> = game.motion.health.damage.entries().map(|(_,d)|d.name.clone()).collect();
    let mut damage_seed = std::collections::BTreeMap::new();
    for tag in game.motion.health.damage.tags().filter(|_| !game.cmds.vanilla_disabled) {
        let values: Vec<_> = game.motion.health.damage.entries().filter(|(id,_)| game.motion.health.damage.in_tag(*id,tag)).map(|(_,d)|d.name.clone()).collect();
        damage_seed.insert(tag.to_owned(),vec![serde_json::to_vec(&serde_json::json!({"values":values})).unwrap_or_default()]);
    }
    let mut damage_tags: Result<crate::game::tconstruct_source_damage_tags::Raw,String> = Ok(damage_seed);
    let mut modifiers: Result<crate::game::tconstruct_modifier_snapshot::Raw, String> = Ok(Default::default());
    fn merge(
        pack: &horizon_client::datapack::DataPack,
        recipes: &mut std::collections::BTreeMap<String, Vec<u8>>,
        tags: &mut std::collections::BTreeMap<String, Vec<Vec<u8>>>,
        fluid_tags: &mut std::collections::BTreeMap<String, Vec<Vec<u8>>>,
        block_tags: &mut std::collections::BTreeMap<String, Vec<Vec<u8>>>,
        materials: &mut std::collections::BTreeMap<String, Vec<Vec<u8>>>,
        material_tags: &mut std::collections::BTreeMap<String, Vec<Vec<u8>>>,
        modifiers: &mut Result<crate::game::tconstruct_modifier_snapshot::Raw, String>,
        damage_tags: &mut Result<crate::game::tconstruct_source_damage_tags::Raw,String>,
    ) {
        let files = pack.__horizon_mixins_tconstruct_recipe_files(&["recipes", "tags", "tinkering"]);
        crate::game::tconstruct_recipes::merge_pack(recipes, files.iter().cloned(), |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
        if let Ok(raw) = damage_tags {
            if let Err(error) = crate::game::tconstruct_source_damage_tags::merge_pack(raw,files.iter().cloned(),|name|pack.__horizon_mixins_tconstruct_recipe_read(name)) {
                *damage_tags = Err(error);
            }
        }
        crate::game::tconstruct_source_tags::merge_pack_tags(tags, files.iter().cloned(), |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
        crate::game::tconstruct_source_tags::merge_pack_fluid_tags(fluid_tags, files.iter().cloned(), |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
        crate::game::tconstruct_source_tags::merge_pack_block_tags(block_tags, files.iter().cloned(), |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
        crate::game::tconstruct_materials::merge_pack(materials, material_tags, files.iter().cloned(), |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
        if let Ok(snapshot) = modifiers {
            if let Err(error) = crate::game::tconstruct_modifier_snapshot::merge_pack(snapshot, files, |name| pack.__horizon_mixins_tconstruct_recipe_read(name)) {
                *modifiers = Err(error);
            }
        }
    }
    if !game.cmds.vanilla_disabled {
        let builtin = horizon_client::datapack::DataPack::builtin(&game.options.resources);
        merge(&builtin, &mut recipes, &mut tags, &mut fluid_tags, &mut block_tags, &mut materials, &mut material_tags, &mut modifiers, &mut damage_tags);
        let vanilla = horizon_client::datapack::DataPack::builtin(&game.options.resources.join("minecraft-original-pack"));
        merge(&vanilla, &mut recipes, &mut tags, &mut fluid_tags, &mut block_tags, &mut materials, &mut material_tags, &mut modifiers, &mut damage_tags);
        crate::game::tconstruct_source_tags::bridge_vanilla_mud_tag(&mut block_tags);
    }
    crate::game::tconstruct_source_tags::merge_mantle_fluid_tags(&mut fluid_tags);
    // A content mod can provide the unflattened source layers as ordinary
    // assets; manual installations retain precedence when those layers exist.
    let source_pack = |name: &str| {
        let manual = game.options.resources.join(name);
        if manual.is_dir() { manual } else {
            game.options.resources.join("assets/tconstruct/source-packs").join(name)
        }
    };
    let forge = horizon_client::datapack::DataPack::builtin(&source_pack("forge-original-pack"));
    merge(&forge, &mut recipes, &mut tags, &mut fluid_tags, &mut block_tags, &mut materials, &mut material_tags, &mut modifiers, &mut damage_tags);
    let original = horizon_client::datapack::DataPack::builtin(&source_pack("tconstruct-original-pack"));
    merge(&original, &mut recipes, &mut tags, &mut fluid_tags, &mut block_tags, &mut materials, &mut material_tags, &mut modifiers, &mut damage_tags);
    for id in &game.fns.enabled {
        if let Some(pack) = game.fns.packs.iter().find(|pack| &pack.id == id) {
            merge(pack, &mut recipes, &mut tags, &mut fluid_tags, &mut block_tags, &mut materials, &mut material_tags, &mut modifiers, &mut damage_tags);
        }
    }
    let damage_types = damage_tags.map(|raw| {
        // One failed tag disables only its associated modifier, not all others.
        ["tconstruct:protection/fire", "minecraft:is_fire", "tconstruct:protection/blast", "minecraft:is_explosion", "tconstruct:protection/magic", "tconstruct:protection/projectile", "tconstruct:protection/melee", "tconstruct:rugged/terrain", "tconstruct:rugged/attacks"]
            .into_iter().filter_map(|tag| crate::game::tconstruct_source_damage_tags::project(tag,&raw,&registry)
                .ok().map(|values| (tag.to_owned(),values))).collect::<std::collections::HashMap<_,_>>()
    });
    let resolved_tags = game.res.rest.get().map(|rest| {
        let registry = rest.items.registry_order.iter().map(|id| id.to_string()).collect();
        let mut resolved = crate::game::tconstruct_source_tags::resolve_registry(&tags, &registry)?;
        crate::game::tconstruct_source_tags::fallback_available_heads(&tags, &registry, &mut resolved);
        if !game.cmds.vanilla_disabled {
            let playable = rest.items.items.iter().filter(|(_, info)| info.jukebox_song.is_some()).map(|(id, _)| id.to_string());
            crate::game::tconstruct_source_tags::fallback_music_discs(&tags, &mut resolved, playable);
        }
        Ok(resolved)
    });
    // Resolve against the actual Horizon built-in registry. This may be a
    // strict subset of the original mod's fluids until those are ported.
    let block_ids = horizon_protocol::registry::Builtin::find(&game.options.resources)
        .ok_or_else(|| "missing built-in registry report".to_owned())
        .and_then(|path| horizon_protocol::registry::Builtin::load(&path).map_err(|error| error.to_string()))
        .and_then(|builtin| builtin.get("minecraft:block").map(|ids| ids.ids.clone()).ok_or_else(|| "missing block registry".to_owned()));
    let resolved_blocks = block_ids.as_ref().map_err(Clone::clone).and_then(|ids| {
        let registry = ids.keys().filter(|id| !id.is_empty()).cloned().collect();
        let mut resolved = crate::game::tconstruct_source_tags::resolve_registry(&block_tags, &registry)?;
        // Herbicide's original tags include source blocks not yet registered
        // in this port. Project only these explicit source tags onto the
        // actual registry; unresolved required nested tags remain errors.
        for id in ["minecraft:replaceable_by_trees", "minecraft:leaves", "tconstruct:slimy_soil/vanilla"] {
            if !resolved.contains_key(id) {
                if let Ok(values) = crate::game::tconstruct_source_tags::resolve_available_blocks(id, &block_tags, &registry) {
                    resolved.insert(id.to_owned(), values.into_iter().collect());
                }
            }
        }
        Ok(resolved)
    });
    let resolved_fluids = horizon_protocol::registry::Builtin::find(&game.options.resources)
        .ok_or_else(|| "missing built-in registry report".to_owned())
        .and_then(|path| horizon_protocol::registry::Builtin::load(&path).map_err(|error| error.to_string()))
        .and_then(|builtin| {
            let ids = builtin.get("minecraft:fluid").ok_or_else(|| "missing fluid registry".to_owned())?;
            let registry = ids.names.iter().filter(|id| !id.is_empty()).cloned().collect();
            crate::game::tconstruct_source_tags::resolve_registry(&fluid_tags, &registry)
        });
    let dedicated = game.standalone_server();
    let working_dir = if dedicated {
        std::env::current_dir().map_err(|error| format!("cannot locate server working directory: {error}"))
    } else { Ok(std::path::PathBuf::new()) };
    let config = working_dir.and_then(|working_dir| {
        tconstruct_source_config::root_for_world(&game.world_root, dedicated, &working_dir)
            .ok_or_else(|| "cannot locate the game root for the source config".to_owned())
    }).and_then(|root| tconstruct_source_config::load_all(&root));
    let (config, ore_rates) = match config {
        Ok((config, rates)) => (Ok(config), Ok(rates)),
        Err(error) => (Err(error.clone()), Err(error)),
    };
    let tank_fluids = std::fs::read(game.options.resources.join("tconstruct-melter-fluids.json"))
        .map_err(|error| error.to_string())
        .and_then(|bytes| serde_json::from_slice::<std::collections::HashSet<String>>(&bytes).map_err(|error| error.to_string()))
        .and_then(|source| crate::game::tconstruct_source_melting_lookup::tank_fluid_tags(&fluid_tags, &source));
    // Capture actual native-hook presence; content package names are not
    // sufficient proof that their overlay survived all validation.
    let (states, complete) = mod_snapshot(game);
    let mod_presence = crate::game::tconstruct_source_melter_world::ModPresence { states, complete };
    // All source-recipe consumers share this snapshot. Resolve Forge wrappers
    // once, after tags and config are known, rather than separately per lookup.
    // An unavailable context must never select a guessed fallback branch.
    let condition_context = (|| {
        let item_tags = resolved_tags.as_ref()?.as_ref().ok()?;
        // The machine catalog uses this source-fluid registry, which includes
        // virtual TConstruct tank fluids absent from the built-in registry.
        let (_, fluid_tags) = tank_fluids.as_ref().ok()?;
        let config = config.as_ref().ok()?;
        let items: std::collections::HashSet<String> = game.res.rest.get()?.items.items.keys().map(|id| id.to_string()).collect();
        let registries = [("minecraft:item".to_owned(), item_tags.clone()), ("minecraft:fluid".to_owned(), fluid_tags.clone())].into_iter().collect();
        Some((registries, config, items))
    })();
    recipes.retain(|_, bytes| {
        let wrapper = serde_json::from_slice::<serde_json::Value>(bytes).ok()
            .and_then(|value| value.get("type").and_then(serde_json::Value::as_str).map(str::to_owned))
            .is_some_and(|kind| kind == "forge:conditional");
        if wrapper {
            let Some((registries, config, items)) = &condition_context else { return false; };
            let Some(selected) = crate::game::tconstruct_source_melter_world::conditional_recipe(bytes, registries, config, items, Some(&mod_presence)) else { return false; };
            *bytes = selected;
        }
        // Gate ordinary recipe conditions once using the authoritative snapshot.
        // Downstream parsers do not all have item/mod registry context.
        if serde_json::from_slice::<serde_json::Value>(bytes).is_err() { return true; }
        if let Some(resolved) = crate::game::tconstruct_source_melter_world::unconditional_recipe(bytes) {
            *bytes = resolved;
            return true;
        }
        let Some((registries, config, items)) = &condition_context else { return false; };
        let Some(resolved) = crate::game::tconstruct_source_melter_world::resolved_recipe_conditions(bytes, registries, config, items, Some(&mod_presence)) else { return false; };
        *bytes = resolved;
        true
    });
    // The source basin mud recipe uses BlockTagIngredient. Resolve its block
    // tag through Block::asItem's exported state fact, not a guessed matching
    // item identifier. Keep it unsupported when either registry is missing.
    if let (Ok(block_tags), Some(rest)) = (&resolved_blocks, game.res.rest.get()) {
        for bytes in recipes.values_mut() {
            let Ok(mut recipe) = serde_json::from_slice::<serde_json::Value>(bytes) else { continue; };
            if recipe.get("type").and_then(serde_json::Value::as_str) != Some("tconstruct:casting_basin") { continue; }
            let Some(cast) = recipe.get_mut("cast") else { continue; };
            let Some(object) = cast.as_object() else { continue; };
            if object.len() != 2 || object.get("type").and_then(serde_json::Value::as_str) != Some("tconstruct:block_tag") { continue; }
            let Some(tag) = object.get("tag").and_then(serde_json::Value::as_str) else { continue; };
            let Some(blocks) = block_tags.get(tag) else { continue; };
            let mut items = std::collections::BTreeSet::new();
            for facts in &game.res.facts().states {
                if blocks.contains(facts.name.as_ref()) && facts.item.as_ref() != "minecraft:air"
                    && rest.items.items.contains_key(&facts.item) {
                    items.insert(facts.item.to_string());
                }
            }
            if items.is_empty() { continue; }
            *cast = serde_json::Value::Array(items.into_iter().map(|item| serde_json::json!({"item": item})).collect());
            if let Ok(rewritten) = serde_json::to_vec(&recipe) { *bytes = rewritten; }
        }
    }
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
    // PartCastItem costs use the registered material-casting recipes in this
    // world. Unknown conditions or incomplete item/registry state fail closed.
    let cast_costs = (|| -> Option<Vec<i32>> {
        let item_tags = resolved_tags.as_ref()?.as_ref().ok()?;
        let fluid_tags = resolved_fluids.as_ref().ok()?;
        let config = config.as_ref().ok()?;
        let registry = &game.res.rest.get()?.items.registry_order;
        let material_items: std::collections::HashSet<String> = crate::game::tconstruct_source_cast_info::PARTS
            .iter().map(|part| format!("tconstruct:{part}")).collect();
        if material_items.iter().any(|id| !registry.iter().any(|entry| &**entry == id)) { return None; }
        let filtered: std::collections::BTreeMap<String, Vec<u8>> = recipes.iter().filter_map(|(id, bytes)| {
            let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
            let item = value.get("result")?.as_str()?;
            if !material_items.contains(item) { return None; }
            Some((id.clone(), bytes.clone()))
        }).collect();
        let registries = [("minecraft:item".to_owned(), item_tags.clone()), ("minecraft:fluid".to_owned(), fluid_tags.clone())].into_iter().collect();
        let costs = crate::game::tconstruct_tx::source_material_casting_costs(&filtered, &material_items, &registries, config, true).ok()?;
        Some(crate::game::tconstruct_source_cast_info::PARTS.iter().map(|part| costs.get(&format!("tconstruct:{part}")).copied().unwrap_or(0)).collect())
    })();
    let known_items = game.res.rest.get().map(|rest| rest.items.items.keys().map(|id| id.to_string()).collect());
    *game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context_mut() = Some(tank_fluids);
    *game.world.chunks.__horizon_mixins_tconstruct_melter_known_items_mut() = known_items;
    *game.world.chunks.__horizon_mixins_tconstruct_source_cast_costs_mut() = cast_costs;
    let durability = game.res.rest.get().map(|rest| {
        rest.items.items.iter().map(|(id, info)| (id.to_string(), info.max_damage)).collect()
    });
    *game.world.chunks.__horizon_mixins_tconstruct_source_native_durability_mut() = durability;
    *game.world.chunks.__horizon_mixins_tconstruct_source_config_snapshot_mut() = Some(config);
    *game.world.chunks.__horizon_mixins_tconstruct_source_ore_rates_mut() = Some(ore_rates);
    *game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags_mut() = resolved_tags;
    *game.world.chunks.__horizon_mixins_tconstruct_resolved_block_ids_mut() = Some(block_ids);
    *game.world.chunks.__horizon_mixins_tconstruct_resolved_block_tags_mut() = Some(resolved_blocks);
    *game.world.chunks.__horizon_mixins_tconstruct_fluid_tag_snapshot_mut() = Some(fluid_tags);
    *game.world.chunks.__horizon_mixins_tconstruct_resolved_fluid_tags_mut() = Some(resolved_fluids);
    *game.world.chunks.__horizon_mixins_tconstruct_resolved_material_definitions_mut() = Some(definitions);
    *game.world.chunks.__horizon_mixins_tconstruct_protection_damage_types_mut() = Some(damage_types);
    *game.world.chunks.__horizon_mixins_tconstruct_resolved_material_stats_mut() = Some(stats);
    *game.world.chunks.__horizon_mixins_tconstruct_source_shared_recipes_mut() = Some(std::sync::Arc::new(recipes.clone()));
    *game.world.chunks.__horizon_mixins_tconstruct_station_building_recipes_mut() = Some(
        crate::game::tconstruct_source_station_recipes::building_recipe_index(&recipes));
    *game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot_mut() = Some(recipes);
    *game.world.chunks.__horizon_mixins_tconstruct_recipe_mod_snapshot_mut() = Some((mod_presence.states, mod_presence.complete));
    *game.world.chunks.__horizon_mixins_tconstruct_recipe_tag_snapshot_mut() = Some(tags);
    *game.world.chunks.__horizon_mixins_tconstruct_material_snapshot_mut() = Some(materials);
    *game.world.chunks.__horizon_mixins_tconstruct_material_tag_snapshot_mut() = Some(material_tags);
    *game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot_mut() = Some(modifiers);
    let generation = game.world.chunks.__horizon_mixins_tconstruct_source_recipe_generation().wrapping_add(1);
    *game.world.chunks.__horizon_mixins_tconstruct_source_recipe_generation_mut() = generation;
    crate::game::tconstruct_gold_guard_world::refresh(game);
    crate::game::tconstruct_projectile_protection_world::refresh(game);
    crate::game::tconstruct_fire_duration_world::refresh(game);
    crate::game::tconstruct_protection_damage_world::refresh(game);
    crate::game::tconstruct_rugged_world::refresh(game);
    crate::game::tconstruct_depth_protection_world::refresh(game);
}

// The active level is captured below the reload hook. Parked levels retain
// their own ChunkMap and must not continue serving pre-reload tag membership.
pub(super) fn invalidate_parked_item_tags(game: &mut crate::game::Game<'_>) {
    for dim in [crate::game::Dim::Overworld, crate::game::Dim::Nether, crate::game::Dim::End] {
        if let Some(world) = game.levels.parked_world_mut(dim) {
            *world.chunks.__horizon_mixins_tconstruct_resolved_item_tags_mut() = None;
            *world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot_mut() = None;
            *world.chunks.__horizon_mixins_tconstruct_protection_damage_types_mut() = None;
        }
    }
}

// The authoritative recipe tables are shared among the active and parked
// dimensions; invalidate every open LAN table's cached generation.
pub(super) fn invalidate_lan_crafting_results(game: &mut crate::game::Game<'_>) {
    fn invalidate(chunks: &mut horizon_client_render::chunk_map::ChunkMap) {
        let generation = chunks.__horizon_mixins_tconstruct_lan_craft_recipe_generation_mut();
        *generation = generation.wrapping_add(1).max(1);
        chunks.__horizon_mixins_tconstruct_lan_craft_menu_generation_mut().clear();
    }
    invalidate(&mut game.world.chunks);
    for dim in [crate::game::Dim::Overworld, crate::game::Dim::Nether, crate::game::Dim::End] {
        if let Some(world) = game.levels.parked_world_mut(dim) { invalidate(&mut world.chunks); }
    }
}
