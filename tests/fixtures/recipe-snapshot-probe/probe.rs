fn inspect_snapshot(value: R<i32>, game: &mut crate::game::Game<'_>) -> R<i32> {
    if let Some(recipes) = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot() {
        let probe = recipes.get("test:probe").and_then(|v| std::str::from_utf8(v).ok()).unwrap_or("missing");
        eprintln!("TCONSTRUCT_RECIPE_PROBE {probe}");
        eprintln!("TCONSTRUCT_COUNT_PROBE {}", recipes.keys().filter(|id| id.starts_with("tconstruct:")).count());
        // A controlled test context: this does not represent the world's full
        // registry, tags or config and is not connected to transactions.
        let mut item_tags = std::collections::HashMap::new();
        item_tags.insert("minecraft:oak_logs".to_owned(), ["minecraft:oak_log".to_owned()].into_iter().collect());
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
    value
}
