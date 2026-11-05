use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(super) fn showcase_item(value: &Value) -> Option<&str> {
    let id = value.as_str().or_else(|| value.get("item").and_then(Value::as_str))?;
    let (namespace, path) = id.split_once(':')?;
    if namespace.is_empty() || path.is_empty()
        || !namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
        || !path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c)) { return None; }
    Some(id)
}

// Tag-only showcases use this world's already-resolved registry, never a
// process-wide/default tag list. Bound metadata below the book wire span limit.
pub(super) fn showcase_tag(value: &Value, tags: Option<&HashMap<String, HashSet<String>>>, known: Option<&HashSet<String>>) -> Option<Value> {
    let object = value.as_object()?;
    if object.len() != 1 { return None; }
    let tag = object.get("tag")?.as_str()?;
    showcase_item(&Value::String(tag.to_owned()))?;
    let known = known?;
    let members = tags?.get(tag)?;
    let mut choices: Vec<_> = members.iter().filter(|id| id.as_str() != "minecraft:air" && known.contains(*id)
        && showcase_item(&Value::String((*id).clone())).is_some()).cloned().collect();
    choices.sort();
    if choices.is_empty() || choices.len() > 128
        || choices.iter().map(|id| id.len() + 3).sum::<usize>() > 4096 { return None; }
    Some(serde_json::json!({"showcase_items":choices}))
}

// Use the same selected ID for drawing and the hover tooltip. Source Mantle
// switches every three seconds; this renderer has a shared wall-clock phase.
pub(super) fn showcase_display_item(spec: &Value, seconds: u64) -> Option<&str> {
    if let Some(value) = spec.get("showcase_item") { return showcase_item(value); }
    let choices = spec.get("showcase_items")?.as_array()?;
    if choices.is_empty() || choices.len() > 128 { return None; }
    showcase_item(&choices[((seconds / 3) % choices.len() as u64) as usize])
}

pub(super) fn resolve(recipe: &Value, tags: Option<&HashMap<String, HashSet<String>>>, grid_size: &str) -> Option<Value> {
    fn ingredient(value: &serde_json::Value, tags: Option<&std::collections::HashMap<String, std::collections::HashSet<String>>>) -> Vec<String> {
        if let Some(choices) = value.as_array() {
            let mut result: Vec<_> = choices.iter().flat_map(|v| ingredient(v, tags)).collect();
            result.sort(); result.dedup(); return result;
        }
        let id = value.as_str().or_else(|| value.get("item").and_then(|v| v.as_str()));
        let tag = id.and_then(|id| id.strip_prefix('#')).or_else(|| value.get("tag").and_then(|v| v.as_str()));
        if let Some(tag) = tag {
            let mut entries: Vec<_> = tags.and_then(|tags| tags.get(tag)).into_iter().flatten().cloned().collect();
            entries.sort(); return entries;
        }
        id.map(|s| vec![s.to_owned()]).unwrap_or_default()
    }
    let grid = if grid_size == "small" { 2 } else { 3 };
    let mut slots: Vec<Vec<String>> = vec![Vec::new(); 9];
    if let (Some(pattern), Some(keys)) = (recipe.get("pattern").and_then(|v| v.as_array()), recipe.get("key").and_then(|v| v.as_object())) {
        for (y, line) in pattern.iter().take(3).enumerate() {
            for (x, ch) in line.as_str()?.chars().take(3).enumerate() {
                if ch != ' ' { slots[y * 3 + x] = keys.get(&ch.to_string()).map(|v| ingredient(v, tags)).unwrap_or_default(); }
            }
        }
    } else {
        for (i, value) in recipe.get("ingredients")?.as_array()?.iter().take(grid * grid).enumerate() {
            slots[(i / grid) * 3 + i % grid] = ingredient(value, tags);
        }
    }
    let result = recipe.get("result").and_then(|v| v.get("id").or_else(|| v.get("item"))).and_then(|v| v.as_str());
    let count = recipe.get("result").and_then(|v| v.get("count")).and_then(|v| v.as_u64()).unwrap_or(1).min(u32::MAX as u64);
    Some(serde_json::json!({"slots": slots, "result": result, "count":count, "grid": if grid_size == "small" { 2 } else { 3 }}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn showcase_tags_use_each_world_and_only_registered_valid_items() {
        let input = serde_json::json!({"tag":"tconstruct:seared_blocks"});
        let tags = HashMap::from([("tconstruct:seared_blocks".into(), HashSet::from([
            "tconstruct:seared_stone".into(), "tconstruct:seared_bricks".into(),
            "minecraft:air".into(), "missing:item".into(), "invalid id".into()]))]);
        let known = HashSet::from(["tconstruct:seared_bricks".into(), "tconstruct:seared_stone".into(), "minecraft:air".into(), "invalid id".into()]);
        let actual = showcase_tag(&input, Some(&tags), Some(&known)).unwrap();
        assert_eq!(actual, serde_json::json!({"showcase_items":["tconstruct:seared_bricks","tconstruct:seared_stone"]}));
        let other = HashMap::from([("tconstruct:seared_blocks".into(), HashSet::from(["tconstruct:seared_stone".into()]))]);
        assert_eq!(showcase_tag(&input, Some(&other), Some(&known)).unwrap()["showcase_items"], serde_json::json!(["tconstruct:seared_stone"]));
        assert_eq!(showcase_tag(&input, Some(&tags), Some(&known)), Some(actual));
        for (tags, known) in [(None, Some(&known)), (Some(&tags), None), (Some(&HashMap::new()), Some(&known)), (Some(&tags), Some(&HashSet::new()))] {
            assert!(showcase_tag(&input, tags, known).is_none());
        }
    }
    #[test]
    fn showcase_tag_rejects_unsupported_forms_and_bounds_metadata() {
        let members: HashSet<_> = (0..129).map(|n| format!("test:item_{n}")).collect();
        let tags = HashMap::from([("test:items".into(), members.clone())]);
        for input in [serde_json::json!({"tag":"test:items"}), serde_json::json!({"tag":42}),
            serde_json::json!({"tag":"bad tag"}), serde_json::json!({"tag":"test:items", "item":"test:item_0"}),
            serde_json::json!({"tag":"test:items", "nbt":{}}), serde_json::json!({"tag":"test:items", "amount_needed":2})] {
            assert!(showcase_tag(&input, Some(&tags), Some(&members)).is_none());
        }
        let members = HashSet::from([format!("test:{}", "x".repeat(4096))]);
        let tags = HashMap::from([("test:items".into(), members.clone())]);
        assert!(showcase_tag(&serde_json::json!({"tag":"test:items"}), Some(&tags), Some(&members)).is_none());
    }
    #[test]
    fn showcase_cycles_every_three_seconds_and_safely_wraps() {
        let spec = serde_json::json!({"showcase_items":["test:a", "test:b"]});
        for seconds in [0, 1, 2, 6, 7, 8] { assert_eq!(showcase_display_item(&spec, seconds), Some("test:a")); }
        for seconds in [3, 4, 5, 9, 10, 11] { assert_eq!(showcase_display_item(&spec, seconds), Some("test:b")); }
        assert_eq!(showcase_display_item(&spec, u64::MAX), Some("test:b"));
        assert_eq!(showcase_display_item(&serde_json::json!({"showcase_item":"test:single"}), u64::MAX), Some("test:single"));
        for spec in [serde_json::json!({"showcase_items":[]}), serde_json::json!({"showcase_items":[42]}), serde_json::json!({"showcase_items":["bad id"]}), serde_json::json!({"showcase_items":42})] {
            assert!(showcase_display_item(&spec, 0).is_none());
        }
    }
    #[test]
    fn showcase_accepts_source_item_forms_and_rejects_invalid_values() {
        for id in ["tconstruct:copper_gauge", "tconstruct:seared_fluid_cannon", "tconstruct:seared_casting_tank"] {
            assert_eq!(showcase_item(&serde_json::json!(id)), Some(id));
            assert_eq!(showcase_item(&serde_json::json!({"item":id})), Some(id));
        }
        for value in [serde_json::Value::Null, serde_json::json!({}), serde_json::json!({"item":42}), serde_json::json!(""), serde_json::json!("tconstruct:"), serde_json::json!("no namespace"), serde_json::json!("tconstruct:bad id")] {
            assert_eq!(showcase_item(&value), None);
        }
    }
    #[test]
    fn preserves_tag_candidates_and_output_count() {
        let tags = HashMap::from([("tconstruct:planklike".to_owned(), ["minecraft:birch_planks".to_owned(), "minecraft:oak_planks".to_owned()].into_iter().collect())]);
        let source = serde_json::json!({"pattern":["pw"], "key":{"p":{"item":"tconstruct:pattern"},"w":{"tag":"tconstruct:planklike"}},"result":{"item":"tconstruct:part_builder","count":4}});
        let actual = resolve(&source, Some(&tags), "small").unwrap();
        assert_eq!(actual["slots"][0], serde_json::json!(["tconstruct:pattern"]));
        assert_eq!(actual["slots"][1], serde_json::json!(["minecraft:birch_planks", "minecraft:oak_planks"]));
        assert_eq!(actual["count"], 4);
        assert_eq!(actual["grid"], 2);
    }
    #[test]
    fn shapeless_small_grid_places_third_input_on_second_row() {
        let source = serde_json::json!({"ingredients":["minecraft:a","minecraft:b","minecraft:c"],"result":{"id":"minecraft:book"}});
        let actual = resolve(&source, None, "small").unwrap();
        assert_eq!(actual["slots"][3], serde_json::json!(["minecraft:c"]));
        assert_eq!(actual["count"], 1);
    }
}
