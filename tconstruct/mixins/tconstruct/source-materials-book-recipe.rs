use serde_json::Value;
use std::collections::{HashMap, HashSet};

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
