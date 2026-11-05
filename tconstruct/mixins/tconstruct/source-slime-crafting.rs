// The pinned source maps slime_recipe_fix to glassRecipeFix, not to the
// similarly named slimeRecipeFix setting. Read the existing parser each load.
const RECIPES: &[(&str, &[u8])] = &[
    ("recipe/tconstruct:common/slime/lead", br##"{"type":"minecraft:crafting_shaped","category":"equipment","pattern":["~~ ","~O ","  ~"],"key":{"O":"#forge:slimeballs","~":"minecraft:string"},"result":{"id":"minecraft:lead","count":2}}"##),
    ("recipe/tconstruct:common/slime/sticky_piston", br##"{"type":"minecraft:crafting_shaped","category":"redstone","pattern":["#","P"],"key":{"#":"#forge:slimeballs","P":"minecraft:piston"},"result":{"id":"minecraft:sticky_piston","count":1}}"##),
];

pub(super) fn contribute(game_root: &std::path::Path, files: &mut Vec<(String, Vec<u8>)>) {
    let Ok(config) = tconstruct_source_config::load(game_root) else { return; };
    if config.get("slime_recipe_fix") != Some(&true) { return; }
    for &(path, bytes) in RECIPES {
        // Do not replace another hook's explicit contribution. The host puts
        // world datapacks above these contributions during table construction.
        if !files.iter().any(|(existing, _)| existing == path) {
            files.push((path.to_owned(), bytes.to_vec()));
        }
    }
}
