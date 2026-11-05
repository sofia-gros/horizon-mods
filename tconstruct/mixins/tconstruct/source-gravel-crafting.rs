// Contribute this source recipe and discovery together under the config gate.
// The host asks for fresh files on world load and each reload.
pub(super) const PATH: &str = "recipe/tconstruct:common/flint";
const RECIPE: &[u8] = br#"{"type":"minecraft:crafting_shapeless","category":"misc","ingredients":["minecraft:gravel","minecraft:gravel","minecraft:gravel"],"result":{"id":"minecraft:flint","count":1}}"#;

pub(super) const ADVANCEMENT_PATH: &str = "advancement/tconstruct:recipes/misc/common/flint";
const ADVANCEMENT: &[u8] = br#"{"parent":"minecraft:recipes/root","criteria":{"has_item":{"conditions":{"items":[{"items":["minecraft:gravel"]}]},"trigger":"minecraft:inventory_changed"},"has_the_recipe":{"conditions":{"recipes":"tconstruct:common/flint"},"trigger":"minecraft:recipe_unlocked"}},"requirements":[["has_item","has_the_recipe"]],"rewards":{"recipes":["tconstruct:common/flint"]},"sends_telemetry_event":false}"#;

pub(super) fn contribute(game_root: &std::path::Path, files: &mut Vec<(String, Vec<u8>)>) {
    // Respect an explicit contribution by another hook; world datapacks are
    // applied above these files by the host. If another hook owns the recipe,
    // do not attach our discovery to its replacement. Never modify resources.
    if files.iter().any(|(path, _)| path == PATH) { return; }
    let Ok(config) = tconstruct_source_config::load(game_root) else { return; };
    if config.get("gravel_to_flint") == Some(&true) {
        files.push((PATH.to_owned(), RECIPE.to_vec()));
        // Do not replace an advancement supplied by another native hook.
        if !files.iter().any(|(path, _)| path == ADVANCEMENT_PATH) {
            files.push((ADVANCEMENT_PATH.to_owned(), ADVANCEMENT.to_vec()));
        }
    }
}
