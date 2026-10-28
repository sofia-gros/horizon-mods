use horizon_items::items::ItemStack;

/// `IMaterialValue.getItemsUsed` and `getRemainder` from the pinned original.
/// Recipe loaders require strictly positive cost, needed and value.
fn material_cost(cost: i32, needed: i32, value: i32) -> Option<(i32, i32)> {
    if cost <= 0 || needed <= 0 || value <= 0 { return None; }
    // Java's int multiplication/subtraction wrap, and division truncates to zero.
    let total = cost.wrapping_mul(needed);
    let used = total / value + i32::from(total % value != 0);
    let mut remainder = value.wrapping_sub(total) % value;
    if remainder < 0 { remainder += value; }
    Some((used, remainder))
}

/// `PartRecipe.assemble` changes the part count when a material's value
/// exceeds the part cost and the material recipe has no leftover item.
fn material_output_count(output_count: i32, value: i32, cost: i32, has_leftover: bool) -> Option<i32> {
    if output_count <= 0 || value <= 0 || cost <= 0 { return None; }
    Some(if !has_leftover && value > cost { output_count.wrapping_mul(value) / cost } else { output_count })
}

/// Numeric portion of `FloatToolStat`: updates accumulate additive values,
/// percent is accumulated separately, then the built value is multiplied.
/// This is not yet wired into the prototype's fixed tool statistics.
#[allow(dead_code)]
struct SourceFloatStat {
    base: f32,
    add: f32,
    percent: f32,
    multiply: f32,
}
#[allow(dead_code)]
impl SourceFloatStat {
    fn new(base: f32) -> Self { Self { base, add: 0.0, percent: 0.0, multiply: 1.0 } }
    fn update(&mut self, value: f32) { self.add += value; self.base = 0.0; }
    fn add(&mut self, value: f64) { self.add = (self.add as f64 + value) as f32; }
    // Java's `float += double` and `float *= double` evaluate in double and
    // narrow after each operation. Its final build expression is float-only.
    fn percent(&mut self, value: f64) { self.percent = (self.percent as f64 + value) as f32; }
    fn multiply(&mut self, value: f64) { self.multiply = (self.multiply as f64 * value) as f32; }
    fn build(&self) -> f32 { (self.base + self.add) * (1.0 + self.percent) * self.multiply }
}

/// Source head/handle additions for a pickaxe before tiers, modifiers, clamps,
/// persistence or item components. These remaining stages are not yet ported.
#[allow(dead_code)]
struct SourceHeadStats { durability: i32, mining_speed: f32, attack_damage: f32 }
#[allow(dead_code)]
struct SourceHandleStats { durability: f32, mining_speed: f32, attack_speed: f32, attack_damage: f32 }
#[allow(dead_code)]
fn source_pickaxe_base(head: &SourceHeadStats, handle: &SourceHandleStats) -> [f32; 4] {
    let mut durability = SourceFloatStat::new(1.0);
    let mut mining = SourceFloatStat::new(1.0);
    let mut attack = SourceFloatStat::new(0.0);
    let mut speed = SourceFloatStat::new(1.0);
    durability.update(head.durability as f32);
    mining.update(head.mining_speed);
    attack.update(head.attack_damage);
    durability.percent(handle.durability as f64);
    mining.percent(handle.mining_speed as f64);
    attack.percent(handle.attack_damage as f64);
    speed.percent(handle.attack_speed as f64);
    // `tool_definitions/pickaxe.json`: base stats update after part stats.
    attack.update(0.5);
    speed.update(1.2);
    [durability.build(), mining.build(), attack.build(), speed.build()]
}

/// Ingredient forms used by pinned original material recipes. This parser and
/// matcher are not yet connected to world recipes, tags or item components.
#[allow(dead_code)]
#[derive(Debug)]
enum SourceIngredient {
    Item(String), Tag(String), Any(Vec<Self>), Difference(Box<Self>, Box<Self>),
    Instrument { item: String, exact: Option<String>, ignore: Option<String> },
}
#[allow(dead_code)]
impl SourceIngredient {
    fn parse(v: &serde_json::Value) -> Option<Self> {
        if let Some(list) = v.as_array() {
            if list.is_empty() { return None; }
            return Some(Self::Any(list.iter().map(Self::parse).collect::<Option<_>>()?));
        }
        let o = v.as_object()?;
        let get = |key: &str| o.get(key).and_then(serde_json::Value::as_str).map(str::to_owned);
        match o.get("type").and_then(serde_json::Value::as_str) {
            None => match (get("item"), get("tag")) {
                (Some(item), None) => Some(Self::Item(item)),
                (None, Some(tag)) => Some(Self::Tag(tag)),
                _ => None,
            },
            Some("forge:difference") => Some(Self::Difference(Box::new(Self::parse(o.get("base")?)?), Box::new(Self::parse(o.get("subtracted")?)?))),
            Some("tconstruct:instrument") => {
                let item = get("item")?;
                let exact = get("instrument");
                let ignore = get("ignore");
                if exact.is_none() && ignore.is_none() { return None; }
                Some(Self::Instrument { item, exact, ignore })
            }
            _ => None,
        }
    }

    /// `instrument` is a normalized valid ID, or None if absent or malformed.
    /// Tags and known instruments must come from the current world's registries.
    fn matches(&self, item: &str, instrument: Option<&str>, item_tags: &std::collections::HashMap<String, std::collections::HashSet<String>>, known_instruments: &std::collections::HashSet<String>, instrument_tags: &std::collections::HashMap<String, std::collections::HashSet<String>>) -> bool {
        match self {
            Self::Item(id) => item == id,
            Self::Tag(tag) => item_tags.get(tag).is_some_and(|set| set.contains(item)),
            Self::Any(list) => list.iter().any(|v| v.matches(item, instrument, item_tags, known_instruments, instrument_tags)),
            Self::Difference(base, subtracted) => base.matches(item, instrument, item_tags, known_instruments, instrument_tags) && !subtracted.matches(item, instrument, item_tags, known_instruments, instrument_tags),
            Self::Instrument { item: expected, exact, ignore } => {
                if item != expected { return false; }
                match (instrument, exact, ignore) {
                    (Some(id), Some(expected), _) => id == expected,
                    (Some(id), None, Some(tag)) => known_instruments.contains(id) && !instrument_tags.get(tag).is_some_and(|set| set.contains(id)),
                    (None, None, Some(_)) => true,
                    _ => false,
                }
            }
        }
    }
}

/// Material-recipe load conditions present in the pinned source catalog.
/// Evaluation takes explicit registry and config snapshots; this remains unconnected to
/// world datapacks and recipe reloads.
#[allow(dead_code)]
#[derive(Debug)]
enum SourceCondition {
    TagFilled { registry: String, tag: String },
    TagCombination { registry: String, required: Vec<String>, ignore: Option<String> },
    Or(Vec<Self>),
    Config(String),
}
#[allow(dead_code)]
impl SourceCondition {
    fn parse(v: &serde_json::Value) -> Option<Self> {
        let o = v.as_object()?;
        let text = |name: &str| o.get(name).and_then(serde_json::Value::as_str).map(str::to_owned);
        let registry = text("registry").unwrap_or_else(|| "minecraft:item".into());
        match o.get("type")?.as_str()? {
            "mantle:tag_filled" => Some(Self::TagFilled { registry, tag: text("tag")? }),
            "mantle:tag_combination_filled" => {
                let value = o.get("match")?;
                let required: Vec<String> = if let Some(id) = value.as_str() { vec![id.into()] }
                    else { value.as_array()?.iter().map(|v| v.as_str().map(str::to_owned)).collect::<Option<_>>()? };
                if required.is_empty() { return None; }
                Some(Self::TagCombination { registry, required, ignore: text("ignore") })
            }
            "tconstruct:config" => {
                let property = text("prop")?.to_lowercase();
                const KNOWN: &[&str] = &["spawn_with_book", "gravel_to_flint", "cheaper_netherite_alloy", "wither_bone_drop", "wither_bone_conversion", "slime_recipe_fix", "glass_recipe_fix", "allow_ingotless_alloys", "force_integration_materials", "slimy_loot_chests", "sync_knockback_resistance"];
                if !KNOWN.contains(&property.as_str()) { return None; }
                Some(Self::Config(if property == "slime_recipe_fix" { "glass_recipe_fix".into() } else { property }))
            }
            "forge:or" => {
                let values = o.get("values")?.as_array()?;
                if values.is_empty() { return None; }
                Some(Self::Or(values.iter().map(Self::parse).collect::<Option<_>>()?))
            }
            _ => None,
        }
    }
    fn matches(&self, registries: &std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashSet<String>>>, config: &std::collections::HashMap<String, bool>) -> Option<bool> {
        match self {
            Self::TagFilled { registry, tag } => Some(registries.get(registry).and_then(|tags| tags.get(tag)).is_some_and(|values| !values.is_empty())),
            Self::TagCombination { registry, required, ignore } => {
                let Some(tags) = registries.get(registry) else { return Some(false); };
                let Some(first) = required.first().and_then(|tag| tags.get(tag)) else { return Some(false); };
                Some(first.iter().any(|entry| !ignore.as_ref().and_then(|tag| tags.get(tag)).is_some_and(|values| values.contains(entry))
                    && required[1..].iter().all(|tag| tags.get(tag).is_some_and(|values| values.contains(entry)))))
            }
            Self::Or(values) => {
                let mut unknown = false;
                for value in values {
                    match value.matches(registries, config) {
                        Some(true) => return Some(true),
                        Some(false) => {},
                        None => unknown = true,
                    }
                }
                if unknown { None } else { Some(false) }
            }
            Self::Config(property) if property == "wither_bone_conversion" => Some(false),
            Self::Config(property) => config.get(property).copied(),
        }
    }
}

/// Source ItemOutput forms in the pinned material recipes. Tag selection and
/// NBT decoding are deliberately deferred until the world registry is present.
#[allow(dead_code)]
#[derive(Debug)]
enum SourceOutputKind { Item(String), Tag(String) }
#[allow(dead_code)]
#[derive(Debug)]
struct SourceItemOutput { kind: SourceOutputKind, count: i32, nbt: Option<serde_json::Value> }
#[allow(dead_code)]
impl SourceItemOutput {
    fn parse(value: &serde_json::Value) -> Option<Self> {
        if let Some(id) = value.as_str() {
            if id == "minecraft:air" { return None; }
            return Some(Self { kind: SourceOutputKind::Item(id.into()), count: 1, nbt: None });
        }
        let obj = value.as_object()?;
        let count = match obj.get("count") { Some(value) => i32::try_from(value.as_i64()?).ok()?, None => 1 };
        if count <= 0 { return None; }
        let kind = if let Some(tag) = obj.get("tag").and_then(serde_json::Value::as_str) {
            SourceOutputKind::Tag(tag.into())
        } else {
            let id = obj.get("item")?.as_str()?;
            if id == "minecraft:air" { return None; }
            SourceOutputKind::Item(id.into())
        };
        Some(Self { kind, count, nbt: obj.get("nbt").cloned() })
    }
    /// `IMaterialValue.getLeftover`: Java int multiply then truncating divide.
    fn count_for_remainder(&self, remainder: i32, needed: i32) -> Option<i32> {
        if remainder <= 0 || needed <= 0 { return None; }
        let count = self.count.wrapping_mul(remainder) / needed;
        (count > 0).then_some(count)
    }
}

/// Pinned `MaterialRecipe` fields used for lookup and cost calculation.
/// Leftover output stays unresolved until Mantle's tag preference and NBT
/// rules are connected to the current world. Not used by gameplay yet.
#[allow(dead_code)]
struct SourceMaterialRecipe {
    ingredient: SourceIngredient,
    material: String,
    value: i32,
    needed: i32,
    leftover: Option<SourceItemOutput>,
    conditions: Vec<SourceCondition>,
}
#[allow(dead_code)]
impl SourceMaterialRecipe {
    fn parse(v: &serde_json::Value) -> Option<Self> {
        if v.get("type")?.as_str()? != "tconstruct:material" { return None; }
        let positive = |name: &str| -> Option<i32> {
            let n = match v.get(name) { Some(value) => value.as_i64()?, None => 1 };
            i32::try_from(n).ok().filter(|n| *n > 0)
        };
        let value = positive("value")?;
        let conditions = match v.get("conditions") {
            Some(list) => list.as_array()?.iter().map(SourceCondition::parse).collect::<Option<_>>()?,
            None => Vec::new(),
        };
        Some(Self {
            ingredient: SourceIngredient::parse(v.get("ingredient")?)?,
            material: v.get("material")?.as_str()?.to_owned(),
            value,
            needed: positive("needed")?,
            // Original MaterialRecipe discards leftover for values of one.
            leftover: if value > 1 { v.get("leftover").and_then(SourceItemOutput::parse) } else { None },
            conditions,
        })
    }
    fn enabled(&self, registries: &std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashSet<String>>>, config: &std::collections::HashMap<String, bool>) -> Option<bool> {
        let mut unknown = false;
        for condition in &self.conditions {
            match condition.matches(registries, config) {
                Some(false) => return Some(false),
                Some(true) => {},
                None => unknown = true,
            }
        }
        if unknown { None } else { Some(true) }
    }
}

/// `TinkerStationRepairRecipe.repairFromSlot`: Java performs the division and
/// multiplication as floats, rounds the item count up, then truncates the
/// repaired durability. The caller supplies the material/modifier-derived rate.
fn repair_from_slot(needed: i32, stack: i32, durability_per_item: f32) -> (i32, i32) {
    if needed <= 0 || stack <= 0 || !(durability_per_item > 0.0) { return (0, 0); }
    let applied = stack.min(((needed as f32) / durability_per_item).ceil() as i32);
    (applied, (applied as f32 * durability_per_item) as i32)
}

/// The original `IMaterialItem` stores the chosen material under `Material`.
/// Horizon 26.3 keeps that legacy item tag inside `minecraft:custom_data`.
fn part_with_material(id: &str, material: &str) -> ItemStack {
    let mut part = ItemStack::new(id, 1);
    part.data_mut().set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::compound([
        ("Material", horizon_world::nbt::Tag::String(material.into())),
    ])));
    let (model, title) = match (id, material) {
        ("tconstruct:pick_head", "tconstruct:flint") => ("tconstruct:flint_pick_head", "Flint Pick Head"),
        ("tconstruct:pick_head", "tconstruct:wood") => ("tconstruct:wood_pick_head", "Wooden Pick Head"),
        ("tconstruct:tool_handle", "tconstruct:flint") => ("tconstruct:flint_handle", "Flint Tool Handle"),
        ("tconstruct:tool_handle", "tconstruct:wood") => ("tconstruct:wood_handle", "Wooden Tool Handle"),
        ("tconstruct:tool_binding", "tconstruct:flint") => ("tconstruct:flint_binding", "Flint Tool Binding"),
        ("tconstruct:tool_binding", "tconstruct:wood") => ("tconstruct:wood_binding", "Wooden Tool Binding"),
        _ => return part,
    };
    part.data_mut().set_other("minecraft:item_model", Some(horizon_world::nbt::Tag::String(model.into())));
    part.data_mut().set_other("minecraft:item_name", Some(horizon_world::nbt::Tag::String(title.into())));
    part
}

/// Legacy prototype IDs encode a single material. If a Material tag is
/// present, reject a conflicting value rather than silently assembling it.
fn part_matches_material(stack: &ItemStack, expected: &str) -> bool {
    use horizon_world::nbt::Tag;
    match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|v| v.get("Material")) {
        None => matches!(&*stack.item, "tconstruct:flint_pick_head" | "tconstruct:wood_handle" | "tconstruct:wood_binding"),
        Some(Tag::String(value)) => value.to_string_lossy() == expected,
        Some(_) => false,
    }
}

/// The fixed prototype has a flint head. Original `MaterialStatsModule`
/// checks the tool's material list before accepting a repair material.
fn fixed_pickaxe_has_flint_head(stack: &ItemStack) -> bool {
    use horizon_world::nbt::Tag;
    if &*stack.item == "tconstruct:flint_pickaxe" { return true; }
    match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_materials")) {
        Some(Tag::List(materials)) => matches!(materials.first(), Some(Tag::String(id)) if id.to_string_lossy() == "tconstruct:flint"),
        _ => false,
    }
}

fn fixed_pickaxe_with_materials() -> ItemStack {
    use horizon_world::nbt::Tag;
    let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
    let materials = ["tconstruct:flint", "tconstruct:wood", "tconstruct:wood"]
        .into_iter().map(|id| Tag::String(id.into())).collect();
    tool.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_materials", Tag::List(materials))])));
    tool.data_mut().set_other("minecraft:item_model", Some(Tag::String("tconstruct:flint_pickaxe".into())));
    tool.data_mut().set_other("minecraft:item_name", Some(Tag::String("Flint Pickaxe".into())));
    tool
}

/// Apply a single result pickup to the authoritative slots and cursor.
/// Inputs are checked before any mutation; tool data is carried by cloning it.
pub(super) fn take(stacks: &mut [Option<ItemStack>], carried: &mut Option<ItemStack>, station: bool, action: i16) -> bool {
    if stacks.len() < 27 { return false; }
    let is = |slot: usize, item: &str, count: u32| stacks.get(slot).and_then(Option::as_ref).is_some_and(|s| &*s.item == item && s.count >= count);
    let (output, consume): (ItemStack, Vec<(usize, u32)>) = if !station {
        let (cost, result) = match action {
            24 => (2, "tconstruct:pick_head"),
            25 => (1, "tconstruct:tool_handle"),
            26 => (1, "tconstruct:tool_binding"),
            _ => return false,
        };
        let (material, material_id) = match stacks[1].as_ref().map(|s| &*s.item) {
            Some("minecraft:flint") => ("minecraft:flint", "tconstruct:flint"),
            Some("minecraft:oak_planks") => ("minecraft:oak_planks", "tconstruct:wood"),
            _ => return false,
        };
        let Some((amount, remainder)) = material_cost(cost, 1, 1) else { return false; };
        let Some(count) = material_output_count(1, 1, cost, false) else { return false; };
        // Only unit-value materials and one-item results are currently wired in.
        if count != 1 { return false; }
        // The current two inputs have unit value and no remainder. Other
        // material recipes and their remainder items are not yet supported.
        let Ok(amount) = u32::try_from(amount) else { return false; };
        if amount == 0 || remainder != 0 || !is(0, "tconstruct:pattern", 1) || !is(1, material, amount) { return false; }
        (part_with_material(result, material_id), vec![(0, 1), (1, amount)])
    } else if action == 24 {
        if !(is(0, "tconstruct:pick_head", 1) || is(0, "tconstruct:flint_pick_head", 1)) || !(is(1, "tconstruct:tool_handle", 1) || is(1, "tconstruct:wood_handle", 1)) || !(is(2, "tconstruct:tool_binding", 1) || is(2, "tconstruct:wood_binding", 1)) { return false; }
        for (slot, material) in [(0, "tconstruct:flint"), (1, "tconstruct:wood"), (2, "tconstruct:wood")] {
            if !part_matches_material(stacks[slot].as_ref().unwrap(), material) { return false; }
        }
        (fixed_pickaxe_with_materials(), vec![(0, 1), (1, 1), (2, 1)])
    } else if action == 25 {
        if !(is(0, "tconstruct:pickaxe", 1) || is(0, "tconstruct:flint_pickaxe", 1)) { return false; }
        let mut tool = stacks[0].as_ref().unwrap().clone();
        if tool.count != 1 || tool.damage == 0 || !fixed_pickaxe_has_flint_head(&tool) { return false; }
        if (1..=2).any(|i| stacks[i].as_ref().is_some_and(|s| &*s.item != "minecraft:flint" || s.count == 0)) { return false; }
        let Ok(mut needed) = i32::try_from(tool.damage) else { return false; };
        let mut consume = vec![(0, 1)];
        for slot in 1..=2 {
            if let Some(stack) = &stacks[slot] {
                let Ok(available) = i32::try_from(stack.count) else { return false; };
                // Fixed prototype rate; material stats and modifier hooks still need porting.
                let (count, repaired) = repair_from_slot(needed, available, 56.0);
                if count > 0 { consume.push((slot, count as u32)); needed = (needed - repaired).max(0); }
            }
        }
        if consume.len() == 1 { return false; }
        tool.damage = needed as u32;
        (tool, consume)
    } else { return false; };
    if let Some(existing) = carried.as_ref() {
        let max = if &*output.item == "tconstruct:flint_pickaxe" || &*output.item == "tconstruct:pickaxe" { 1 } else { 64 };
        let mut compare = existing.clone();
        compare.count = output.count;
        if compare != output || existing.count >= max { return false; }
    }
    for (slot, count) in consume {
        let stack = stacks[slot].as_mut().unwrap();
        stack.count -= count;
        if stack.count == 0 { stacks[slot] = None; }
    }
    match carried.as_mut() {
        Some(stack) => stack.count += output.count,
        None => *carried = Some(output),
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_material_cost_rounds_up_and_tracks_remainder() {
        assert_eq!(material_cost(2, 1, 1), Some((2, 0)));
        assert_eq!(material_cost(1, 2, 3), Some((1, 1)));
        assert_eq!(material_cost(2, 1, 3), Some((1, 1)));
        assert_eq!(material_cost(2, 3, 2), Some((3, 0)));
        assert_eq!(material_cost(i32::MAX, 2, 3), Some((1, 2)));
        assert_eq!(material_cost(1, 1, 0), None);
    }
    #[test]
    fn original_part_count_depends_on_material_value_and_leftover() {
        assert_eq!(material_output_count(1, 1, 2, false), Some(1));
        assert_eq!(material_output_count(1, 3, 1, false), Some(3));
        assert_eq!(material_output_count(2, 5, 2, false), Some(5));
        assert_eq!(material_output_count(1, 3, 2, true), Some(1));
        assert_eq!(material_output_count(1, 3, 0, false), None);
    }
    #[test]
    fn original_material_ingredients_cover_tags_difference_and_instruments() {
        use std::collections::{HashMap, HashSet};
        let mut tags = HashMap::new();
        tags.insert("minecraft:planks".into(), HashSet::from(["minecraft:oak_planks".into(), "minecraft:warped_planks".into()]));
        tags.insert("tconstruct:wood_variants/planks".into(), HashSet::from(["minecraft:warped_planks".into()]));
        let difference = SourceIngredient::parse(&serde_json::json!({"type":"forge:difference", "base":{"tag":"minecraft:planks"}, "subtracted":{"tag":"tconstruct:wood_variants/planks"}})).unwrap();
        let known = HashSet::from(["minecraft:ponder_goat_horn".into(), "minecraft:sing_goat_horn".into()]);
        let mut instruments = HashMap::new();
        instruments.insert("tconstruct:variant_horns".into(), HashSet::from(["minecraft:sing_goat_horn".into()]));
        assert!(difference.matches("minecraft:oak_planks", None, &tags, &known, &instruments));
        assert!(!difference.matches("minecraft:warped_planks", None, &tags, &known, &instruments));
        let exact = SourceIngredient::parse(&serde_json::json!({"type":"tconstruct:instrument", "item":"minecraft:goat_horn", "instrument":"minecraft:sing_goat_horn"})).unwrap();
        assert!(exact.matches("minecraft:goat_horn", Some("minecraft:sing_goat_horn"), &tags, &known, &instruments));
        assert!(!exact.matches("minecraft:goat_horn", None, &tags, &known, &instruments));
        let fallback = SourceIngredient::parse(&serde_json::json!({"type":"tconstruct:instrument", "item":"minecraft:goat_horn", "ignore":"tconstruct:variant_horns"})).unwrap();
        assert!(fallback.matches("minecraft:goat_horn", None, &tags, &known, &instruments));
        assert!(fallback.matches("minecraft:goat_horn", Some("minecraft:ponder_goat_horn"), &tags, &known, &instruments));
        assert!(!fallback.matches("minecraft:goat_horn", Some("minecraft:sing_goat_horn"), &tags, &known, &instruments));
        assert!(!fallback.matches("minecraft:goat_horn", Some("example:unknown"), &tags, &known, &instruments));
        let alternatives = SourceIngredient::parse(&serde_json::json!([{"tag":"forge:storage_blocks/copper"}, {"item":"minecraft:waxed_copper_block"}])).unwrap();
        assert!(alternatives.matches("minecraft:waxed_copper_block", None, &tags, &known, &instruments));
        assert!(SourceIngredient::parse(&serde_json::json!({"type":"example:unknown"})).is_none());
    }
    #[test]
    fn source_tag_conditions_use_intersection_exclusion_and_or() {
        use std::collections::{HashMap, HashSet};
        let mut items = HashMap::new();
        items.insert("minecraft:planks".into(), HashSet::from(["minecraft:oak_planks".into(), "minecraft:warped_planks".into()]));
        items.insert("tconstruct:wood_variants/planks".into(), HashSet::from(["minecraft:warped_planks".into()]));
        items.insert("test:only_warped".into(), HashSet::from(["minecraft:warped_planks".into()]));
        let mut registries = HashMap::new();
        registries.insert("minecraft:item".into(), items);
        let condition = SourceCondition::parse(&serde_json::json!({"type":"mantle:tag_combination_filled", "match":"minecraft:planks", "ignore":"tconstruct:wood_variants/planks"})).unwrap();
        let mut config = HashMap::new();
        assert_eq!(condition.matches(&registries, &config), Some(true));
        let intersection = SourceCondition::parse(&serde_json::json!({"type":"mantle:tag_combination_filled", "match":["minecraft:planks", "test:only_warped"], "ignore":"tconstruct:wood_variants/planks"})).unwrap();
        assert_eq!(intersection.matches(&registries, &config), Some(false));
        let or = SourceCondition::parse(&serde_json::json!({"type":"forge:or", "values":[{"type":"mantle:tag_filled","tag":"test:missing"}, {"type":"mantle:tag_filled","tag":"minecraft:planks"}]})).unwrap();
        assert_eq!(or.matches(&registries, &config), Some(true));
        let other = SourceCondition::parse(&serde_json::json!({"type":"mantle:tag_filled", "registry":"minecraft:instrument", "tag":"minecraft:planks"})).unwrap();
        assert_eq!(other.matches(&registries, &config), Some(false));
        let configured = SourceCondition::parse(&serde_json::json!({"type":"tconstruct:config", "prop":"FORCE_INTEGRATION_MATERIALS"})).unwrap();
        assert_eq!(configured.matches(&registries, &config), None);
        let config_or_tag = SourceCondition::parse(&serde_json::json!({"type":"forge:or", "values":[{"type":"tconstruct:config","prop":"force_integration_materials"}, {"type":"mantle:tag_filled","tag":"test:missing"}]})).unwrap();
        assert_eq!(config_or_tag.matches(&registries, &config), None);
        config.insert("force_integration_materials".into(), false);
        assert_eq!(config_or_tag.matches(&registries, &config), Some(false));
        config.insert("force_integration_materials".into(), true);
        assert_eq!(configured.matches(&registries, &config), Some(true));
        config.insert("glass_recipe_fix".into(), true);
        let alias = SourceCondition::parse(&serde_json::json!({"type":"tconstruct:config", "prop":"slime_recipe_fix"})).unwrap();
        assert_eq!(alias.matches(&registries, &config), Some(true));
        config.insert("wither_bone_conversion".into(), true);
        let removed = SourceCondition::parse(&serde_json::json!({"type":"tconstruct:config", "prop":"wither_bone_conversion"})).unwrap();
        assert_eq!(removed.matches(&registries, &config), Some(false));
        assert!(SourceCondition::parse(&serde_json::json!({"type":"tconstruct:config", "prop":"unknown"})).is_none());
        assert!(SourceCondition::parse(&serde_json::json!({"type":"forge:or", "values":[]})).is_none());
    }
    #[test]
    fn parses_all_pinned_material_recipe_ingredients() {
        use std::io::Read;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../docs/upstream-recipes.json.gz");
        let file = std::fs::File::open(path).expect("pinned reference catalog");
        let mut json = String::new();
        flate2::read::GzDecoder::new(file).read_to_string(&mut json).unwrap();
        let catalog: serde_json::Value = serde_json::from_str(&json).unwrap();
        let mut count = 0;
        let mut condition_count = 0;
        let mut leftover_count = 0;
        for recipe in catalog["groups"]["materials"].as_array().unwrap() {
            let data = &recipe["data"];
            if data["type"] == "tconstruct:material" {
                let parsed = SourceMaterialRecipe::parse(data).unwrap_or_else(|| panic!("{}", recipe["source"]));
                assert!(parsed.value > 0 && parsed.needed > 0 && !parsed.material.is_empty());
                if data.get("leftover").is_some() && parsed.value > 1 {
                    assert!(parsed.leftover.is_some(), "{}", recipe["source"]);
                    leftover_count += 1;
                }
                for condition in data["conditions"].as_array().into_iter().flatten() {
                    assert!(SourceCondition::parse(condition).is_some(), "{}", recipe["source"]);
                    condition_count += 1;
                }
                count += 1;
            }
        }
        assert_eq!(count, 217);
        assert_eq!(condition_count, 44);
        assert_eq!(leftover_count, 47);
    }
    #[test]
    fn source_material_recipe_defaults_and_deferred_leftovers() {
        let single = SourceMaterialRecipe::parse(&serde_json::json!({"type":"tconstruct:material", "ingredient":{"item":"minecraft:flint"}, "material":"tconstruct:flint", "leftover":"minecraft:stick"})).unwrap();
        assert_eq!((single.value, single.needed), (1, 1));
        assert!(single.leftover.is_none());
        let block = SourceMaterialRecipe::parse(&serde_json::json!({"type":"tconstruct:material", "ingredient":{"tag":"forge:storage_blocks/copper"}, "material":"tconstruct:copper", "value":9, "leftover":{"tag":"forge:ingots/copper", "count":1}})).unwrap();
        assert_eq!(material_cost(2, block.needed, block.value), Some((1, 7)));
        let output = block.leftover.unwrap();
        assert!(matches!(&output.kind, SourceOutputKind::Tag(tag) if tag == "forge:ingots/copper"));
        assert_eq!(output.count_for_remainder(7, 1), Some(7));
        assert_eq!(output.count_for_remainder(1, 2), None);
        assert!(SourceMaterialRecipe::parse(&serde_json::json!({"type":"tconstruct:material", "ingredient":{"item":"minecraft:flint"}, "material":"tconstruct:flint", "needed":0})).is_none());
    }
    #[test]
    fn original_head_and_handle_durability_uses_float_then_integer() {
        for (head, handle, expected) in [(85.0, 0.0, 85), (85.0, -0.15_f32, 72), (60.0, 0.0, 60), (60.0, -0.15_f32, 51)] {
            let mut stat = SourceFloatStat::new(1.0);
            stat.update(head);
            stat.percent(handle as f64);
            assert_eq!(stat.build() as i32, expected);
        }
        let mut stat = SourceFloatStat::new(1.0);
        stat.update(85.0);
        stat.percent(-0.15_f32 as f64);
        stat.add(5.0);
        stat.multiply(1.5);
        assert_eq!(stat.build(), 114.75);
    }
    #[test]
    fn original_pickaxe_part_stats_preserve_float_precision_before_modifiers() {
        let flint = SourceHeadStats { durability: 85, mining_speed: 3.5, attack_damage: 1.25 };
        let wood_handle = SourceHandleStats { durability: 0.0, mining_speed: 0.0, attack_speed: 0.0, attack_damage: 0.0 };
        assert_eq!(source_pickaxe_base(&flint, &wood_handle), [85.0, 3.5, 1.75, 1.2]);
        let flint_handle = SourceHandleStats { durability: -0.15, mining_speed: 0.0, attack_speed: 0.0, attack_damage: 0.1 };
        let stats = source_pickaxe_base(&flint, &flint_handle);
        assert_eq!(stats[0], 72.25);
        assert_eq!(stats[2].to_bits(), 0x3ff66667);
    }
    #[test]
    fn original_repair_rounds_count_before_truncating_total() {
        let per_item = (85.0_f32 / 3.0) * 2.0;
        assert_eq!(repair_from_slot(85, 2, per_item), (2, 113));
        assert_eq!(repair_from_slot(85, 1, per_item), (1, 56));
        assert_eq!(repair_from_slot(29, 1, per_item), (1, 56));
        assert_eq!(repair_from_slot(85, 2, 0.0), (0, 0));
    }
    #[test]
    fn part_take_is_atomic_and_consumes_default_pattern() {
        let mut slots = vec![None; 27];
        slots[0] = Some(ItemStack::new("tconstruct:pattern", 1));
        slots[1] = Some(ItemStack::new("minecraft:flint", 2));
        let mut cursor = Some(ItemStack::new("minecraft:stick", 1));
        let before = slots.clone();
        assert!(!take(&mut slots, &mut cursor, false, 24));
        assert_eq!(slots, before);
        cursor = None;
        assert!(take(&mut slots, &mut cursor, false, 24));
        assert_eq!(cursor.as_ref().unwrap().item.as_ref(), "tconstruct:pick_head");
        assert_eq!(cursor.as_ref().unwrap().data().and_then(|d| d.other("minecraft:custom_data")), Some(&horizon_world::nbt::Tag::compound([("Material", horizon_world::nbt::Tag::String("tconstruct:flint".into()))])));
        assert!(slots[1].is_none());
        assert!(slots[0].is_none());
        cursor = None;
        assert!(!take(&mut slots, &mut cursor, false, 24));
        assert!(cursor.is_none());
    }
    #[test]
    fn both_supported_materials_make_each_fixed_part_with_source_cost() {
        for (item, id) in [("minecraft:flint", "tconstruct:flint"), ("minecraft:oak_planks", "tconstruct:wood")] {
            for (action, output, cost) in [(24, "tconstruct:pick_head", 2), (25, "tconstruct:tool_handle", 1), (26, "tconstruct:tool_binding", 1)] {
                let mut slots = vec![None; 27];
                slots[0] = Some(ItemStack::new("tconstruct:pattern", 1));
                slots[1] = Some(ItemStack::new(item, cost));
                let mut cursor = None;
                assert!(take(&mut slots, &mut cursor, false, action));
                let part = cursor.unwrap();
                assert_eq!(&*part.item, output);
                assert!(part_matches_material(&part, id));
                assert!(slots[0].is_none() && slots[1].is_none());
            }
        }
    }
    #[test]
    fn preview_does_not_consume_and_rechecks_material() {
        let mut slots = vec![None; 27];
        slots[0] = Some(ItemStack::new("tconstruct:pattern", 1));
        slots[1] = Some(ItemStack::new("minecraft:flint", 2));
        let before = slots.clone();
        assert_eq!(part_preview(&slots, 0).unwrap().item.as_ref(), "tconstruct:pick_head");
        assert_eq!(slots, before);
        assert_eq!(part_preview(&slots, 1).unwrap().item.as_ref(), "tconstruct:tool_handle");
        slots[1] = Some(ItemStack::new("minecraft:flint", 1));
        assert!(part_preview(&slots, 0).is_none());
    }
    #[test]
    fn assembly_retains_materials_and_rejects_conflicting_identity() {
        assert!(!part_matches_material(&ItemStack::new("tconstruct:pick_head", 1), "tconstruct:flint"));
        assert!(part_matches_material(&ItemStack::new("tconstruct:flint_pick_head", 1), "tconstruct:flint"));
        let mut slots = vec![None; 27];
        slots[0] = Some(part_with_material("tconstruct:flint_pick_head", "tconstruct:flint"));
        slots[1] = Some(part_with_material("tconstruct:wood_handle", "tconstruct:wood"));
        slots[2] = Some(part_with_material("tconstruct:wood_binding", "tconstruct:flint"));
        let before = slots.clone();
        let mut cursor = None;
        assert!(!take(&mut slots, &mut cursor, true, 24));
        assert_eq!(slots, before);
        slots[2] = Some(part_with_material("tconstruct:wood_binding", "tconstruct:wood"));
        assert!(take(&mut slots, &mut cursor, true, 24));
        let data = cursor.unwrap();
        let list = data.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|v| v.get("tic_materials"));
        assert_eq!(list, Some(&horizon_world::nbt::Tag::List(["tconstruct:flint", "tconstruct:wood", "tconstruct:wood"].into_iter().map(|id| horizon_world::nbt::Tag::String(id.into())).collect())));
    }
    #[test]
    fn repair_rejects_missing_or_wrong_head_material_without_mutating() {
        let mut slots = vec![None; 27];
        let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
        tool.damage = 50;
        slots[0] = Some(tool);
        slots[1] = Some(ItemStack::new("minecraft:flint", 1));
        let before = slots.clone();
        let mut cursor = None;
        assert!(!take(&mut slots, &mut cursor, true, 25));
        assert_eq!(slots, before);
        let mut wrong = fixed_pickaxe_with_materials();
        wrong.damage = 50;
        wrong.data_mut().set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::compound([
            ("tic_materials", horizon_world::nbt::Tag::List(vec![horizon_world::nbt::Tag::String("tconstruct:wood".into())]))
        ])));
        slots[0] = Some(wrong);
        let before = slots.clone();
        assert!(!take(&mut slots, &mut cursor, true, 25));
        assert_eq!(slots, before);
        assert!(cursor.is_none());
    }
    #[test]
    fn repair_preserves_name_and_unknown_components() {
        let mut slots = vec![None; 27];
        let mut tool = ItemStack::new("tconstruct:flint_pickaxe", 1);
        tool.damage = 85;
        tool.data_mut().custom_name = Some("Named tool".into());
        tool.data_mut().other.push(("example:custom".into(), horizon_world::nbt::Tag::Int(17)));
        slots[0] = Some(tool.clone());
        slots[1] = Some(ItemStack::new("minecraft:flint", 1));
        let mut cursor = None;
        assert!(take(&mut slots, &mut cursor, true, 25));
        let repaired = cursor.as_ref().unwrap();
        assert_eq!(repaired.damage, 29);
        assert_eq!(repaired.extra, tool.extra);
        assert!(slots[0].is_none() && slots[1].is_none());
        assert!(!take(&mut slots, &mut None, true, 25));
    }
    #[test]
    fn repair_rechecks_split_inputs_and_never_consumes_excess() {
        let mut slots = vec![None; 27];
        let mut tool = ItemStack::new("tconstruct:flint_pickaxe", 1);
        tool.damage = 85;
        slots[0] = Some(tool);
        slots[1] = Some(ItemStack::new("minecraft:flint", 1));
        slots[2] = Some(ItemStack::new("minecraft:flint", 3));
        let mut cursor = None;
        assert!(take(&mut slots, &mut cursor, true, 25));
        assert_eq!(cursor.unwrap().damage, 0);
        assert!(slots[1].is_none());
        assert_eq!(slots[2].as_ref().unwrap().count, 2);
    }

}

/// The selected recipe has no result unless the current inputs still match.
pub(super) fn part_preview(stacks: &[Option<ItemStack>], selected: i16) -> Option<ItemStack> {
    let mut copy = stacks.to_vec();
    let mut result = None;
    take(&mut copy, &mut result, false, selected + 24).then_some(result).flatten()
}
