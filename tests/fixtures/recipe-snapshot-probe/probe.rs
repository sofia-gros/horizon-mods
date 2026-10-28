fn inspect_snapshot(value: R<i32>, game: &mut crate::game::Game<'_>) -> R<i32> {
    if let Some(recipes) = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot() {
        let probe = recipes.get("test:probe").and_then(|v| std::str::from_utf8(v).ok()).unwrap_or("missing");
        eprintln!("TCONSTRUCT_RECIPE_PROBE {probe}");
    } else {
        eprintln!("TCONSTRUCT_RECIPE_PROBE uninitialized");
    }
    value
}
