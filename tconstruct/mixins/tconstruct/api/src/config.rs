//! ConfigEnabledCondition's common boolean settings (SlimeKnights, MIT).
//! Other common/client settings are added with their consuming domains.
use pluto_lib::{ConfigEvents, ConfigKind, ConfigRegistry, ModConfig};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

// Source: common/config/Config.java and common/json/ConfigEnabledCondition.java.
const SETTINGS: &[(&str, &str, &str, bool)] = &[
    ("spawn_with_book", "gameplay", "shouldSpawnWithTinkersBook", true),
    ("sync_knockback_resistance", "gameplay", "syncKnockbackResistance", true),
    ("gravel_to_flint", "recipes", "addGravelToFlintRecipe", true),
    ("cheaper_netherite_alloy", "recipes", "cheaperNetheriteAlloy", true),
    ("wither_bone_drop", "recipes", "witherBoneDrop", true),
    ("glass_recipe_fix", "recipes", "glassRecipeFix", true),
    // The original condition deliberately references glassRecipeFix for BOTH names.
    ("slime_recipe_fix", "recipes", "glassRecipeFix", true),
    ("allow_ingotless_alloys", "compatability", "allowIngotlessAlloys", true),
    ("force_integration_materials", "debug", "forceIntegrationMaterials", false),
    ("slimy_loot_chests", "loot", "slimy_loot", true),
];

pub(crate) fn known(name: &str) -> bool {
    name == "wither_bone_conversion" || SETTINGS.iter().any(|(condition, _, _, _)| *condition == name)
}

pub(crate) fn default_text() -> String {
    let mut root = toml::map::Map::new();
    for &(_, section, key, default) in SETTINGS {
        let table = root.entry(section).or_insert_with(|| toml::Value::Table(Default::default()));
        table.as_table_mut().unwrap().insert(key.into(), toml::Value::Boolean(default));
    }
    toml::to_string(&root).expect("boolean config defaults serialize")
}

pub(crate) fn parse(text: &str) -> Result<BTreeMap<String, bool>, String> {
    let root: toml::Table = toml::from_str(text).map_err(|e| format!("invalid Tinkers common config: {e}"))?;
    let mut values = BTreeMap::new();
    for &(condition, section, key, default) in SETTINGS {
        let value = match root.get(section) {
            None => default,
            Some(table) => match table.as_table().ok_or_else(|| format!("{section} must be a table"))?.get(key) {
                None => default,
                Some(value) => value.as_bool().ok_or_else(|| format!("{section}.{key} must be boolean"))?,
            },
        };
        values.insert(condition.into(), value);
    }
    values.insert("wither_bone_conversion".into(), false);
    Ok(values)
}

type State = Arc<RwLock<Result<BTreeMap<String, bool>, String>>>;
pub(crate) fn register() -> State {
    let state = Arc::new(RwLock::new(Err("Tinkers common config has not loaded".into())));
    ConfigRegistry::register("tconstruct", ConfigKind::Common, &default_text());
    for event in [ConfigEvents::LOADING, ConfigEvents::RELOADING] {
        let output = state.clone();
        event.register(move |config: &ModConfig| {
            if config.mod_id == "tconstruct" && config.kind == ConfigKind::Common {
                let parsed = parse(&config.text);
                if let Err(error) = &parsed { eprintln!("Tinkers: {error}"); }
                *output.write().expect("Tinkers config lock poisoned") = parsed;
            }
        });
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_aliases_defaults_and_invalid_values() {
        let defaults = parse(&default_text()).unwrap();
        assert!(!defaults["force_integration_materials"]);
        assert!(!defaults["wither_bone_conversion"]);
        let changed = parse("[recipes]\nglassRecipeFix = false\nslimeRecipeFix = true\n[debug]\nforceIntegrationMaterials = true\n").unwrap();
        assert!(!changed["glass_recipe_fix"] && !changed["slime_recipe_fix"]);
        assert!(changed["force_integration_materials"] && changed["spawn_with_book"]);
        assert!(parse("[debug]\nforceIntegrationMaterials = 'true'\n").is_err());
        assert!(parse("debug = false").is_err());
    }
}
