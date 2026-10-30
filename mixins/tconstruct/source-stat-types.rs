// Pinned built-in material stat loaders. Unsupported numeric encodings and
// unknown stat types fail closed until the source loaders are fully ported.
use std::collections::{HashMap, HashSet};
use serde_json::{Map, Value};

pub(super) struct TierRegistry<'a> {
    pub names: &'a HashSet<String>,
    /// False if other Mixins may have registered additional Forge tiers.
    pub complete: bool,
}

/// A present result is a valid source stat; the bool says whether its type
/// implements IRepairableMaterialStats. None means the source drops that stat.
/// Tier IDs must come from a complete Forge tier registry for this world.
pub(super) fn validate_builtin_stat(
    id: &str, fields: &Map<String, Value>, tiers: Option<&TierRegistry<'_>>,
) -> Result<Option<bool>, &'static str> {
    let int = |key: &str, required: bool, min: i64| -> Result<bool, &'static str> {
        match fields.get(key) {
            None | Some(Value::Null) if !required => Ok(true),
            None | Some(Value::Null) => Ok(false),
            Some(Value::Number(n)) => {
                let v = n.as_f64().ok_or("unsupported integer encoding")?.trunc();
                if !v.is_finite() || v < i32::MIN as f64 || v > i32::MAX as f64 { return Err("unsupported integer encoding"); }
                Ok((v as i64) >= min)
            },
            Some(_) => Ok(false),
        }
    };
    let float = |key: &str, nonnegative: bool| -> Result<bool, &'static str> {
        match fields.get(key) {
            None | Some(Value::Null) => Ok(true),
            Some(Value::Number(n)) => {
                let value = n.as_f64().ok_or("unsupported float encoding")? as f32;
                Ok(!value.is_nan() && (!nonnegative || value >= 0.0))
            },
            Some(_) => Ok(false),
        }
    };
    let check_floats = |keys: &[(&str, bool)]| -> Result<bool, &'static str> {
        for &(key, nonnegative) in keys { if !float(key, nonnegative)? { return Ok(false); } }
        Ok(true)
    };
    let Some(name) = id.strip_prefix("tconstruct:") else { return Err("unknown registered stat type"); };
    let (valid, repairable) = match name {
        "binding" | "bowstring" | "maille" | "shield_core" | "repair_kit" | "cuirass" | "arrow_head" | "arrow_shaft" | "fletching" => (true, false),
        "head" => {
            let tier = match fields.get("mining_tier") {
                None | Some(Value::Null) => "minecraft:wood".to_owned(),
                Some(Value::String(s)) => if s.contains(':') { s.clone() } else { format!("minecraft:{s}") },
                Some(_) => return Ok(None),
            };
            let tiers = tiers.ok_or("tier registry unavailable")?;
            if !tiers.names.contains(&tier) && !tiers.complete { return Err("tier registry incomplete"); }
            (int("durability", true, 1)? && check_floats(&[("mining_speed", true), ("melee_attack", true)])? && tiers.names.contains(&tier), true)
        },
        "handle" => (check_floats(&[("durability", false), ("mining_speed", false), ("melee_speed", false), ("melee_damage", false)])?, false),
        "grip" => (check_floats(&[("durability", false), ("accuracy", false), ("melee_damage", true)])?, false),
        "limb" => (int("durability", true, 1)? && check_floats(&[("draw_speed", false), ("velocity", false), ("accuracy", false)])?, true),
        "plating_helmet" | "plating_chestplate" | "plating_leggings" | "plating_boots" => (int("durability", true, 1)? && check_floats(&[("armor", true), ("toughness", true), ("knockback_resistance", true)])?, true),
        "plating_shield" => (int("durability", true, 1)? && check_floats(&[("toughness", true), ("knockback_resistance", true)])?, true),
        "ribcage" | "shell" | "laces" => (int("repair_amount", true, 1)?, true),
        "skull" => (int("durability", true, 1)? && int("armor", false, 0)?, true),
        "slime" => (int("durability", true, 1)? && int("overslime_capacity", true, 0)?, true),
        _ => return Err("unknown registered stat type"),
    };
    Ok(valid.then_some(repairable))
}

/// Validate merged fields without treating unknown registries or encodings as
/// absent. This is not used by gameplay until world tier/type registries exist.
pub(super) fn validate_builtin_stats(
    fields: &std::collections::BTreeMap<String, Map<String, Value>>,
    tiers: Option<&TierRegistry<'_>>,
) -> Result<HashMap<String, bool>, &'static str> {
    let mut result = HashMap::new();
    for (id, value) in fields {
        if let Some(repairable) = validate_builtin_stat(id, value, tiers)? {
            result.insert(id.clone(), repairable);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builtin_stats_validate_required_ranges_and_tiers() {
        let ids = ["minecraft:wood".into(), "minecraft:stone".into()].into_iter().collect();
        let tiers = TierRegistry { names: &ids, complete: true };
        let object = |v: Value| v.as_object().unwrap().clone();
        assert_eq!(validate_builtin_stat("tconstruct:head", &object(serde_json::json!({"durability":85,"mining_tier":"minecraft:stone","mining_speed":3.5})), Some(&tiers)), Ok(Some(true)));
        assert_eq!(validate_builtin_stat("tconstruct:head", &object(serde_json::json!({"durability":0})), Some(&tiers)), Ok(None));
        assert_eq!(validate_builtin_stat("tconstruct:head", &object(serde_json::json!({"durability":1.9})), Some(&tiers)), Ok(Some(true)));
        assert_eq!(validate_builtin_stat("tconstruct:head", &object(serde_json::json!({"durability":85})), None), Err("tier registry unavailable"));
        assert_eq!(validate_builtin_stat("tconstruct:head", &object(serde_json::json!({"durability":85,"mining_tier":"test:unknown"})), Some(&tiers)), Ok(None));
        let incomplete = TierRegistry { names: &ids, complete: false };
        assert_eq!(validate_builtin_stat("tconstruct:head", &object(serde_json::json!({"durability":85,"mining_tier":"test:unknown"})), Some(&incomplete)), Err("tier registry incomplete"));
        assert_eq!(validate_builtin_stat("tconstruct:binding", &Map::new(), Some(&tiers)), Ok(Some(false)));
        assert_eq!(validate_builtin_stat("tconstruct:grip", &object(serde_json::json!({"melee_damage":-1})), Some(&tiers)), Ok(None));
        assert_eq!(validate_builtin_stat("test:unknown", &Map::new(), Some(&tiers)), Err("unknown registered stat type"));
    }

    #[test]
    fn validates_all_pinned_material_stats_for_source_tiers() {
        use std::io::Read;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors()
            .map(|p| p.join("content/reference/tconstruct-materials.zip"))
            .find(|p| p.is_file()).expect("pinned material archive");
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let ids = ["wood", "stone", "iron", "gold", "diamond", "netherite"].into_iter()
            .map(|name| format!("minecraft:{name}")).collect();
        let tiers = TierRegistry { names: &ids, complete: false };
        let mut count = 0;
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).unwrap();
            if !entry.name().contains("/materials/stats/") { continue; }
            let name = entry.name().to_owned();
            let mut bytes = Vec::new(); entry.read_to_end(&mut bytes).unwrap();
            let merged = super::super::tconstruct_tx::source_merge_material_stats(&[bytes]).unwrap();
            let valid = validate_builtin_stats(&merged, Some(&tiers)).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(valid.len(), merged.len(), "{name}");
            count += 1;
        }
        assert_eq!(count, 99);
    }
}
