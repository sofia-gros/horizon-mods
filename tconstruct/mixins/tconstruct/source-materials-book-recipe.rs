use serde_json::Value;
use std::collections::{HashMap, HashSet};

// Bind each casting-fluid bucket to its own translated description. Sorting
// icons separately from tooltips would name the wrong fluid while cycling.
pub(super) fn material_fluid_options(choices: &[(String, String)], title: &str, template: &str) -> Option<Value> {
    if title.trim().is_empty() || title.len() > 256 || template.matches("%s").count() != 1 { return None; }
    let mut choices = choices.to_vec(); choices.sort(); choices.dedup();
    if choices.is_empty() || choices.len() > 128 { return None; }
    let mut options = Vec::new();
    for (item, fluid) in choices {
        if item.len() > 256 || item == "minecraft:air" || showcase_item(&Value::String(item.clone())).is_none()
            || fluid.trim().is_empty() || fluid.len() > 256 { return None; }
        let description = template.replacen("%s", &fluid, 1);
        if description.len() > 256 { return None; }
        options.push(serde_json::json!({"item":item, "tooltip":[title,description]}));
    }
    let entry = serde_json::json!({"options":options});
    (entry.to_string().len() <= 4096).then_some(entry)
}

// Keep every supported part/fluid combination together with the input material.
// These are input-part previews, not output tools or filled fluid containers.
pub(super) fn material_composite_options(parts: &[String], fluids: &[String], material: &str,
    material_name: &str, title: &str, template: &str) -> Option<Value> {
    if title.trim().is_empty() || title.len() > 256 || material_name.trim().is_empty() || material_name.len() > 256
        || material.len() > 256 || showcase_item(&Value::String(material.into())).is_none() { return None; }
    let format: Vec<_> = template.split("%s").collect();
    if format.len() != 3 { return None; }
    let mut parts = parts.to_vec(); parts.sort(); parts.dedup();
    let mut fluids = fluids.to_vec(); fluids.sort(); fluids.dedup();
    if parts.is_empty() || fluids.is_empty() || parts.len().checked_mul(fluids.len())? > 128 { return None; }
    if parts.iter().any(|part| part.len() > 256 || part == "minecraft:air" || showcase_item(&Value::String(part.clone())).is_none())
        || fluids.iter().any(|fluid| fluid.trim().is_empty() || fluid.len() > 256) { return None; }
    let mut options = Vec::new();
    for fluid in fluids {
        // Substitute the original placeholders only; names can themselves contain %s.
        let description = format!("{}{}{}{}{}", format[0], fluid, format[1], material_name, format[2]);
        if description.len() > 256 { return None; }
        for part in &parts {
            options.push(serde_json::json!({"item":part,"material":material,"tooltip":[title,description]}));
        }
    }
    let entry = serde_json::json!({"options":options});
    (entry.to_string().len() <= 4096).then_some(entry)
}

// Legacy category/repair/tool entries retain their existing behavior. New
// paired options select once, then drawing and hover share the same entry.
pub(super) fn material_sidebar_entry(entry: &Value, seconds: u64) -> Option<&Value> {
    let Some(options) = entry.get("options") else { return Some(entry); };
    let options = options.as_array().filter(|values| !values.is_empty() && values.len() <= 128)?;
    let selected = &options[((seconds / 3) % options.len() as u64) as usize];
    let item = selected.get("item")?;
    let id = item.as_str().filter(|id| id.len() <= 256 && *id != "minecraft:air")?;
    if showcase_item(item) != Some(id) { return None; }
    if let Some(material) = selected.get("material") {
        let id = material.as_str().filter(|id| id.len() <= 256)?;
        if showcase_item(material) != Some(id) { return None; }
    }
    let tooltip = selected.get("tooltip")?.as_array().filter(|lines| lines.len() == 2)?;
    if !tooltip.iter().all(|line| line.as_str().is_some_and(|line| !line.trim().is_empty() && line.len() <= 256)) { return None; }
    Some(selected)
}

// Top-down 3x3 layers, north first. Null cells are not required; the wall
// layer's center must remain air. The controller faces north, away from it.
pub(super) fn small_smeltery_layers() -> [[Option<&'static str>; 9]; 2] {
    [
        [None, None, None, None, Some("tconstruct:seared_bricks"), None, None, None, None],
        [None, Some("tconstruct:smeltery_controller"), None,
            Some("tconstruct:seared_bricks"), Some("minecraft:air"), Some("tconstruct:seared_fuel_tank"),
            None, Some("tconstruct:seared_drain"), None],
    ]
}

// A fixed construction example must disappear when pack overrides make its
// exact block choices invalid. Registered icons alone cannot prove a plan valid.
pub(super) fn small_smeltery_spec(tags: Option<&HashMap<String, HashSet<String>>>, mut known: impl FnMut(&str) -> bool) -> Option<Value> {
    let tags = tags?;
    for (tag, block) in [
        ("tconstruct:smeltery/floor", "tconstruct:seared_bricks"),
        ("tconstruct:smeltery/wall", "tconstruct:seared_bricks"),
        ("tconstruct:smeltery/wall", "tconstruct:seared_drain"),
        ("tconstruct:smeltery/tanks", "tconstruct:seared_fuel_tank"),
    ] {
        if !tags.get(tag).is_some_and(|members| members.contains(block)) { return None; }
    }
    for id in small_smeltery_layers().into_iter().flatten().flatten() {
        if id != "minecraft:air" && (!known(id)
            || tags.get("tconstruct:structure_air").is_some_and(|members| members.contains(id))) { return None; }
    }
    Some(serde_json::json!({"small_smeltery":true}))
}

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

// Modifier ingredients are separate Station/Anvil inputs, not a crafting grid.
// Never drop a required slot or silently trim alternatives to fit the display.
pub(super) fn modifier_input_spec(inputs: &[Vec<String>]) -> Option<Value> {
    if !(1..=5).contains(&inputs.len()) { return None; }
    let mut slots = inputs.to_vec();
    for choices in &mut slots {
        choices.sort(); choices.dedup();
        if choices.is_empty() || choices.len() > 128 { return None; }
        if choices.iter().any(|id| id.len() > 256 || id == "minecraft:air"
            || showcase_item(&Value::String(id.clone())).is_none()) { return None; }
    }
    let spec = serde_json::json!({"modifier_inputs":slots});
    (spec.to_string().len() <= 4096).then_some(spec)
}

// Drawing and hover consume exactly the same selected item for each slot.
pub(super) fn modifier_input_items(spec: &Value, seconds: u64) -> Option<Vec<&str>> {
    let slots = spec.get("modifier_inputs")?.as_array().filter(|slots| (1..=5).contains(&slots.len()))?;
    slots.iter().map(|slot| modifier_input_choice(slot, seconds)).collect()
}

fn modifier_input_choice(slot: &Value, seconds: u64) -> Option<&str> {
    let choices = slot.as_array().filter(|choices| !choices.is_empty() && choices.len() <= 128)?;
    for choice in choices {
        let id = choice.as_str().filter(|id| id.len() <= 256 && *id != "minecraft:air")?;
        if showcase_item(choice) != Some(id) { return None; }
    }
    choices[((seconds / 3) % choices.len() as u64) as usize].as_str()
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ModifierProgressView<'a> {
    pub input: &'a str,
    pub leftover: Option<&'a str>,
    pub per_item: u32,
    pub needed: u32,
    pub from_zero: u32,
    pub returned_from_zero: u32,
}

pub(super) fn modifier_progress_spec(inputs: &[String], per_item: i32, needed: i32, leftover: Option<&str>) -> Option<Value> {
    let inputs = modifier_input_spec(&[inputs.to_vec()])?;
    let spec = serde_json::json!({"modifier_progress":{
        "inputs":inputs["modifier_inputs"][0], "per_item":per_item, "needed":needed, "leftover":leftover
    }});
    modifier_progress_view(&spec, 0)?;
    (spec.to_string().len() <= 4096).then_some(spec)
}

// This is an example starting at zero progress, never a prediction for a tool.
// Calculate in u64 so ceil(needed/per_item)*per_item cannot overflow i32.
pub(super) fn modifier_progress_view(spec: &Value, seconds: u64) -> Option<ModifierProgressView<'_>> {
    let progress = spec.get("modifier_progress")?;
    let input = modifier_input_choice(progress.get("inputs")?, seconds)?;
    let per_item = progress.get("per_item")?.as_u64().filter(|value| matches!(value, 1 | 4 | 9))?;
    let needed = progress.get("needed")?.as_u64().filter(|value| *value > 0 && *value <= i32::MAX as u64)?;
    let leftover = match (per_item, progress.get("leftover")?) {
        (1, Value::Null) => None,
        (4 | 9, value) => {
            let id = value.as_str().filter(|id| id.len() <= 256 && *id != "minecraft:air")?;
            if showcase_item(value) != Some(id) { return None; }
            Some(id)
        }
        _ => return None,
    };
    let from_zero = needed / per_item + u64::from(needed % per_item != 0);
    Some(ModifierProgressView { input, leftover, per_item:per_item as u32, needed:needed as u32,
        from_zero:from_zero as u32, returned_from_zero:(from_zero * per_item - needed) as u32 })
}

// Slot frames are 22px wide, spaced by 28px and centered on a 182px page.
pub(super) fn modifier_input_x(index: usize, count: usize) -> Option<f32> {
    if !(1..=5).contains(&count) || index >= count { return None; }
    Some((182.0 - (count as f32 * 28.0 - 6.0)) / 2.0 + index as f32 * 28.0)
}

// Static book diagrams are explanatory, not a crafting authority. Omit the
// whole graphic when a required input or shape cannot be represented exactly.
fn crafting_ingredient(value: &Value, tags: Option<&HashMap<String, HashSet<String>>>,
    known: &mut impl FnMut(&str) -> bool, depth: usize) -> Option<Vec<String>> {
    if depth > 8 { return None; }
    let mut choices = if let Some(values) = value.as_array() {
        if values.is_empty() || values.len() > 128 { return None; }
        let mut choices = Vec::new();
        for value in values {
            choices.extend(crafting_ingredient(value, tags, known, depth + 1)?);
            if choices.len() > 128 { return None; }
        }
        choices
    } else if value.get("type").and_then(Value::as_str) == Some("forge:difference") {
        // Resolve both complete sets first. Unknown subtraction semantics must
        // never silently turn an excluded workbench back into an alternative.
        let object = value.as_object()?;
        if object.len() != 3 { return None; }
        let base = crafting_ingredient(object.get("base")?, tags, known, depth + 1)?;
        let subtracted: HashSet<_> = crafting_ingredient(object.get("subtracted")?, tags, known, depth + 1)?
            .into_iter().collect();
        base.into_iter().filter(|item| !subtracted.contains(item)).collect()
    } else {
        let (id, tag) = if let Some(value) = value.as_str() {
            value.strip_prefix('#').map(|tag| (tag, true)).unwrap_or((value, false))
        } else {
            let object = value.as_object()?;
            // Complex Forge ingredients and component/NBT restrictions need
            // their own renderer; showing their base item would be misleading.
            if object.len() != 1 { return None; }
            if let Some(item) = object.get("item") { (item.as_str()?, false) }
            else { (object.get("tag")?.as_str()?, true) }
        };
        if id.len() > 256 || showcase_item(&Value::String(id.into())) != Some(id) { return None; }
        if tag {
            let members = tags?.get(id)?;
            // Bound the snapshot walk as well as the resulting metadata.
            if members.len() > 128 { return None; }
            members.iter().filter(|item| item.as_str() != "minecraft:air" && item.len() <= 256
                && showcase_item(&Value::String((*item).clone())).is_some() && known(item)).cloned().collect()
        } else {
            if id == "minecraft:air" || !known(id) { return None; }
            vec![id.into()]
        }
    };
    choices.sort(); choices.dedup();
    (!choices.is_empty() && choices.len() <= 128).then_some(choices)
}

pub(super) fn resolve(recipe: &Value, tags: Option<&HashMap<String, HashSet<String>>>, grid_size: &str,
    mut known: impl FnMut(&str) -> bool, mut conditions: impl FnMut(&[Value]) -> bool) -> Option<Value> {
    if let Some(raw) = recipe.get("conditions") {
        let values = raw.as_array()?;
        if !values.is_empty() && !conditions(values) { return None; }
    }
    let grid = match grid_size { "small" => 2, "large" => 3, _ => return None };
    let shaped = match recipe.get("type")?.as_str()? {
        "minecraft:crafting_shaped" | "mantle:crafting_shaped_retextured" => true,
        "minecraft:crafting_shapeless" => false,
        _ => return None,
    };
    let mut slots: Vec<Vec<String>> = vec![Vec::new(); 9];
    if shaped {
        if recipe.get("ingredients").is_some() { return None; }
        let pattern = recipe.get("pattern")?.as_array()?;
        let keys = recipe.get("key")?.as_object()?;
        if pattern.is_empty() || pattern.len() > grid || keys.is_empty() || keys.len() > grid * grid { return None; }
        let width = pattern.first()?.as_str()?.chars().count();
        if width == 0 || width > grid { return None; }
        let mut resolved = HashMap::new();
        for (key, value) in keys {
            let mut chars = key.chars();
            let ch = chars.next()?;
            if ch == ' ' || chars.next().is_some() { return None; }
            resolved.insert(ch, crafting_ingredient(value, tags, &mut known, 0)?);
        }
        let mut used = HashSet::new();
        for (y, line) in pattern.iter().enumerate() {
            let line = line.as_str()?;
            if line.chars().count() != width { return None; }
            for (x, ch) in line.chars().enumerate() {
                if ch != ' ' {
                    slots[y * 3 + x] = resolved.get(&ch)?.clone();
                    used.insert(ch);
                }
            }
        }
        if used.len() != resolved.len() { return None; }
    } else {
        if recipe.get("pattern").is_some() || recipe.get("key").is_some() { return None; }
        let ingredients = recipe.get("ingredients")?.as_array()?;
        if ingredients.is_empty() || ingredients.len() > grid * grid { return None; }
        for (i, value) in ingredients.iter().enumerate() {
            slots[(i / grid) * 3 + i % grid] = crafting_ingredient(value, tags, &mut known, 0)?;
        }
    }
    let result = recipe.get("result")?.as_object()?;
    if result.keys().any(|key| !matches!(key.as_str(), "item" | "id" | "count"))
        || result.contains_key("item") == result.contains_key("id") { return None; }
    let id = result.get("item").or_else(|| result.get("id"))?.as_str()?;
    if id.len() > 256 || id == "minecraft:air" || showcase_item(&Value::String(id.into())) != Some(id) || !known(id) { return None; }
    let count = match result.get("count") {
        None => 1,
        Some(value) => value.as_u64().filter(|count| *count > 0 && *count <= u32::MAX as u64)?,
    };
    let spec = serde_json::json!({"slots":slots,"result":id,"count":count,"grid":grid});
    (spec.to_string().len() <= 4096).then_some(spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn composite_options_cycle_every_part_and_fluid_with_input_material() {
        let entry = material_composite_options(&["test:head".into(), "test:binding".into(), "test:head".into()],
            &["Fluid Z".into(), "Fluid A".into(), "Fluid Z".into()], "test:input", "Input", "Composite", "Cast %s on %s parts").unwrap();
        assert_eq!(entry["options"].as_array().unwrap().len(), 4);
        for (phase, (part, fluid)) in [("test:binding", "Fluid A"), ("test:head", "Fluid A"),
            ("test:binding", "Fluid Z"), ("test:head", "Fluid Z")].into_iter().enumerate() {
            for second in 0..3 {
                let selected = material_sidebar_entry(&entry, phase as u64 * 3 + second).unwrap();
                assert_eq!(selected["item"], part);
                assert_eq!(selected["material"], "test:input");
                assert_eq!(selected["tooltip"], serde_json::json!(["Composite", format!("Cast {fluid} on Input parts")]));
            }
        }
        assert_eq!(material_sidebar_entry(&entry, 12), material_sidebar_entry(&entry, 0));
        assert_eq!(material_sidebar_entry(&entry, u64::MAX).unwrap()["item"], "test:head");
        let replaced = material_composite_options(&["test:head".into()], &["Only current fluid".into()],
            "test:input", "Input", "Composite", "Cast %s on %s parts").unwrap();
        assert_eq!(replaced["options"].as_array().unwrap().len(), 1);
        assert_eq!(replaced["options"][0]["tooltip"][1], "Cast Only current fluid on Input parts");
    }
    #[test]
    fn composite_options_validate_bounds_and_preserve_literal_names() {
        let build = |parts: &[String], fluids: &[String]| material_composite_options(parts, fluids,
            "test:input", "Input", "Composite", "Cast %s on %s parts");
        for parts in [vec![], vec!["bad id".into()], vec!["minecraft:air".into()],
            (0..129).map(|i| format!("test:part_{i}")).collect(),
            (0..100).map(|i| format!("test:part_{i}")).collect()] {
            assert!(build(&parts, &["Fluid".into()]).is_none());
        }
        for fluids in [vec![], vec![" ".into()], vec!["x".repeat(257)], vec!["x".repeat(250)]] {
            assert!(build(&["test:head".into()], &fluids).is_none());
        }
        for (material, name, title, format) in [("bad id", "Input", "Composite", "%s on %s"),
            ("test:input", "", "Composite", "%s on %s"), ("test:input", "Input", "", "%s on %s"),
            ("test:input", "Input", "Composite", "%s"), ("test:input", "Input", "Composite", "%s %s %s")] {
            assert!(material_composite_options(&["test:head".into()], &["Fluid".into()], material, name, title, format).is_none());
        }
        let literal = material_composite_options(&["test:head".into()], &["Fluid %s".into()],
            "test:input", "Input %s", "Composite", "%s on %s").unwrap();
        assert_eq!(literal["options"][0]["tooltip"][1], "Fluid %s on Input %s");
        for material in [serde_json::json!(42), serde_json::json!("bad id")] {
            let mut malformed = literal.clone(); malformed["options"][0]["material"] = material;
            assert!(material_sidebar_entry(&malformed, 0).is_none());
        }
    }
    #[test]
    fn material_fluid_options_keep_bucket_and_description_together() {
        let choices = vec![("test:z_bucket".into(), "Molten Z".into()), ("test:a_bucket".into(), "Molten A".into()),
            ("test:z_bucket".into(), "Molten Z".into())];
        let entry = material_fluid_options(&choices, "Castable", "Cast from %s").unwrap();
        assert_eq!(entry["options"].as_array().unwrap().len(), 2);
        for seconds in [0, 1, 2, 6, 7, 8] {
            let selected = material_sidebar_entry(&entry, seconds).unwrap();
            assert_eq!(selected["item"], "test:a_bucket");
            assert_eq!(selected["tooltip"], serde_json::json!(["Castable", "Cast from Molten A"]));
        }
        for seconds in [3, 4, 5, u64::MAX] {
            let selected = material_sidebar_entry(&entry, seconds).unwrap();
            assert_eq!(selected["item"], "test:z_bucket");
            assert_eq!(selected["tooltip"][1], "Cast from Molten Z");
        }
        let shared_icon = material_fluid_options(&[("test:a".into(), "A".into()), ("test:a".into(), "B".into())], "Castable", "Cast from %s").unwrap();
        assert_eq!(shared_icon["options"].as_array().unwrap().len(), 2);
    }
    #[test]
    fn material_fluid_options_reject_invalid_and_over_budget_pairs() {
        for choices in [vec![], vec![("bad id".into(), "A".into())], vec![("minecraft:air".into(), "A".into())],
            vec![("test:a".into(), " ".into())], vec![("test:a".into(), "x".repeat(257))],
            (0..129).map(|i| (format!("test:bucket_{i}"), format!("Fluid {i}"))).collect(),
            (0..100).map(|i| (format!("test:bucket_{i}"), format!("Fluid {i}"))).collect()] {
            assert!(material_fluid_options(&choices, "Castable", "Cast from %s").is_none());
        }
        for (title, template) in [("", "Cast from %s"), ("Castable", "No placeholder"), ("Castable", "%s and %s")] {
            assert!(material_fluid_options(&[("test:a".into(), "A".into())], title, template).is_none());
        }
        for entry in [serde_json::json!({"options":[]}), serde_json::json!({"options":42}),
            serde_json::json!({"options":[{"item":"bad id","tooltip":["Title","Text"]}]}),
            serde_json::json!({"options":[{"item":"test:a","tooltip":["Title",42]}]}),
            serde_json::json!({"options":[{"item":"test:a","tooltip":["Title"]}]})] {
            assert!(material_sidebar_entry(&entry, 0).is_none());
        }
    }
    #[test]
    fn material_sidebar_legacy_entries_remain_unchanged() {
        for entry in [serde_json::json!({"item":"test:tool","materials":["test:material"]}),
            serde_json::json!({"choices":["test:a","test:b"],"tooltip":["Original tooltip"]})] {
            for seconds in [0, 3, u64::MAX] { assert_eq!(material_sidebar_entry(&entry, seconds), Some(&entry)); }
        }
    }
    #[test]
    fn small_smeltery_uses_current_tags_and_all_registered_parts() {
        let tags = HashMap::from([
            ("tconstruct:smeltery/floor".into(), HashSet::from(["tconstruct:seared_bricks".into()])),
            ("tconstruct:smeltery/wall".into(), HashSet::from(["tconstruct:seared_bricks".into(), "tconstruct:seared_drain".into()])),
            ("tconstruct:smeltery/tanks".into(), HashSet::from(["tconstruct:seared_fuel_tank".into()])),
        ]);
        assert_eq!(small_smeltery_spec(Some(&tags), |_| true), Some(serde_json::json!({"small_smeltery":true})));
        assert!(small_smeltery_spec(None, |_| true).is_none());
        assert!(small_smeltery_spec(Some(&HashMap::new()), |_| true).is_none());
        for (tag, members) in &tags {
            for block in members {
                let mut removed = tags.clone(); removed.get_mut(tag).unwrap().remove(block);
                assert!(small_smeltery_spec(Some(&removed), |_| true).is_none());
            }
        }
        for missing in ["tconstruct:seared_bricks", "tconstruct:smeltery_controller", "tconstruct:seared_fuel_tank", "tconstruct:seared_drain"] {
            assert!(small_smeltery_spec(Some(&tags), |id| id != missing).is_none());
            let mut ambiguous = tags.clone();
            ambiguous.insert("tconstruct:structure_air".into(), HashSet::from([missing.to_owned()]));
            assert!(small_smeltery_spec(Some(&ambiguous), |_| true).is_none());
        }
    }
    #[test]
    fn incremental_preview_counts_points_items_and_conditional_returns() {
        for (per_item, needed, leftover, count, returned) in [
            (1, 72, None, 72, 0), (4, 72, Some("test:point"), 18, 0),
            (9, 20, Some("test:point"), 3, 7), (9, 2, Some("test:point"), 1, 7),
            (4, 1, Some("test:point"), 1, 3), (9, i32::MAX, Some("test:point"), 238609295, 8),
        ] {
            let spec = modifier_progress_spec(&["test:b".into(), "test:a".into()], per_item, needed, leftover).unwrap();
            let view = modifier_progress_view(&spec, 0).unwrap();
            assert_eq!((view.per_item, view.needed, view.from_zero, view.returned_from_zero), (per_item as u32, needed as u32, count, returned));
            assert_eq!(view.input, "test:a");
            assert_eq!(view.leftover, leftover);
            assert_eq!(modifier_progress_view(&spec, 3).unwrap().input, "test:b");
            assert_eq!(modifier_progress_view(&spec, u64::MAX).unwrap().input, "test:b");
        }
    }
    #[test]
    fn incremental_preview_rejects_unknown_values_and_bad_return_items() {
        for (per_item, needed, leftover) in [
            (0, 10, None), (2, 10, Some("test:point")), (1, 0, None), (1, -1, None),
            (1, 10, Some("test:point")), (4, 10, None), (9, 10, Some("bad id")),
            (9, 10, Some("minecraft:air")),
        ] {
            assert!(modifier_progress_spec(&["test:input".into()], per_item, needed, leftover).is_none());
        }
        assert!(modifier_progress_spec(&[], 1, 10, None).is_none());
        let mut spec = modifier_progress_spec(&["test:input".into()], 1, 10, None).unwrap();
        spec["modifier_progress"]["needed"] = serde_json::json!(u64::MAX);
        assert!(modifier_progress_view(&spec, 0).is_none());
        spec["modifier_progress"]["needed"] = serde_json::json!(10);
        spec["modifier_progress"]["inputs"] = serde_json::json!(["test:input", 42]);
        assert!(modifier_progress_view(&spec, 0).is_none());
    }
    #[test]
    fn modifier_inputs_preserve_five_separate_slots_and_cycle_alternatives() {
        let input = vec![vec!["test:b".into(), "test:a".into(), "test:b".into()],
            vec!["test:a".into()], vec!["test:c".into()], vec!["test:d".into()], vec!["test:e".into()]];
        let spec = modifier_input_spec(&input).unwrap();
        assert_eq!(spec["modifier_inputs"][0], serde_json::json!(["test:a", "test:b"]));
        assert_eq!(spec["modifier_inputs"].as_array().unwrap().len(), 5);
        for second in [0, 1, 2, 6, 7, 8] {
            assert_eq!(modifier_input_items(&spec, second).unwrap(), ["test:a", "test:a", "test:c", "test:d", "test:e"]);
        }
        for second in [3, 4, 5, u64::MAX] {
            assert_eq!(modifier_input_items(&spec, second).unwrap(), ["test:b", "test:a", "test:c", "test:d", "test:e"]);
        }
    }
    #[test]
    fn modifier_input_bounds_fail_without_partial_or_misleading_icons() {
        for slots in [vec![], vec![vec![]], vec![vec!["test:a".into()]; 6],
            vec![vec!["bad id".into()]], vec![vec!["minecraft:air".into()]],
            vec![vec![format!("test:{}", "a".repeat(256))]],
            vec![(0..129).map(|i| format!("test:item_{i}")).collect()],
            vec![(0..128).map(|i| format!("test:very_long_alternative_item_{i:03}")).collect()]] {
            assert!(modifier_input_spec(&slots).is_none());
        }
        for spec in [serde_json::json!({"modifier_inputs":[]}), serde_json::json!({"modifier_inputs":[[]]}),
            serde_json::json!({"modifier_inputs":[["test:a", 42]]}),
            serde_json::json!({"modifier_inputs":[["test:a", "bad id"]]}),
            serde_json::json!({"modifier_inputs":[[{"item":"test:a"}]]}),
            serde_json::json!({"modifier_inputs":[["minecraft:air"]]})] {
            assert!(modifier_input_items(&spec, 0).is_none());
        }
    }
    #[test]
    fn modifier_input_geometry_fits_one_to_five_numbered_slots() {
        for count in 1..=5 {
            let first = modifier_input_x(0, count).unwrap();
            let last = modifier_input_x(count - 1, count).unwrap();
            assert_eq!(first, 182.0 - last - 22.0);
            assert!(first >= 0.0 && last + 22.0 <= 182.0);
            for index in 1..count { assert_eq!(modifier_input_x(index, count).unwrap() - modifier_input_x(index - 1, count).unwrap(), 28.0); }
            assert_eq!(modifier_input_x(count, count), None);
        }
        assert_eq!(modifier_input_x(0, 0), None);
        assert_eq!(modifier_input_x(0, 6), None);
    }
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
        let source = serde_json::json!({"type":"minecraft:crafting_shaped","pattern":["pw"], "key":{"p":{"item":"tconstruct:pattern"},"w":{"tag":"tconstruct:planklike"}},"result":{"item":"tconstruct:part_builder","count":4}});
        let actual = resolve(&source, Some(&tags), "small", |_| true, |_| false).unwrap();
        assert_eq!(actual["slots"][0], serde_json::json!(["tconstruct:pattern"]));
        assert_eq!(actual["slots"][1], serde_json::json!(["minecraft:birch_planks", "minecraft:oak_planks"]));
        assert_eq!(actual["count"], 4);
        assert_eq!(actual["grid"], 2);
    }
    #[test]
    fn shapeless_small_grid_places_third_input_on_second_row() {
        let source = serde_json::json!({"type":"minecraft:crafting_shapeless","ingredients":["minecraft:a","minecraft:b","minecraft:c"],"result":{"id":"minecraft:book"}});
        let actual = resolve(&source, None, "small", |_| true, |_| false).unwrap();
        assert_eq!(actual["slots"][3], serde_json::json!(["minecraft:c"]));
        assert_eq!(actual["count"], 1);
    }
    fn simple_crafting_recipe() -> Value {
        serde_json::json!({"type":"minecraft:crafting_shapeless", "ingredients":[{"item":"test:input"}],
            "result":{"item":"test:output"}})
    }
    #[test]
    fn crafting_conditions_gate_before_resolving_icons() {
        let mut source = simple_crafting_recipe();
        source["conditions"] = serde_json::json!([{"type":"forge:false"}]);
        assert!(resolve(&source, None, "large", |_| panic!("disabled recipe must not resolve icons"), |_| false).is_none());
        assert!(resolve(&source, None, "large", |_| true, |conditions| conditions.len() == 1).is_some());
        source["conditions"] = serde_json::json!([]);
        assert!(resolve(&source, None, "large", |_| true, |_| panic!("empty conditions need no snapshot")).is_some());
        source["conditions"] = serde_json::json!({"type":"forge:true"});
        assert!(resolve(&source, None, "large", |_| true, |_| true).is_none());
    }
    #[test]
    fn crafting_requires_registered_inputs_outputs_and_live_nonempty_tags() {
        let mut source = simple_crafting_recipe();
        for unavailable in ["test:input", "test:output"] {
            assert!(resolve(&source, None, "large", |id| id != unavailable, |_| true).is_none());
        }
        source["ingredients"][0] = serde_json::json!({"tag":"test:inputs"});
        assert!(resolve(&source, None, "large", |_| true, |_| true).is_none());
        let tags = HashMap::from([("test:inputs".into(), ["test:input".into(), "test:missing".into(), "minecraft:air".into()].into_iter().collect())]);
        let view = resolve(&source, Some(&tags), "large", |id| id != "test:missing", |_| true).unwrap();
        assert_eq!(view["slots"][0], serde_json::json!(["test:input"]));
        assert!(resolve(&source, Some(&tags), "large", |id| id == "test:output", |_| true).is_none());
    }
    #[test]
    fn crafting_rejects_unsupported_serializers_ingredients_and_results() {
        let source = simple_crafting_recipe();
        for kind in ["forge:conditional", "test:custom", "minecraft:smelting", ""] {
            let mut bad = source.clone(); bad["type"] = serde_json::json!(kind);
            assert!(resolve(&bad, None, "large", |_| true, |_| true).is_none());
        }
        for input in [serde_json::json!({"type":"forge:intersection","children":[{"item":"test:input"},{"item":"test:other"}]}),
            serde_json::json!({"item":"test:input","nbt":{}}), serde_json::json!({"item":"test:input","tag":"test:inputs"}),
            serde_json::json!("minecraft:air"), serde_json::json!("bad id"), serde_json::json!([])] {
            let mut bad = source.clone(); bad["ingredients"][0] = input;
            assert!(resolve(&bad, None, "large", |_| true, |_| true).is_none());
        }
        for output in [serde_json::json!({"item":"test:output","nbt":{}}),
            serde_json::json!({"item":"test:output","id":"test:other"}), serde_json::json!({"item":"minecraft:air"}),
            serde_json::json!({"item":"bad id"}), serde_json::json!({}), serde_json::json!("test:output")] {
            let mut bad = source.clone(); bad["result"] = output;
            assert!(resolve(&bad, None, "large", |_| true, |_| true).is_none());
        }
        for count in [serde_json::json!(0), serde_json::json!(-1), serde_json::json!(1.5),
            serde_json::json!("2"), serde_json::json!(u32::MAX as u64 + 1)] {
            let mut bad = source.clone(); bad["result"]["count"] = count;
            assert!(resolve(&bad, None, "large", |_| true, |_| true).is_none());
        }
    }
    #[test]
    fn crafting_never_truncates_oversized_or_malformed_grids() {
        let mut source = simple_crafting_recipe();
        for grid in ["small", "large"] {
            let limit = if grid == "small" { 4 } else { 9 };
            source["ingredients"] = serde_json::json!(vec![serde_json::json!({"item":"test:input"}); limit + 1]);
            assert!(resolve(&source, None, grid, |_| true, |_| true).is_none());
        }
        assert!(resolve(&simple_crafting_recipe(), None, "unknown", |_| true, |_| true).is_none());
        let shaped = serde_json::json!({"type":"minecraft:crafting_shaped","pattern":["aa","aa"],
            "key":{"a":{"item":"test:input"}},"result":{"item":"test:output"}});
        assert!(resolve(&shaped, None, "small", |_| true, |_| true).is_some());
        for pattern in [serde_json::json!([]), serde_json::json!([""]), serde_json::json!(["aaa"]),
            serde_json::json!(["a","a","a"]), serde_json::json!(["aa","a"]), serde_json::json!(["ab"]), serde_json::json!(["  "])] {
            let mut bad = shaped.clone(); bad["pattern"] = pattern;
            assert!(resolve(&bad, None, "small", |_| true, |_| true).is_none());
        }
        for key in ["unused", "b", " "] {
            let mut bad = shaped.clone(); bad["key"][key] = serde_json::json!({"item":"test:input"});
            assert!(resolve(&bad, None, "small", |_| true, |_| true).is_none());
        }
        let mut ambiguous = shaped; ambiguous["ingredients"] = serde_json::json!([{"item":"test:input"}]);
        assert!(resolve(&ambiguous, None, "small", |_| true, |_| true).is_none());
    }
    #[test]
    fn crafting_bounds_alternatives_depth_and_wire_metadata() {
        let mut source = simple_crafting_recipe();
        source["ingredients"][0] = serde_json::json!([{"item":"test:b"},{"item":"test:a"},{"item":"test:b"}]);
        let view = resolve(&source, None, "large", |_| true, |_| true).unwrap();
        assert_eq!(view["slots"][0], serde_json::json!(["test:a","test:b"]));
        source["ingredients"][0] = serde_json::json!((0..129).map(|i| format!("test:item_{i}")).collect::<Vec<_>>());
        assert!(resolve(&source, None, "large", |_| true, |_| true).is_none());
        let mut deep = serde_json::json!("test:input");
        for _ in 0..10 { deep = serde_json::json!([deep]); }
        source["ingredients"][0] = deep;
        assert!(resolve(&source, None, "large", |_| true, |_| true).is_none());
        let choices: Vec<_> = (0..128).map(|i| format!("test:{}_{i}", "x".repeat(50))).collect();
        source["ingredients"][0] = serde_json::json!(choices);
        assert!(resolve(&source, None, "large", |_| true, |_| true).is_none());
    }

    #[test]
    fn crafting_difference_excludes_current_world_members_without_truncating() {
        let source = serde_json::json!({"type":"minecraft:crafting_shaped", "pattern":["p","w"],
            "key":{"p":{"item":"tconstruct:pattern"},"w":{"type":"forge:difference",
                "base":{"tag":"forge:workbenches"},"subtracted":{"item":"tconstruct:crafting_station"}}},
            "result":{"item":"tconstruct:crafting_station"}});
        let mut tags = HashMap::from([("forge:workbenches".into(), ["minecraft:crafting_table".into(),
            "tconstruct:crafting_station".into(), "test:workbench".into()].into_iter().collect())]);
        let view = resolve(&source, Some(&tags), "small", |_| true, |_| false).unwrap();
        assert_eq!(view["slots"][3], serde_json::json!(["minecraft:crafting_table","test:workbench"]));
        assert_eq!(view["slots"][0], serde_json::json!(["tconstruct:pattern"]));
        tags.get_mut("forge:workbenches").unwrap().remove("test:workbench");
        let refreshed = resolve(&source, Some(&tags), "small", |_| true, |_| false).unwrap();
        assert_eq!(refreshed["slots"][3], serde_json::json!(["minecraft:crafting_table"]));
        tags.get_mut("forge:workbenches").unwrap().remove("minecraft:crafting_table");
        assert!(resolve(&source, Some(&tags), "small", |_| true, |_| false).is_none());
    }
    #[test]
    fn crafting_difference_requires_bounded_supported_subtraction() {
        let mut source = simple_crafting_recipe();
        let difference = serde_json::json!({"type":"forge:difference", "base":["test:a","test:b","test:c"],
            "subtracted":["test:b","test:c"]});
        source["ingredients"][0] = difference.clone();
        let view = resolve(&source, None, "large", |_| true, |_| false).unwrap();
        assert_eq!(view["slots"][0], serde_json::json!(["test:a"]));
        for subtracted in [serde_json::json!({"type":"forge:nbt","item":"test:b","nbt":{}}),
            serde_json::json!({"tag":"test:missing"}), serde_json::json!([]), serde_json::Value::Null] {
            source["ingredients"][0] = difference.clone(); source["ingredients"][0]["subtracted"] = subtracted;
            assert!(resolve(&source, None, "large", |_| true, |_| false).is_none());
        }
        source["ingredients"][0] = difference.clone(); source["ingredients"][0]["extra"] = serde_json::json!(true);
        assert!(resolve(&source, None, "large", |_| true, |_| false).is_none());
        let mut nested = serde_json::json!({"item":"test:a"});
        for _ in 0..10 { nested = serde_json::json!({"type":"forge:difference","base":nested,"subtracted":{"item":"test:b"}}); }
        source["ingredients"][0] = nested;
        assert!(resolve(&source, None, "large", |_| true, |_| false).is_none());
    }

}
