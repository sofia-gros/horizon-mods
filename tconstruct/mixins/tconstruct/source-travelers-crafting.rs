// Compile the five source Travelers recipes into ordinary native shaped
// recipes. Each variant fixes both material identities; the native crafting
// engine remains responsible for preview, consumption, shift craft and remainders.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug)]
pub(super) struct MaterialItem {
    pub item: String,
    pub material: String,
    pub value: i32,
    pub needed: i32,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Plan {
    pub result: String,
    pub materials: Vec<String>,
    pub category: String,
    pub group: String,
    pub show_notification: bool,
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Option<Vec<String>>>,
}
pub(super) const RESULTS: [&str; 5] = [
    "tconstruct:travelers_helmet",
    "tconstruct:travelers_chestplate",
    "tconstruct:travelers_leggings",
    "tconstruct:travelers_boots",
    "tconstruct:travelers_shield",
];

fn material_key(
    value: &Value,
    item: &MaterialItem,
    allowed: &impl Fn(&Value, &str) -> bool,
) -> Option<bool> {
    if let Some(values) = value.as_array() {
        let mut matched = false;
        for value in values {
            matched |= material_key(value, item, allowed)?;
        }
        return Some(matched);
    }
    let obj = value.as_object()?;
    match obj.get("type")?.as_str()? {
        "tconstruct:material_value" => {
            if obj
                .keys()
                .any(|key| !["type", "material", "value"].contains(&key.as_str()))
            {
                return None;
            }
            let value = obj.get("value")?.as_f64()?;
            // This slice accepts exact-value material ingredients, as all five
            // source recipes use. Blocks/nuggets cannot substitute for one unit.
            Some(
                value.is_finite()
                    && (value as f32).is_finite()
                    && value > 0.0
                    && item.needed > 0
                    && item.value as f32 / item.needed as f32 == value as f32
                    && allowed(obj.get("material")?, &item.material),
            )
        }
        // Fake ingots store their material in a component, not their item ID.
        // They must never become an unrestricted ordinary ingredient here.
        "tconstruct:material" if obj.get("item")?.as_str()? == "tconstruct:fake_ingot" => {
            if obj
                .keys()
                .any(|key| !["type", "item", "material"].contains(&key.as_str()))
                || !obj.contains_key("material")
            {
                return None;
            }
            Some(false)
        }
        _ => None,
    }
}
fn ordinary(
    value: &Value,
    tags: &HashMap<String, HashSet<String>>,
    known: &HashSet<String>,
) -> Option<Vec<String>> {
    let obj = value.as_object()?;
    if obj.len() != 1 {
        return None;
    }
    let values: BTreeSet<String> = if let Some(id) = obj.get("item") {
        [id.as_str()?.to_owned()].into_iter().collect()
    } else {
        tags.get(obj.get("tag")?.as_str()?)?
            .iter()
            .cloned()
            .collect()
    };
    let values: Vec<_> = values.into_iter().filter(|id| known.contains(id)).collect();
    (!values.is_empty()).then_some(values)
}
pub(super) fn compile(
    bytes: &[u8],
    candidates: &[MaterialItem],
    tags: &HashMap<String, HashSet<String>>,
    known: &HashSet<String>,
    allowed: impl Fn(&Value, &str) -> bool,
) -> Option<Vec<Plan>> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let obj = value.as_object()?;
    if obj.keys().any(|key| {
        ![
            "type",
            "category",
            "group",
            "key",
            "parts",
            "pattern",
            "result",
            "show_notification",
        ]
        .contains(&key.as_str())
    }) || value["type"].as_str()? != "tconstruct:crafting_shaped_materials"
    {
        return None;
    }
    if obj.get("group").is_some_and(|value| !value.is_string())
        || obj.get("category").is_some_and(|value| !value.is_string())
        || obj
            .get("show_notification")
            .is_some_and(|value| !value.is_boolean())
    {
        return None;
    }
    let result = value["result"].as_object()?;
    if result.len() != 1 {
        return None;
    }
    let result = result.get("item")?.as_str()?;
    if !RESULTS.contains(&result) || !known.contains(result) {
        return None;
    }
    let parts: Vec<char> = value["parts"].as_str()?.chars().collect();
    if parts.len() != 2 || parts[0] == parts[1] || parts.contains(&' ') {
        return None;
    }
    let rows: Vec<Vec<char>> = value["pattern"]
        .as_array()?
        .iter()
        .map(|v| Some(v.as_str()?.chars().collect()))
        .collect::<Option<_>>()?;
    let width = rows.first()?.len();
    if !(1..=3).contains(&width)
        || !(1..=3).contains(&rows.len())
        || rows.iter().any(|row| row.len() != width)
    {
        return None;
    }
    // Source patterns already have no empty border. Refuse unsupported pack
    // borders instead of assigning a different native pattern origin.
    if rows.first()?.iter().all(|c| *c == ' ')
        || rows.last()?.iter().all(|c| *c == ' ')
        || rows.iter().all(|r| r[0] == ' ')
        || rows.iter().all(|r| r[width - 1] == ' ')
    {
        return None;
    }
    let keys = value["key"].as_object()?;
    let used: BTreeSet<char> = rows
        .iter()
        .flatten()
        .copied()
        .filter(|key| *key != ' ')
        .collect();
    if parts.iter().any(|key| !used.contains(key))
        || used.len() != keys.len()
        || used.iter().any(|key| !keys.contains_key(&key.to_string()))
    {
        return None;
    }
    // A single item identity cannot stand for two different materials. Component
    // sensitive candidates must be excluded by the world adapter, not guessed.
    let mut identities = BTreeMap::new();
    for item in candidates {
        if let Some(previous) =
            identities.insert(&item.item, (&item.material, item.value, item.needed))
        {
            if previous != (&item.material, item.value, item.needed) {
                return None;
            }
        }
    }
    let part_keys: Vec<_> = parts
        .iter()
        .map(|c| keys.get(&c.to_string()))
        .collect::<Option<_>>()?;
    let mut choices: [BTreeMap<String, Vec<String>>; 2] = Default::default();
    for item in candidates {
        if !known.contains(&item.item) {
            continue;
        }
        let hits = [
            material_key(part_keys[0], item, &allowed)?,
            material_key(part_keys[1], item, &allowed)?,
        ];
        // Source scans every cell for every part. Overlapping part predicates
        // require that dynamic scan; don't approximate them by slot position.
        if hits[0] && hits[1] {
            return None;
        }
        for p in 0..2 {
            if hits[p] {
                choices[p]
                    .entry(item.material.clone())
                    .or_default()
                    .push(item.item.clone());
            }
        }
    }
    if choices[0].len().checked_mul(choices[1].len())? > 4096 {
        return None;
    }
    for part in &mut choices {
        for items in part.values_mut() {
            items.sort();
            items.dedup();
        }
    }
    let all_parts: BTreeSet<&String> = choices
        .iter()
        .flat_map(|part| part.values())
        .flatten()
        .collect();
    let mut normal = BTreeMap::new();
    for (key, ingredient) in keys {
        let mut chars = key.chars();
        let key = chars.next()?;
        if chars.next().is_some() || key == ' ' {
            return None;
        }
        if !parts.contains(&key) {
            let items = ordinary(ingredient, tags, known)?;
            // Source scans all cells for material identities, including ordinary
            // ingredients. Refuse any such overlap, not just this variant's IDs.
            if items.iter().any(|id| all_parts.contains(id)) {
                return None;
            }
            normal.insert(key, items);
        }
    }
    let mut plans = Vec::new();
    for (first, a) in &choices[0] {
        for (second, b) in &choices[1] {
            let mut cells = Vec::new();
            for key in rows.iter().flatten() {
                let items = if *key == ' ' {
                    cells.push(None);
                    continue;
                } else if *key == parts[0] {
                    a.clone()
                } else if *key == parts[1] {
                    b.clone()
                } else {
                    normal.get(key)?.clone()
                };
                cells.push(Some(items));
            }
            plans.push(Plan {
                result: result.into(),
                materials: vec![first.clone(), second.clone()],
                category: value["category"].as_str().unwrap_or_default().to_owned(),
                group: value["group"].as_str().unwrap_or_default().to_owned(),
                show_notification: value["show_notification"].as_bool().unwrap_or(true),
                width,
                height: rows.len(),
                cells,
            });
        }
    }
    Some(plans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn recipe(result: &str, pattern: &[&str]) -> Value {
        json!({"type":"tconstruct:crafting_shaped_materials", "parts":"cl", "pattern":pattern,
            "key":{"c":{"type":"tconstruct:material_value","material":"metal","value":1.0},
                "l":{"type":"tconstruct:material_value","material":"lining","value":1.0},
                "s":{"item":"minecraft:string"}},"result":{"item":result}})
    }
    fn items() -> Vec<MaterialItem> {
        [
            ("minecraft:iron_ingot", "metal", 1, 1),
            ("minecraft:copper_ingot", "metal", 1, 1),
            ("minecraft:leather", "lining", 1, 1),
            ("minecraft:iron_block", "metal", 9, 1),
            ("minecraft:iron_nugget", "metal", 1, 9),
        ]
        .into_iter()
        .map(|(item, kind, value, needed)| MaterialItem {
            item: item.into(),
            material: format!("test:{kind}_{}", item.split(':').nth(1).unwrap()),
            value,
            needed,
        })
        .collect()
    }
    fn compile_with(value: &Value, candidates: &[MaterialItem]) -> Option<Vec<Plan>> {
        let known = candidates
            .iter()
            .map(|c| c.item.clone())
            .chain(RESULTS.map(str::to_owned))
            .chain(["minecraft:string".into(), "tconstruct:fake_ingot".into()])
            .collect();
        compile(
            &serde_json::to_vec(value).unwrap(),
            candidates,
            &HashMap::new(),
            &known,
            |predicate, material| {
                predicate
                    .as_str()
                    .is_some_and(|kind| material.starts_with(&format!("test:{kind}_")))
            },
        )
    }
    #[test]
    fn five_equipment_patterns_keep_parts_and_source_quantities() {
        for (result, pattern, metal, lining) in [
            (RESULTS[0], vec!["l l", "sls", "c c"], 2, 3),
            (RESULTS[1], vec!["lsl", "lcl", "lcl"], 2, 6),
            (RESULTS[2], vec!["lsl", "c c", "l l"], 2, 4),
            (RESULTS[3], vec!["s s", "c c", "l l"], 2, 2),
            (RESULTS[4], vec!["scs", "lcl"], 2, 2),
        ] {
            let plans = compile_with(&recipe(result, &pattern), &items()).unwrap();
            assert_eq!(plans.len(), 2);
            for plan in plans {
                assert_eq!(plan.result, result);
                assert_eq!((plan.width, plan.height), (3, pattern.len()));
                assert_eq!(plan.materials[1], "test:lining_leather");
                let count = |id: &str| {
                    plan.cells
                        .iter()
                        .flatten()
                        .filter(|items| items.iter().any(|item| item == id))
                        .count()
                };
                assert_eq!(count("minecraft:leather"), lining);
                let metal_id = if plan.materials[0].ends_with("iron_ingot") {
                    "minecraft:iron_ingot"
                } else {
                    "minecraft:copper_ingot"
                };
                assert_eq!(count(metal_id), metal);
                assert_eq!(
                    count("minecraft:iron_block") + count("minecraft:iron_nugget"),
                    0
                );
            }
        }
    }
    #[test]
    fn same_material_alternatives_share_cells_but_distinct_materials_do_not_mix() {
        let mut candidates = items();
        candidates.push(MaterialItem {
            item: "mod:iron_ingot".into(),
            material: "test:metal_iron_ingot".into(),
            value: 1,
            needed: 1,
        });
        let plans = compile_with(&recipe(RESULTS[4], &["scs", "lcl"]), &candidates).unwrap();
        let iron = plans
            .iter()
            .find(|p| p.materials[0] == "test:metal_iron_ingot")
            .unwrap();
        for cell in iron
            .cells
            .iter()
            .flatten()
            .filter(|cell| cell.iter().any(|id| id == "minecraft:iron_ingot"))
        {
            assert_eq!(
                cell,
                &vec![
                    "minecraft:iron_ingot".to_owned(),
                    "mod:iron_ingot".to_owned()
                ]
            );
            assert!(!cell.contains(&"minecraft:copper_ingot".into()));
        }
    }
    #[test]
    fn invalid_patterns_unknown_fields_and_missing_parts_are_not_projected() {
        let good = recipe(RESULTS[4], &["scs", "lcl"]);
        for (key, value) in [
            ("parts", json!("cc")),
            ("parts", json!("c ")),
            ("pattern", json!(["scs"])),
            ("pattern", json!(["scs", "lc ", "   "])),
            ("extra_materials", json!(["test:extra"])),
            ("result", json!({"item":RESULTS[4],"count":2})),
            ("show_notification", json!("true")),
        ] {
            let mut changed = good.clone();
            changed[key] = value;
            assert!(compile_with(&changed, &items()).is_none(), "{key}");
        }
        let mut changed = good.clone();
        changed["key"]["x"] = json!({"item":"minecraft:string"});
        assert!(compile_with(&changed, &items()).is_none());
    }
    #[test]
    fn no_unrestricted_fake_ingot_or_unknown_material_ingredient() {
        let mut value = recipe(RESULTS[4], &["scs", "lcl"]);
        let metal = value["key"]["c"].clone();
        value["key"]["c"] = json!([metal,{"type":"tconstruct:material","item":"tconstruct:fake_ingot","material":"metal"}]);
        let plans = compile_with(&value, &items()).unwrap();
        assert!(plans
            .iter()
            .flat_map(|p| &p.cells)
            .flatten()
            .flatten()
            .all(|id| id != "tconstruct:fake_ingot"));
        value["key"]["c"][1]["item"] = json!("mod:component_sensitive");
        assert!(compile_with(&value, &items()).is_none());
    }
    #[test]
    fn normal_cells_cannot_select_another_material_from_the_global_scan() {
        let mut value = recipe(RESULTS[4], &["scs", "lcl"]);
        value["key"]["s"] = json!({"item":"minecraft:copper_ingot"});
        assert!(compile_with(&value, &items()).is_none());
    }
    #[test]
    fn ambiguous_item_identity_and_overlapping_part_predicates_fail_closed() {
        let mut candidates = items();
        let mut second = candidates[0].clone();
        second.material = "test:metal_other".into();
        candidates.push(second);
        assert!(compile_with(&recipe(RESULTS[4], &["scs", "lcl"]), &candidates).is_none());
        let mut value = recipe(RESULTS[4], &["scs", "lcl"]);
        value["key"]["l"] = value["key"]["c"].clone();
        assert!(compile_with(&value, &items()).is_none());
    }
    #[test]
    fn every_refresh_uses_current_candidates_and_known_registry() {
        let value = recipe(RESULTS[4], &["scs", "lcl"]);
        assert_eq!(compile_with(&value, &items()).unwrap().len(), 2);
        assert!(compile_with(&value, &[]).unwrap().is_empty());
        assert_eq!(compile_with(&value, &items()).unwrap().len(), 2);
        assert!(compile(
            &serde_json::to_vec(&value).unwrap(),
            &items(),
            &HashMap::new(),
            &HashSet::new(),
            |_, _| true
        )
        .is_none());
    }
    #[test]
    fn book_metadata_preserves_source_values_and_native_defaults() {
        let mut value = recipe(RESULTS[4], &["scs", "lcl"]);
        let plans = compile_with(&value, &items()).unwrap();
        assert!(plans.iter().all(|p| p.category.is_empty() && p.group.is_empty() && p.show_notification));
        value["category"] = json!("equipment");
        value["group"] = json!("test:travelers");
        value["show_notification"] = json!(false);
        let plans = compile_with(&value, &items()).unwrap();
        assert!(plans.iter().all(|p| p.category == "equipment" && p.group == "test:travelers" && !p.show_notification));
        value["show_notification"] = json!("false");
        assert!(compile_with(&value, &items()).is_none());
    }

}
