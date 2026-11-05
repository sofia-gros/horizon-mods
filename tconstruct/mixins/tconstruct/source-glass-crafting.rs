// Config-gated ordinary recipes and discovery emitted before each world load.
// Default glass tags come from the content package; datapacks retain final precedence.
pub(super) const RECIPES: &[(&str, &[u8])] = &[
    ("recipe/tconstruct:common/glass/vanilla/glass_bottle", br##"{"type":"minecraft:crafting_shaped","category":"misc","key":{"#":"#forge:glass/colorless"},"pattern":["# #"," # "],"result":{"id":"minecraft:glass_bottle","count":3},"show_notification":true}"##),
    ("recipe/tconstruct:common/glass/vanilla/beacon", br##"{"type":"minecraft:crafting_shaped","category":"misc","key":{"G":"#forge:glass/colorless","O":"minecraft:obsidian","S":"minecraft:nether_star"},"pattern":["GGG","GSG","OOO"],"result":{"id":"minecraft:beacon","count":1},"show_notification":true}"##),
    ("recipe/tconstruct:common/glass/vanilla/daylight_detector", br##"{"type":"minecraft:crafting_shaped","category":"redstone","key":{"G":"#forge:glass/colorless","Q":"minecraft:quartz","W":"#minecraft:wooden_slabs"},"pattern":["GGG","QQQ","WWW"],"result":{"id":"minecraft:daylight_detector","count":1},"show_notification":true}"##),
    ("recipe/tconstruct:common/glass/vanilla/end_crystal", br##"{"type":"minecraft:crafting_shaped","category":"misc","key":{"E":"minecraft:ender_eye","G":"#forge:glass/colorless","T":"minecraft:ghast_tear"},"pattern":["GGG","GEG","GTG"],"result":{"id":"minecraft:end_crystal","count":1},"show_notification":true}"##),
];

// Key each source discovery by its native recipe; no positional pairing.
pub(super) const ADVANCEMENTS: &[(&str, &str, &[u8])] = &[
    ("recipe/tconstruct:common/glass/vanilla/glass_bottle", "advancement/tconstruct:recipes/brewing/common/glass/vanilla/glass_bottle", br##"{"parent":"minecraft:recipes/root","criteria":{"has_glass":{"conditions":{"items":[{"items":"#forge:glass/colorless"}]},"trigger":"minecraft:inventory_changed"},"has_the_recipe":{"conditions":{"recipes":"tconstruct:common/glass/vanilla/glass_bottle"},"trigger":"minecraft:recipe_unlocked"}},"requirements":[["has_glass","has_the_recipe"]],"rewards":{"recipes":["tconstruct:common/glass/vanilla/glass_bottle"]},"sends_telemetry_event":false}"##),
    ("recipe/tconstruct:common/glass/vanilla/beacon", "advancement/tconstruct:recipes/misc/common/glass/vanilla/beacon", br##"{"parent":"minecraft:recipes/root","criteria":{"has_nether_star":{"conditions":{"items":[{"items":["minecraft:nether_star"]}]},"trigger":"minecraft:inventory_changed"},"has_the_recipe":{"conditions":{"recipes":"tconstruct:common/glass/vanilla/beacon"},"trigger":"minecraft:recipe_unlocked"}},"requirements":[["has_nether_star","has_the_recipe"]],"rewards":{"recipes":["tconstruct:common/glass/vanilla/beacon"]},"sends_telemetry_event":false}"##),
    ("recipe/tconstruct:common/glass/vanilla/daylight_detector", "advancement/tconstruct:recipes/redstone/common/glass/vanilla/daylight_detector", br##"{"parent":"minecraft:recipes/root","criteria":{"has_quartz":{"conditions":{"items":[{"items":["minecraft:quartz"]}]},"trigger":"minecraft:inventory_changed"},"has_the_recipe":{"conditions":{"recipes":"tconstruct:common/glass/vanilla/daylight_detector"},"trigger":"minecraft:recipe_unlocked"}},"requirements":[["has_quartz","has_the_recipe"]],"rewards":{"recipes":["tconstruct:common/glass/vanilla/daylight_detector"]},"sends_telemetry_event":false}"##),
    ("recipe/tconstruct:common/glass/vanilla/end_crystal", "advancement/tconstruct:recipes/decorations/common/glass/vanilla/end_crystal", br##"{"parent":"minecraft:recipes/root","criteria":{"has_ender_eye":{"conditions":{"items":[{"items":["minecraft:ender_eye"]}]},"trigger":"minecraft:inventory_changed"},"has_the_recipe":{"conditions":{"recipes":"tconstruct:common/glass/vanilla/end_crystal"},"trigger":"minecraft:recipe_unlocked"}},"requirements":[["has_ender_eye","has_the_recipe"]],"rewards":{"recipes":["tconstruct:common/glass/vanilla/end_crystal"]},"sends_telemetry_event":false}"##),
];

pub(super) fn contribute(game_root: &std::path::Path, files: &mut Vec<(String, Vec<u8>)>) {
    let Ok(config) = tconstruct_source_config::load(game_root) else { return; };
    if config.get("glass_recipe_fix") != Some(&true) { return; }
    for &(path, recipe) in RECIPES {
        // Preserve explicit earlier contributions; do not replace resource or world-pack data.
        if !files.iter().any(|(existing, _)| existing == path) {
            files.push((path.to_owned(), recipe.to_vec()));
            // Only attach our discovery when we supplied this recipe. Respect
            // an earlier advancement contributor independently as well.
            if let Some(&(_, advancement_path, advancement)) = ADVANCEMENTS.iter().find(|entry| entry.0 == path) {
                if !files.iter().any(|(existing, _)| existing == advancement_path) {
                    files.push((advancement_path.to_owned(), advancement.to_vec()));
                }
            }
        }
    }
}
