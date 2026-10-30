use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn add_traits(
    result: &mut BTreeMap<String, i32>,
    default: Option<&Vec<Value>>,
    per_stat: &BTreeMap<String, Vec<Value>>,
    kind: &str,
) {
    let stat = match kind {
        "head" | "handle" | "binding" | "limb" | "grip" | "bowstring" | "maille" => {
            format!("tconstruct:{kind}")
        }
        "plating_helmet" | "plating_chestplate" | "plating_leggings" | "plating_boots" => {
            "tconstruct:armor".into()
        }
        _ => return,
    };
    let fallback = match kind {
        "head" | "handle" | "binding" => Some("tconstruct:melee_harvest"),
        "limb" | "grip" | "bowstring" => Some("tconstruct:ranged"),
        _ => None,
    };
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
    }
}
