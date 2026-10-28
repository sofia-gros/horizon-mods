fn inspect_snapshot(value: R<i32>, game: &mut crate::game::Game<'_>) -> R<i32> {
    if let Some(recipes) = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot() {
        let probe = recipes.get("test:probe").and_then(|v| std::str::from_utf8(v).ok()).unwrap_or("missing");
        eprintln!("TCONSTRUCT_RECIPE_PROBE {probe}");
        eprintln!("TCONSTRUCT_COUNT_PROBE {}", recipes.keys().filter(|id| id.starts_with("tconstruct:")).count());
        // A controlled test context: this does not represent the world's full
        // registry, tags or config and is not connected to transactions.
        let mut item_tags = std::collections::HashMap::new();
        item_tags.insert("minecraft:oak_logs".to_owned(), ["minecraft:oak_log".to_owned()].into_iter().collect());
        item_tags.insert("tconstruct:patterns/default".to_owned(), ["tconstruct:pattern".to_owned()].into_iter().collect());
        let instruments = Default::default();
        let instrument_tags = Default::default();
        let registries = Default::default();
        let config = Default::default();
        for (name, item) in [("FLINT", "minecraft:flint"), ("OAK_LOG", "minecraft:oak_log")] {
            let result = crate::game::tconstruct_tx::source_material_lookup(recipes, item, None, &item_tags, &instruments, &instrument_tags, &registries, &config, true);
            match result {
                Ok(Some(found)) => {
                    let leftover = found.leftover.as_ref().and_then(|v| v.count_for_remainder(3, found.needed)).unwrap_or(0);
                    eprintln!("TCONSTRUCT_LOOKUP_{name} {} {} {} {} {}", found.id, found.material, found.value, found.needed, leftover);
                },
                other => eprintln!("TCONSTRUCT_LOOKUP_{name} {other:?}"),
            }
        }
        let empty = crate::game::tconstruct_tx::source_part_choices(recipes, "tconstruct:pattern", &crate::game::tconstruct_tx::SourcePartInput::Empty, &item_tags, &instruments, &instrument_tags, &registries, &config, true);
        match empty {
            Ok(choices) => eprintln!("TCONSTRUCT_CHOICES_EMPTY {} {}", choices.len(), choices.first().map_or("none", |choice| choice.pattern.as_str())),
            Err(error) => eprintln!("TCONSTRUCT_CHOICES_EMPTY {error:?}"),
        }
        let supported = ["tconstruct:pick_head", "tconstruct:tool_handle", "tconstruct:tool_binding"].into_iter().map(str::to_owned).collect();
        let input = crate::game::tconstruct_tx::SourcePartInput::Material { input_item:"minecraft:oak_planks", material:"tconstruct:wood", craftable:true, supported_outputs:&supported };
        let wood = crate::game::tconstruct_tx::source_part_choices(recipes, "tconstruct:pattern", &input, &item_tags, &instruments, &instrument_tags, &registries, &config, true);
        match wood {
            Ok(choices) => eprintln!("TCONSTRUCT_CHOICES_WOOD {}", choices.iter().map(|choice| format!("{}:{}", choice.pattern, choice.cost)).collect::<Vec<_>>().join(",")),
            Err(error) => eprintln!("TCONSTRUCT_CHOICES_WOOD {error:?}"),
        }
        // Controlled item IDs and loaded-mod set; this probes source data,
        // not the unimplemented normal Part Builder transaction.
        let mods = std::collections::HashSet::new();
        let recycled = crate::game::tconstruct_tx::source_all_part_choices(recipes, "tconstruct:venom_bottle", &crate::game::tconstruct_tx::SourcePartInput::Invalid, "minecraft:flint_and_steel", 1, &item_tags, &instruments, &instrument_tags, &registries, &config, Some(&mods), None, true);
        match recycled {
            Ok(choices) => eprintln!("TCONSTRUCT_RECYCLING_CHOICES {}", choices.iter().map(|c| format!("{}:{}", c.pattern, c.cost)).collect::<Vec<_>>().join(",")),
            Err(error) => eprintln!("TCONSTRUCT_RECYCLING_CHOICES {error:?}"),
        }
        for (marker, id) in [("STICK", "minecraft:stick"), ("BOAT", "minecraft:acacia_chest_boat")] {
            use sha2::{Digest, Sha256};
            let hash = recipes.get(id).map(|bytes| format!("{:x}", Sha256::digest(bytes))).unwrap_or_else(|| "missing".into());
            eprintln!("TCONSTRUCT_{marker}_PROBE {hash}");
        }
    } else {
        eprintln!("TCONSTRUCT_RECIPE_PROBE uninitialized");
    }
    if let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_recipe_tag_snapshot() {
        let files = tags.get("test:probe").map(|v| v.iter().map(|b| String::from_utf8_lossy(b).into_owned()).collect::<Vec<_>>().join("|"));
        eprintln!("TCONSTRUCT_TAG_PROBE {}", files.as_deref().unwrap_or("missing"));
        eprintln!("TCONSTRUCT_PLANKS_PROBE {}", tags.get("minecraft:planks").map_or(0, Vec::len));
        eprintln!("TCONSTRUCT_FORGE_IRON_PROBE {}", tags.get("forge:ingots/iron").map_or(0, Vec::len));
    } else { eprintln!("TCONSTRUCT_TAG_PROBE uninitialized"); }
    if let Some(materials) = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot() {
        use sha2::{Digest, Sha256};
        let probe = materials.get("test:tinkering/materials/definition/probe").and_then(|v| v.last()).and_then(|v| std::str::from_utf8(v).ok()).unwrap_or("missing");
        let parsed = serde_json::from_str::<serde_json::Value>(probe).ok();
        let probe = parsed.as_ref().and_then(|v| v.get("marker")).and_then(serde_json::Value::as_str).unwrap_or(probe);
        let wood = materials.get("tconstruct:tinkering/materials/definition/wood").and_then(|v| v.last()).map(|v| format!("{:x}", Sha256::digest(v))).unwrap_or_else(|| "missing".into());
        eprintln!("TCONSTRUCT_MATERIAL_PROBE {probe}");
        eprintln!("TCONSTRUCT_MATERIAL_COUNT_PROBE {}", materials.keys().filter(|id| id.starts_with("tconstruct:")).count());
        eprintln!("TCONSTRUCT_MATERIAL_WOOD_PROBE {wood}");
        let tool = materials.get("tconstruct:tinkering/tool_definitions/pickaxe").and_then(|v| v.last());
        let tool_value = tool.and_then(|v| serde_json::from_slice::<serde_json::Value>(v).ok());
        let marker = tool_value.as_ref().and_then(|v| v.get("marker")).and_then(serde_json::Value::as_str).map(str::to_owned)
            .or_else(|| tool.map(|v| format!("{:x}", Sha256::digest(v)))).unwrap_or_else(|| "missing".into());
        eprintln!("TCONSTRUCT_TOOL_DEFINITION_PROBE {marker}");
        // Test-only context: fixed source fixture modules and registered item
        // IDs are supplied explicitly; this is not full module validation.
        let test_parts = tool.and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok()).and_then(|value| {
            let modules = value.get("modules").and_then(serde_json::Value::as_array)?;
            let mut defaults = std::collections::HashMap::new();
            for module in modules {
                let typ = module.get("type")?.as_str()?;
                defaults.insert(typ.to_owned(), matches!(typ, "tconstruct:part_stats" | "tconstruct:tool_parts"));
            }
            let registered = game.res.rest.get()?.items.registry_order.iter().map(|id| id.to_string()).collect();
            crate::game::tconstruct_tx::source_tool_parts_hook(&[tool?.clone()], true, &defaults, &registered).ok()
        });
        eprintln!("TCONSTRUCT_TOOL_PARTS_PROBE {}", test_parts.map_or("unavailable".into(), |parts| parts.join(",")));
    } else { eprintln!("TCONSTRUCT_MATERIAL_PROBE uninitialized"); }
    if let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_material_tag_snapshot() {
        let files = tags.get("test:tinkering/tags/materials/probe").map(|v| v.iter().map(|b| String::from_utf8_lossy(b).into_owned()).collect::<Vec<_>>().join("|"));
        eprintln!("TCONSTRUCT_MATERIAL_TAG_PROBE {}", files.as_deref().unwrap_or("missing"));
        eprintln!("TCONSTRUCT_MATERIAL_TAG_COUNT_PROBE {}", tags.keys().filter(|id| id.starts_with("tconstruct:")).count());
    } else { eprintln!("TCONSTRUCT_MATERIAL_TAG_PROBE uninitialized"); }
    if let Some(Ok(tags)) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags() {
        let has = |tag: &str, item: &str| tags.get(tag).is_some_and(|items| items.contains(item));
        eprintln!("TCONSTRUCT_RESOLVED_OAK {} {} {}", has("minecraft:oak_logs", "minecraft:oak_log"), has("minecraft:oak_logs", "minecraft:stripped_oak_log"), has("tconstruct:patterns/default", "tconstruct:pattern"));
        let bartered = tags.get("tconstruct:parts/bartered");
        eprintln!("TCONSTRUCT_REGISTERED_PART_TAGS {} {} {}", bartered.map_or(0, |v| v.len()), has("tconstruct:parts/bartered", "tconstruct:pick_head"), tags.contains_key("tconstruct:parts"));
    } else { eprintln!("TCONSTRUCT_RESOLVED_OAK unavailable"); }
    if let Some(Ok(tags)) = game.world.chunks.__horizon_mixins_tconstruct_resolved_fluid_tags() {
        let has = |tag: &str, fluid: &str| tags.get(tag).is_some_and(|values| values.contains(fluid));
        eprintln!("TCONSTRUCT_RESOLVED_CREOSOTE {} {}", has("forge:creosote", "minecraft:water"), has("forge:creosote", "minecraft:lava"));
    } else { eprintln!("TCONSTRUCT_RESOLVED_CREOSOTE unavailable"); }
    match game.world.chunks.__horizon_mixins_tconstruct_resolved_material_definitions() {
        Some(Ok((materials, _, _))) => eprintln!("TCONSTRUCT_WORLD_MATERIALS {} {} {}", materials.contains_key("tconstruct:wood"), materials.contains_key("tconstruct:flint"), materials.contains_key("tconstruct:treated_wood")),
        Some(Err(error)) => eprintln!("TCONSTRUCT_WORLD_MATERIALS error {error}"),
        None => eprintln!("TCONSTRUCT_WORLD_MATERIALS unavailable"),
    }
    match game.world.chunks.__horizon_mixins_tconstruct_resolved_material_stats() {
        Some(Ok(stats)) => {
            let flint = stats.get("tconstruct:flint");
            let head = flint.and_then(|s| s.get("tconstruct:head"));
            let binding = flint.and_then(|s| s.get("tconstruct:binding"));
            eprintln!("TCONSTRUCT_WORLD_FLINT_STATS {} {} {}", head.is_some(), binding.is_some(), head.copied().unwrap_or(false));
        },
        Some(Err(error)) => eprintln!("TCONSTRUCT_WORLD_FLINT_STATS error {error}"),
        None => eprintln!("TCONSTRUCT_WORLD_FLINT_STATS unavailable"),
    }
    if let (Some(Ok((definitions, _, _))), Some(Ok(stats)), Some(Ok(item_tags)), Some(Ok(fluid_tags)), Some(Ok(config)), Some(recipes)) = (
        game.world.chunks.__horizon_mixins_tconstruct_resolved_material_definitions(),
        game.world.chunks.__horizon_mixins_tconstruct_resolved_material_stats(),
        game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags(),
        game.world.chunks.__horizon_mixins_tconstruct_resolved_fluid_tags(),
        game.world.chunks.__horizon_mixins_tconstruct_source_config_snapshot(),
        game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot(),
    ) {
        let registries = [("minecraft:item".to_owned(), item_tags.clone()), ("minecraft:fluid".to_owned(), fluid_tags.clone())].into_iter().collect();
        let empty = Default::default();
        let lookup = crate::game::tconstruct_tx::source_material_lookup(recipes, "minecraft:flint", None, item_tags, &empty, &Default::default(), &registries, config, true);
        let choices = lookup.ok().flatten().and_then(|found| {
            let definition = definitions.get(&found.material)?;
            let stat_map = stats.get(&found.material)?;
            let valid = stat_map.keys().cloned().collect();
            let supported = crate::game::tconstruct_tx::source_supported_original_parts(&valid, stat_map.values().any(|v| *v));
            let input = crate::game::tconstruct_tx::SourcePartInput::Material { input_item: "minecraft:flint", material: &found.material, craftable: definition.0, supported_outputs: &supported };
            crate::game::tconstruct_tx::source_part_choices(recipes, "tconstruct:pattern", &input, item_tags, &empty, &Default::default(), &registries, config, true).ok()
        });
        if let Some(choices) = choices {
            let has = |pattern: &str| choices.iter().any(|choice| choice.pattern == pattern);
            eprintln!("TCONSTRUCT_WORLD_FLINT_CHOICES {} {} {}", has("tconstruct:pick_head"), has("tconstruct:tool_handle"), has("tconstruct:tool_binding"));
        } else { eprintln!("TCONSTRUCT_WORLD_FLINT_CHOICES unavailable"); }
    } else { eprintln!("TCONSTRUCT_WORLD_FLINT_CHOICES unavailable"); }
    if let (Some(Ok((definitions, _, material_tags))), Some(Ok(item_tags)), Some(Ok(fluid_tags)), Some(Ok(config)), Some(recipes)) = (
        game.world.chunks.__horizon_mixins_tconstruct_resolved_material_definitions(),
        game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags(),
        game.world.chunks.__horizon_mixins_tconstruct_resolved_fluid_tags(),
        game.world.chunks.__horizon_mixins_tconstruct_source_config_snapshot(),
        game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot(),
    ) {
        let registries = [("minecraft:item".to_owned(), item_tags.clone()), ("minecraft:fluid".to_owned(), fluid_tags.clone())].into_iter().collect();
        let visible = definitions.iter().filter(|(_, value)| !value.1).map(|(id, _)| id.clone()).collect();
        let known = game.res.rest.get().map(|rest| rest.items.registry_order.iter().map(|id| id.to_string()).collect());
        let result = known.ok_or(crate::game::tconstruct_tx::SourceLookupError::IncompleteContext)
            .and_then(|known| crate::game::tconstruct_tx::source_visible_base_recipe_values("tconstruct:bronze", recipes, &visible, &known, &registries, config, true))
            .and_then(|values| crate::game::tconstruct_tx::source_supported_fake_items("tconstruct:bronze", material_tags, &values, true));
        match result {
            Ok(items) => eprintln!("TCONSTRUCT_FAKE_BRONZE {} {}", items.contains("tconstruct:fake_ingot"), items.contains("tconstruct:fake_storage_block")),
            Err(error) => eprintln!("TCONSTRUCT_FAKE_BRONZE error {error:?}"),
        }
    } else { eprintln!("TCONSTRUCT_FAKE_BRONZE unavailable"); }
    match game.world.chunks.__horizon_mixins_tconstruct_source_config_snapshot() {
        Some(Ok(config)) => eprintln!("TCONSTRUCT_CONFIG_PROBE {} {}", config.get("force_integration_materials").copied().unwrap_or(true), config.get("allow_ingotless_alloys").copied().unwrap_or(false)),
        Some(Err(error)) => eprintln!("TCONSTRUCT_CONFIG_PROBE unsupported {error}"),
        None => eprintln!("TCONSTRUCT_CONFIG_PROBE uninitialized"),
    }
    if let (Some(Ok(tags)), Some(Ok(config)), Some(recipes)) = (
        game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags(),
        game.world.chunks.__horizon_mixins_tconstruct_source_config_snapshot(),
        game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot(),
    ) {
        let mut registries = std::collections::HashMap::new();
        registries.insert("minecraft:item".to_owned(), tags.clone());
        let instruments = Default::default();
        let instrument_tags = Default::default();
        for (name, item) in [("FLINT", "minecraft:flint"), ("OAK_LOG", "minecraft:oak_log")] {
            // This diagnostic uses the real item tags and default config.
            // Other registries and full TConstruct item registration are not
            // complete, so it is not a production crafting context.
            let result = crate::game::tconstruct_tx::source_material_lookup(recipes, item, None, tags, &instruments, &instrument_tags, &registries, config, true);
            match result {
                Ok(Some(found)) => {
                    eprintln!("TCONSTRUCT_REAL_LOOKUP_{name} {} {} {} {}", found.id, found.material, found.value, found.needed);
                    let recipe_id = "tconstruct:tools/parts/builder/pick_head";
                    let plan = recipes.get(recipe_id).and_then(|bytes| serde_json::from_slice(bytes).ok()).and_then(|value| crate::game::tconstruct_tx::SourcePartRecipe::parse(recipe_id.into(), &value)).and_then(|recipe| {
                        let supported = ["tconstruct:pick_head".to_owned()].into_iter().collect();
                        let input = crate::game::tconstruct_tx::SourcePartInput::Material { input_item:item, material:&found.material, craftable:true, supported_outputs:&supported };
                        crate::game::tconstruct_tx::source_part_plan(&recipe, &input, Some(&found), 3, false).ok().flatten()
                    });
                    match plan {
                        Some(plan) => eprintln!("TCONSTRUCT_REAL_PLAN_{name} {} {} {}", plan.materials_used, plan.pattern_used, plan.leftover.as_ref().map_or(0, |(_, count)| *count)),
                        None => eprintln!("TCONSTRUCT_REAL_PLAN_{name} unavailable"),
                    }
                },
                Ok(None) => { eprintln!("TCONSTRUCT_REAL_LOOKUP_{name} none"); eprintln!("TCONSTRUCT_REAL_PLAN_{name} none"); },
                Err(error) => { eprintln!("TCONSTRUCT_REAL_LOOKUP_{name} {error:?}"); eprintln!("TCONSTRUCT_REAL_PLAN_{name} unavailable"); },
            }
        }
    } else { eprintln!("TCONSTRUCT_REAL_LOOKUP_FLINT unavailable"); eprintln!("TCONSTRUCT_REAL_LOOKUP_OAK_LOG unavailable"); eprintln!("TCONSTRUCT_REAL_PLAN_FLINT unavailable"); eprintln!("TCONSTRUCT_REAL_PLAN_OAK_LOG unavailable"); }
    value
}
