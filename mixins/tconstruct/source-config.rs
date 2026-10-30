use std::collections::HashMap;
use std::path::Path;

/// Snapshot the original common-config defaults only when no config file is
/// present. A present file is rejected until Forge TOML and its exact source
/// semantics are implemented; guessing would silently change recipe rules.
pub(super) fn load(game_root: &Path) -> Result<HashMap<String, bool>, String> {
    let path = game_root.join("config/tconstruct-common.toml");
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
        Err(error) => return Err(format!("cannot inspect {}: {error}", path.display())),
        Ok(_) => return Err(format!("reading Forge config {} is not implemented", path.display())),
    }
    Ok([
        ("spawn_with_book", true),
        ("gravel_to_flint", true),
        ("cheaper_netherite_alloy", true),
        ("wither_bone_drop", true),
        ("wither_bone_conversion", false),
        ("glass_recipe_fix", true),
        ("allow_ingotless_alloys", true),
        ("force_integration_materials", false),
        ("slimy_loot_chests", true),
        ("sync_knockback_resistance", true),
    ].iter().map(|&(key, value)| (key.to_owned(), value)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_original_and_an_existing_config_is_not_guessed() {
        let root = std::env::temp_dir().join(format!("tconstruct-source-config-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let defaults = load(&root).unwrap();
        assert_eq!(defaults["force_integration_materials"], false);
        assert_eq!(defaults["allow_ingotless_alloys"], true);
        assert_eq!(defaults["glass_recipe_fix"], true);
        assert_eq!(defaults["wither_bone_conversion"], false);
        std::fs::create_dir(root.join("config")).unwrap();
        std::fs::write(root.join("config/tconstruct-common.toml"), "[debug]\nforceIntegrationMaterials = true\n").unwrap();
        assert!(load(&root).unwrap_err().contains("not implemented"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
