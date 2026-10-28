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
        let wood = materials.get("tconstruct:tinkering/materials/definition/wood").and_then(|v| v.last()).map(|v| format!("{:x}", Sha256::digest(v))).unwrap_or_else(|| "missing".into());
        eprintln!("TCONSTRUCT_MATERIAL_PROBE {probe}");
        eprintln!("TCONSTRUCT_MATERIAL_COUNT_PROBE {}", materials.keys().filter(|id| id.starts_with("tconstruct:")).count());
        eprintln!("TCONSTRUCT_MATERIAL_WOOD_PROBE {wood}");
    } else { eprintln!("TCONSTRUCT_MATERIAL_PROBE uninitialized"); }
    if let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_material_tag_snapshot() {
        let files = tags.get("test:tinkering/tags/materials/probe").map(|v| v.iter().map(|b| String::from_utf8_lossy(b).into_owned()).collect::<Vec<_>>().join("|"));
        eprintln!("TCONSTRUCT_MATERIAL_TAG_PROBE {}", files.as_deref().unwrap_or("missing"));
        eprintln!("TCONSTRUCT_MATERIAL_TAG_COUNT_PROBE {}", tags.keys().filter(|id| id.starts_with("tconstruct:")).count());
    } else { eprintln!("TCONSTRUCT_MATERIAL_TAG_PROBE uninitialized"); }
    if let Some(Ok(tags)) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags() {
        let has = |tag: &str, item: &str| tags.get(tag).is_some_and(|items| items.contains(item));
        eprintln!("TCONSTRUCT_RESOLVED_OAK {} {} {}", has("minecraft:oak_logs", "minecraft:oak_log"), has("minecraft:oak_logs", "minecraft:stripped_oak_log"), has("tconstruct:patterns/default", "tconstruct:pattern"));
    } else { eprintln!("TCONSTRUCT_RESOLVED_OAK unavailable"); }
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
