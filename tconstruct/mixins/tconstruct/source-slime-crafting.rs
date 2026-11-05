// The pinned source maps slime_recipe_fix to glassRecipeFix, not to the
// similarly named slimeRecipeFix setting. Read the existing parser each load.
pub(super) const RECIPES: &[(&str, &[u8])] = &[
    ("recipe/tconstruct:common/slime/lead", br##"{"type":"minecraft:crafting_shaped","category":"equipment","pattern":["~~ ","~O ","  ~"],"key":{"O":"#forge:slimeballs","~":"minecraft:string"},"result":{"id":"minecraft:lead","count":2}}"##),
    ("recipe/tconstruct:common/slime/sticky_piston", br##"{"type":"minecraft:crafting_shaped","category":"redstone","pattern":["#","P"],"key":{"#":"#forge:slimeballs","P":"minecraft:piston"},"result":{"id":"minecraft:sticky_piston","count":1}}"##),
];

// Key source discovery by native recipe, under the same per-world gate.
pub(super) const ADVANCEMENTS: &[(&str, &str, &[u8])] = &[
    ("recipe/tconstruct:common/slime/lead", "advancement/tconstruct:recipes/tools/common/slime/lead", br##"{"parent":"minecraft:recipes/root","criteria":{"has_slime_ball":{"conditions":{"items":[{"items":"#forge:slimeballs"}]},"trigger":"minecraft:inventory_changed"},"has_the_recipe":{"conditions":{"recipes":"tconstruct:common/slime/lead"},"trigger":"minecraft:recipe_unlocked"}},"requirements":[["has_slime_ball","has_the_recipe"]],"rewards":{"recipes":["tconstruct:common/slime/lead"]},"sends_telemetry_event":false}"##),
    ("recipe/tconstruct:common/slime/sticky_piston", "advancement/tconstruct:recipes/redstone/common/slime/sticky_piston", br##"{"parent":"minecraft:recipes/root","criteria":{"has_slime_ball":{"conditions":{"items":[{"items":"#forge:slimeballs"}]},"trigger":"minecraft:inventory_changed"},"has_the_recipe":{"conditions":{"recipes":"tconstruct:common/slime/sticky_piston"},"trigger":"minecraft:recipe_unlocked"}},"requirements":[["has_slime_ball","has_the_recipe"]],"rewards":{"recipes":["tconstruct:common/slime/sticky_piston"]},"sends_telemetry_event":false}"##),
];

pub(super) fn contribute(game_root: &std::path::Path, files: &mut Vec<(String, Vec<u8>)>) {
    let Ok(config) = tconstruct_source_config::load(game_root) else { return; };
    if config.get("slime_recipe_fix") != Some(&true) { return; }
    for &(path, bytes) in RECIPES {
        // Do not replace another hook's explicit contribution. The host puts
        // world datapacks above these contributions during table construction.
        if !files.iter().any(|(existing, _)| existing == path) {
            files.push((path.to_owned(), bytes.to_vec()));
            // Do not attach our discovery to another contributor's recipe.
            // Preserve an earlier same-ID advancement independently too.
            if let Some(&(_, advancement_path, advancement)) = ADVANCEMENTS.iter().find(|entry| entry.0 == path) {
                if !files.iter().any(|(existing, _)| existing == advancement_path) {
                    files.push((advancement_path.to_owned(), advancement.to_vec()));
                }
            }
        }
    }
}
