// Config-gated ordinary recipes contributed before each native world recipe load.
// Default glass tags come from the content package; datapacks retain final precedence.
pub(super) const RECIPES: &[(&str, &[u8])] = &[
    ("recipe/tconstruct:common/glass/vanilla/glass_bottle", br##"{"type":"minecraft:crafting_shaped","category":"misc","key":{"#":"#forge:glass/colorless"},"pattern":["# #"," # "],"result":{"id":"minecraft:glass_bottle","count":3},"show_notification":true}"##),
    ("recipe/tconstruct:common/glass/vanilla/beacon", br##"{"type":"minecraft:crafting_shaped","category":"misc","key":{"G":"#forge:glass/colorless","O":"minecraft:obsidian","S":"minecraft:nether_star"},"pattern":["GGG","GSG","OOO"],"result":{"id":"minecraft:beacon","count":1},"show_notification":true}"##),
    ("recipe/tconstruct:common/glass/vanilla/daylight_detector", br##"{"type":"minecraft:crafting_shaped","category":"redstone","key":{"G":"#forge:glass/colorless","Q":"minecraft:quartz","W":"#minecraft:wooden_slabs"},"pattern":["GGG","QQQ","WWW"],"result":{"id":"minecraft:daylight_detector","count":1},"show_notification":true}"##),
    ("recipe/tconstruct:common/glass/vanilla/end_crystal", br##"{"type":"minecraft:crafting_shaped","category":"misc","key":{"E":"minecraft:ender_eye","G":"#forge:glass/colorless","T":"minecraft:ghast_tear"},"pattern":["GGG","GEG","GTG"],"result":{"id":"minecraft:end_crystal","count":1},"show_notification":true}"##),
];

pub(super) fn contribute(game_root: &std::path::Path, files: &mut Vec<(String, Vec<u8>)>) {
    let Ok(config) = tconstruct_source_config::load(game_root) else { return; };
    if config.get("glass_recipe_fix") != Some(&true) { return; }
    for &(path, recipe) in RECIPES {
        // Preserve explicit earlier contributions; do not replace resource or world-pack data.
        if !files.iter().any(|(existing, _)| existing == path) {
            files.push((path.to_owned(), recipe.to_vec()));
        }
    }
}
