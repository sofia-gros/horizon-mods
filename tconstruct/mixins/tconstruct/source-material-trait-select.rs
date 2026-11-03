use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn add_traits(
    result: &mut BTreeMap<String, i32>,
    default: Option<&Vec<Value>>,
    per_stat: &BTreeMap<String, Vec<Value>>,
    kind: &str,
) {
    add_traits_ordered(result, &mut Vec::new(), default, per_stat, kind);
}

/// Preserve first occurrence, matching the source's stable LinkedHashMap order.
pub(super) fn add_traits_ordered(
    result: &mut BTreeMap<String, i32>,
    order: &mut Vec<String>,
    default: Option<&Vec<Value>>,
    per_stat: &BTreeMap<String, Vec<Value>>,
    kind: &str,
) {
    // The source registry first checks the exact stat type and then its
    // registered trait fallback, before using the material's default traits.
    let fallback = match kind {
        "head" | "handle" | "binding" => Some("tconstruct:melee_harvest"),
        "limb" | "grip" | "bowstring" => Some("tconstruct:ranged"),
        "plating_helmet" | "plating_chestplate" | "plating_leggings" | "plating_boots"
        | "plating_shield" | "maille" | "shield_core" | "cuirass" | "slime" | "skull"
        | "shell" | "laces" => Some("tconstruct:armor"),
        "arrow_head" | "arrow_shaft" | "fletching" => Some("tconstruct:ammo"),
        // Ribcage deliberately has no armor fallback in MaterialRegistry.
        "ribcage" | "repair_kit" => None,
        _ => return,
    };
    let stat = format!("tconstruct:{kind}");
    let selected = per_stat
        .get(&stat)
        .or_else(|| fallback.and_then(|id| per_stat.get(id)))
        .or(default);
    for entry in selected.into_iter().flatten() {
        let Some(name) = entry.get("name").and_then(Value::as_str) else {
            continue;
        };
        let Some(level) = entry
            .get("level")
            .and_then(Value::as_i64)
            .and_then(|n| i32::try_from(n).ok())
            .filter(|n| *n > 0)
        else {
            continue;
        };
        if !result.contains_key(name) { order.push(name.to_owned()); }
        *result.entry(name.to_owned()).or_insert(0) += level;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selects_stat_fallback_and_sums_repeated_material_traits() {
        let default = vec![serde_json::json!({"name":"tconstruct:cultivated","level":1})];
        let mut per_stat = BTreeMap::new();
        per_stat.insert(
            "tconstruct:ranged".into(),
            vec![serde_json::json!({"name":"tconstruct:holy","level":1})],
        );
        per_stat.insert(
            "tconstruct:head".into(),
            vec![serde_json::json!({"name":"tconstruct:jagged","level":2})],
        );
        let mut result = BTreeMap::new();
        add_traits(&mut result, Some(&default), &per_stat, "head");
        add_traits(&mut result, Some(&default), &per_stat, "handle");
        add_traits(&mut result, Some(&default), &per_stat, "binding");
        add_traits(&mut result, Some(&default), &per_stat, "limb");
        assert_eq!(result["tconstruct:jagged"], 2);
        assert_eq!(result["tconstruct:cultivated"], 2);
        assert_eq!(result["tconstruct:holy"], 1);
        let mut levels = BTreeMap::new();
        let mut order = Vec::new();
        add_traits_ordered(&mut levels, &mut order, Some(&default), &per_stat, "head");
        add_traits_ordered(&mut levels, &mut order, Some(&default), &per_stat, "handle");
        add_traits_ordered(&mut levels, &mut order, Some(&default), &per_stat, "binding");
        assert_eq!(order, ["tconstruct:jagged", "tconstruct:cultivated"]);
        assert_eq!(levels["tconstruct:cultivated"], 2);
    }
}
