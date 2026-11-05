// Contribute only this source config-gated ordinary recipe. The host asks for
// fresh files before building each world's tables and again on every reload.
pub(super) const PATH: &str = "recipe/tconstruct:common/flint";
const RECIPE: &[u8] = br#"{"type":"minecraft:crafting_shapeless","category":"misc","ingredients":["minecraft:gravel","minecraft:gravel","minecraft:gravel"],"result":{"id":"minecraft:flint","count":1}}"#;

pub(super) fn contribute(game_root: &std::path::Path, files: &mut Vec<(String, Vec<u8>)>) {
    // Respect an explicit contribution by another hook; world datapacks are
    // applied above these files by the host. Never modify resource files.
    if files.iter().any(|(path, _)| path == PATH) { return; }
    let Ok(config) = tconstruct_source_config::load(game_root) else { return; };
    if config.get("gravel_to_flint") == Some(&true) {
        files.push((PATH.to_owned(), RECIPE.to_vec()));
    }
}
