fn inspect_snapshot(value: R<i32>, game: &mut crate::game::Game<'_>) -> R<i32> {
    if let Some(recipes) = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot() {
        let probe = recipes.get("test:probe").and_then(|v| std::str::from_utf8(v).ok()).unwrap_or("missing");
        eprintln!("TCONSTRUCT_RECIPE_PROBE {probe}");
        eprintln!("TCONSTRUCT_COUNT_PROBE {}", recipes.keys().filter(|id| id.starts_with("tconstruct:")).count());
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
    value
}
