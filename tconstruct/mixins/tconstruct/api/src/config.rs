//! ConfigEnabledCondition's common boolean settings (SlimeKnights, MIT).
//! Other common/client settings are added with their consuming domains.
use pluto_lib::{ConfigEvents, ConfigKind, ConfigRegistry, ModConfig};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock};

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

/// Config.repairKitAmount: durability of a repair kit in ingots, 2 by default, 0..=32767.
const REPAIR_KIT_AMOUNT: (&str, &str, f64) = ("gameplay", "repairKitAmount", 2.0);
static REPAIR_KIT: Mutex<Option<Result<f32, String>>> = Mutex::new(None);

fn parse_repair_kit_amount(text: &str) -> Result<f32, String> {
    let root: toml::Table = toml::from_str(text).map_err(|e| format!("invalid Tinkers common config: {e}"))?;
    let (section, key, default) = REPAIR_KIT_AMOUNT;
    let value = match root.get(section) {
        None => default,
        Some(table) => match table.as_table().ok_or_else(|| format!("{section} must be a table"))?.get(key) {
            None => default,
            Some(toml::Value::Float(value)) => *value,
            // TOML integers are accepted for a double setting, as Forge's config coerces them.
            Some(toml::Value::Integer(value)) => *value as f64,
            Some(_) => return Err(format!("{section}.{key} must be a number")),
        },
    };
    if !(0.0..=f64::from(i16::MAX)).contains(&value) { return Err(format!("{section}.{key} must be within 0..=32767")); }
    Ok(value as f32)
}

/// The loaded repair kit amount; an error (never the default) when the config is missing or invalid.
pub(crate) fn repair_kit_amount() -> Result<f32, String> {
    REPAIR_KIT.lock().expect("repair kit lock poisoned").clone().unwrap_or_else(|| Err("Tinkers common config has not loaded".into()))
}

pub(crate) fn known(name: &str) -> bool {
    name == "wither_bone_conversion" || SETTINGS.iter().any(|(condition, _, _, _)| *condition == name)
}

pub(crate) fn default_text() -> String {
    let mut root = toml::map::Map::new();
    for &(_, section, key, default) in SETTINGS {
        let table = root.entry(section).or_insert_with(|| toml::Value::Table(Default::default()));
        table.as_table_mut().unwrap().insert(key.into(), toml::Value::Boolean(default));
    }
    let (section, key, default) = REPAIR_KIT_AMOUNT;
    root.entry(section).or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut().unwrap().insert(key.into(), toml::Value::Float(default));
    toml::to_string(&root).expect("config defaults serialize")
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
                *REPAIR_KIT.lock().expect("repair kit lock poisoned") = Some(parse_repair_kit_amount(&config.text));
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
        assert_eq!(parse_repair_kit_amount(&default_text()).unwrap(), 2.0, "the written default round-trips");
        assert!(!defaults["force_integration_materials"]);
        assert!(!defaults["wither_bone_conversion"]);
        let changed = parse("[recipes]\nglassRecipeFix = false\nslimeRecipeFix = true\n[debug]\nforceIntegrationMaterials = true\n").unwrap();
        assert!(!changed["glass_recipe_fix"] && !changed["slime_recipe_fix"]);
        assert!(changed["force_integration_materials"] && changed["spawn_with_book"]);
        assert!(parse("[debug]\nforceIntegrationMaterials = 'true'\n").is_err());
        assert!(parse("debug = false").is_err());
    }
    #[test]
    fn repair_kit_amount_defaults_to_two_and_rejects_out_of_range_or_non_numbers() {
        assert_eq!(parse_repair_kit_amount("").unwrap(), 2.0);
        assert_eq!(parse_repair_kit_amount("[gameplay]\nrepairKitAmount = 3.5\n").unwrap(), 3.5);
        assert_eq!(parse_repair_kit_amount("[gameplay]\nrepairKitAmount = 0\n").unwrap(), 0.0);
        for bad in ["[gameplay]\nrepairKitAmount = -1.0\n", "[gameplay]\nrepairKitAmount = 40000\n",
                    "[gameplay]\nrepairKitAmount = 'two'\n", "gameplay = 1"] {
            assert!(parse_repair_kit_amount(bad).is_err(), "{bad}");
        }
    }
}
