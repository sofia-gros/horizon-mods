//! Bounded reader for the original TConstruct common-config condition keys.
//! Unsupported settings do not change known conditions. Invalid supported
//! values are omitted so only the affected condition remains unknown.
use std::collections::HashMap;

const FIELDS: &[(&str, &str, &str, bool)] = &[
    ("gameplay", "shouldSpawnWithTinkersBook", "spawn_with_book", true),
    ("gameplay", "syncKnockbackResistance", "sync_knockback_resistance", true),
    ("recipes", "addGravelToFlintRecipe", "gravel_to_flint", true),
    ("recipes", "cheaperNetheriteAlloy", "cheaper_netherite_alloy", true),
    ("recipes", "witherBoneDrop", "wither_bone_drop", true),
    ("recipes", "glassRecipeFix", "glass_recipe_fix", true),
    ("compatability", "allowIngotlessAlloys", "allow_ingotless_alloys", true),
    ("debug", "forceIntegrationMaterials", "force_integration_materials", false),
    ("loot", "slimy_loot", "slimy_loot_chests", true),
];

pub fn parse(input: &str) -> Result<HashMap<String, bool>, String> {
    let root: toml::Table = toml::from_str(input).map_err(|e: toml::de::Error| e.to_string())?;
    let mut values = HashMap::new();
    for &(section, key, prop, default) in FIELDS {
        let value = match root.get(section) {
            None => Some(default),
            Some(table) => table.as_table().and_then(|table| match table.get(key) {
                None => Some(default),
                Some(value) => value.as_bool(),
            }),
        };
        if let Some(value) = value { values.insert(prop.to_owned(), value); }
    }
    // Source's deprecated condition is hardcoded false; the original maps
    // slime_recipe_fix to glassRecipeFix (not the separate slimeRecipeFix key).
    values.insert("wither_bone_conversion".into(), false);
    if let Some(&value) = values.get("glass_recipe_fix") { values.insert("slime_recipe_fix".into(), value); }
    Ok(values)
}

/// Dedicated servers keep config beside server.properties in the process
/// working directory, regardless of --universe. Single-player worlds live
/// under the game root's saves directory.
pub fn root_for_world(world_root: &std::path::Path, dedicated: bool, working_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    if dedicated { Some(working_dir.to_owned()) }
    else { world_root.parent()?.parent().map(std::path::Path::to_path_buf) }
}

/// Load the supported conditions from the game root without treating a broken
/// symlink or unreadable file as absent.
pub fn load(game_root: &std::path::Path) -> Result<HashMap<String, bool>, String> {
    let path = game_root.join("config/tconstruct-common.toml");
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return parse(""),
        Err(error) => return Err(format!("cannot inspect {}: {error}", path.display())),
        Ok(_) => {},
    }
    let text = std::fs::read_to_string(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    parse(&text).map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dedicated_root_is_cwd_not_world_or_universe_parent() {
        use std::path::{Path, PathBuf};
        assert_eq!(root_for_world(Path::new("/game/saves/example"), false, Path::new("/other")), Some(PathBuf::from("/game")));
        assert_eq!(root_for_world(Path::new("/server/world"), true, Path::new("/server")), Some(PathBuf::from("/server")));
        assert_eq!(root_for_world(Path::new("/data/universe/world"), true, Path::new("/server")), Some(PathBuf::from("/server")));
        assert_eq!(root_for_world(Path::new("/"), false, Path::new("/server")), None);
    }
    #[test]
    fn loads_missing_then_changed_and_invalid_file() {
        let root = std::env::temp_dir().join(format!("tconstruct-source-config-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        assert!(!load(&root).unwrap()["force_integration_materials"]);
        std::fs::create_dir(root.join("config")).unwrap();
        let path = root.join("config/tconstruct-common.toml");
        std::fs::write(&path, "[debug]\nforceIntegrationMaterials = true\n").unwrap();
        assert!(load(&root).unwrap()["force_integration_materials"]);
        std::fs::write(&path, "[recipes]\nglassRecipeFix = false\n").unwrap();
        let updated = load(&root).unwrap();
        assert!(!updated["glass_recipe_fix"] && !updated["slime_recipe_fix"]);
        std::fs::write(&path, "[recipes]\nglassRecipeFix = 1\n").unwrap();
        let invalid = load(&root).unwrap();
        assert!(!invalid.contains_key("glass_recipe_fix") && invalid["spawn_with_book"]);
        std::fs::write(&path, "[recipes\n").unwrap();
        assert!(load(&root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn defaults_and_original_alias() {
        let values = parse("").unwrap();
        assert!(values["spawn_with_book"] && values["glass_recipe_fix"] && values["slime_recipe_fix"]);
        assert!(!values["force_integration_materials"] && !values["wither_bone_conversion"]);
    }
    #[test]
    fn reads_forge_sections_and_preserves_source_alias() {
        let values = parse("[gameplay]\nshouldSpawnWithTinkersBook = false\n[recipes]\nglassRecipeFix = false\n[compatability]\nallowIngotlessAlloys = false\n[debug]\nforceIntegrationMaterials = true\n[loot]\nslimy_loot = false\n").unwrap();
        for key in ["spawn_with_book", "glass_recipe_fix", "slime_recipe_fix", "allow_ingotless_alloys", "slimy_loot_chests"] { assert!(!values[key]); }
        assert!(values["force_integration_materials"]);
    }
    #[test]
    fn each_supported_field_uses_its_exact_section_and_key() {
        for &(section, key, property, default) in FIELDS {
            let text = format!("[{section}]\n{key} = {}\n", !default);
            let values = parse(&text).unwrap();
            assert_eq!(values[property], !default, "{section}.{key}");
            let wrong_section = format!("[unrelated]\n{key} = {}\n", !default);
            assert_eq!(parse(&wrong_section).unwrap()[property], default);
        }
    }
    #[test]
    fn unknown_and_invalid_fields_are_isolated() {
        for input in ["[recipes\n", "[debug]\nforceIntegrationMaterials = false\nforceIntegrationMaterials = true"] {
            assert!(parse(input).is_err(), "{input}");
        }
        let values = parse("[unrelated]\nvalue = [1, 2]\n[recipes]\nglassRecipeFix = 1\nslimeRecipeFix = true\ncheaperNetheriteAlloy = false\n").unwrap();
        assert!(!values.contains_key("glass_recipe_fix"));
        assert!(!values.contains_key("slime_recipe_fix"));
        assert!(!values["cheaper_netherite_alloy"]);
        assert!(values["spawn_with_book"]);
        let values = parse("recipes = true").unwrap();
        assert!(!values.contains_key("glass_recipe_fix"));
        assert!(!values.contains_key("cheaper_netherite_alloy"));
        assert!(values["spawn_with_book"]);
    }
}
