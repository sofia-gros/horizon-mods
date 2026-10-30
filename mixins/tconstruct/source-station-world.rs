//! Station recipe selection using the current world's source data.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use super::tconstruct_source_station_recipes::{BuildingMatch, BuildingRecipe};

/// Resolve the registered pickaxe against source recipes and material stats.
pub(super) fn building_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>]) -> Option<(BuildingMatch, ItemStack)> {
    let recipes = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let definition = materials.get("tconstruct:tinkering/tool_definitions/pickaxe")?.last()?;
    for bytes in recipes.values() {
        let Some(recipe) = BuildingRecipe::parse(bytes) else { continue; };
        if recipe.result != "tconstruct:pickaxe" { continue; }
        let Some(parts) = recipe.parts(definition) else { continue; };
        let Some(inputs) = slots.get(..3) else { return None; };
        let Some(matched) = recipe.match_inputs(&parts, parts.len(), inputs, |stack| {
            match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|tag| tag.get("Material")) {
                Some(Tag::String(material)) => Some(material.to_string_lossy().to_string()),
                _ => None,
            }
        }, |part, material| {
            super::tconstruct_source_builder_world::supported_material(world, material)
                .is_ok_and(|(_, supported)| supported.contains(part))
        }, |_, _| false) else { continue; };
        if matched.result_count != 1 { return None; }
        let output = assemble(world, definition, &matched.materials, None)?;
        return Some((matched, output));
    }
    None
}

fn assemble(world: &super::tconstruct_world::World, definition: &[u8], identities: &[String], original: Option<&ItemStack>) -> Option<ItemStack> {
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let mut stat_maps: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();
    for material in identities {
        let (namespace, id) = material.split_once(':')?;
        let id = id.split('#').next()?;
        let layers = materials.get(&format!("{namespace}:tinkering/materials/stats/{id}"))?;
        let merged = super::tconstruct_tx::source_merge_material_stats(layers).ok()?;
        stat_maps.push(merged.into_iter().map(|(key, fields)| (key, serde_json::Value::Object(fields))).collect());
    }
    let stats = super::tconstruct_source_station_stats::base_stats(definition, &stat_maps)?;
    let durability = stats.durability as i32;
    if durability < 1 { return None; }
    let head = stat_maps.first()?.get("tconstruct:head")?.as_object()?;
    let tier = match head.get("mining_tier")?.as_str()?.strip_prefix("minecraft:")? {
        "wood" => "wooden",
        tier @ ("stone" | "iron" | "gold" | "diamond" | "netherite") => tier,
        _ => return None,
    };
    let mut output = original.cloned().unwrap_or_else(|| ItemStack::new("tconstruct:pickaxe", 1));
    let mut custom = output.data().and_then(|data| data.other("minecraft:custom_data")).cloned()
        .unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
    let Tag::Compound(ref mut fields) = custom else { return None; };
    fields.insert("tic_materials".into(), Tag::List(identities.iter().map(|m| Tag::String(m.as_str().into())).collect()));
    fields.insert("tic_max_damage".into(), Tag::Int(durability));
    fields.insert("tic_attack_damage".into(), Tag::Float(stats.attack_damage.to_bits()));
    fields.insert("tic_attack_speed".into(), Tag::Float(stats.attack_speed.to_bits()));
    output.data_mut().set_other("minecraft:custom_data", Some(custom));
    output.data_mut().set_other("minecraft:max_damage", Some(Tag::Int(durability)));
    output.data_mut().set_other("minecraft:tool", Some(Tag::compound([
        ("rules", Tag::List(vec![
        Tag::compound([("blocks", Tag::String(format!("#minecraft:incorrect_for_{tier}_tool").as_str().into())), ("correct_for_drops", Tag::Byte(0))]),
        Tag::compound([("blocks", Tag::String("#minecraft:mineable/pickaxe".into())), ("speed", Tag::Float(stats.mining_speed.to_bits())), ("correct_for_drops", Tag::Byte(1))]),
        ])),
        ("default_mining_speed", Tag::Float(1.0f32.to_bits())),
        ("damage_per_block", Tag::Int(1)),
    ])));
    Some(output)
}

/// Replace one part, rebuilding base stats and preserving the original stack.
pub(super) fn swap_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>]) -> Option<(ItemStack, Vec<(usize, u32)>)> {
    let tool = slots.first()?.as_ref()?;
    if tool.item.as_ref() != "tconstruct:pickaxe" || tool.count != 1 { return None; }
    let mut inputs = (1..=2).filter_map(|index| slots.get(index).and_then(Option::as_ref).map(|stack| (index, stack)));
    let (slot, part) = inputs.next()?;
    if inputs.next().is_some() { return None; }
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let definition = materials.get("tconstruct:tinkering/tool_definitions/pickaxe")?.last()?;
    let recipe = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?.values()
        .filter_map(|bytes| BuildingRecipe::parse(bytes)).find(|recipe| recipe.result == "tconstruct:pickaxe")?;
    let parts = recipe.parts(definition)?;
    let index = if parts.get(slot - 1).is_some_and(|id| id == &*part.item) { slot - 1 }
        else { parts.iter().position(|id| id == &*part.item)? };
    let Tag::String(material) = part.data()?.other("minecraft:custom_data")?.get("Material")? else { return None; };
    let material = material.to_string_lossy().to_string();
    if !super::tconstruct_source_builder_world::supported_material(world, &material).ok()?.1.contains(&*part.item) { return None; }
    let Tag::List(values) = tool.data()?.other("minecraft:custom_data")?.get("tic_materials")? else { return None; };
    let mut identities = values.iter().map(|value| match value { Tag::String(id) => Some(id.to_string_lossy().to_string()), _ => None }).collect::<Option<Vec<_>>>()?;
    let prior = identities.get_mut(index)?;
    let changed = *prior != material;
    *prior = material.clone();
    let part_name = part.item.strip_prefix("tconstruct:")?;
    let cost_index = super::tconstruct_source_cast_info::PARTS.iter().position(|id| *id == part_name)?;
    let cost = *world.chunks.__horizon_mixins_tconstruct_source_cast_costs().as_ref()?.get(cost_index)?;
    let repair = if index == 0 && cost > 0 {
        let (namespace, name) = material.split_once(':')?;
        let layers = materials.get(&format!("{namespace}:tinkering/materials/stats/{}", name.split('#').next()?))?;
        let stats = super::tconstruct_tx::source_merge_material_stats(layers).ok()?;
        (stats.get("tconstruct:head")?.get("durability")?.as_f64()? as f32 * cost as f32 / 3.0) as u32
    } else { 0 };
    if !changed && (tool.damage == 0 || repair == 0) { return None; }
    let mut output = assemble(world, definition, &identities, Some(tool))?;
    super::tconstruct_tx::fixed_set_damage(&mut output, tool.damage.saturating_sub(repair));
    Some((output, vec![(0, 1), (slot, 1)]))
}

/// Compute the pickaxe's material repair from the world's material recipes.
pub(super) fn repair_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>]) -> Option<(ItemStack, Vec<(usize, u32)>)> {
    let mut tool = slots.first()?.as_ref()?.clone();
    if tool.item.as_ref() != "tconstruct:pickaxe" || tool.count != 1 || tool.damage == 0 { return None; }
    let custom = tool.data()?.other("minecraft:custom_data")?;
    let Tag::List(ids) = custom.get("tic_materials")? else { return None; };
    let Tag::String(head_id) = ids.first()? else { return None; };
    let head_id = head_id.to_string_lossy().to_string();
    let (namespace, path) = head_id.split_once(':')?;
    let path = path.split('#').next()?;
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let stat_layers = materials.get(&format!("{namespace}:tinkering/materials/stats/{path}"))?;
    let stats = super::tconstruct_tx::source_merge_material_stats(stat_layers).ok()?;
    let durability = stats.get("tconstruct:head")?.get("durability")?.as_f64()? as f32;
    let mut remaining = i32::try_from(tool.damage).ok()?;
    let mut consumed = vec![(0, 1)];
    for index in 1..=2 {
        let Some(stack) = slots.get(index).and_then(Option::as_ref) else { continue; };
        let material = super::tconstruct_source_builder_world::lookup_material(world, stack).ok()??;
        if material.material != head_id || material.needed <= 0 { return None; }
        let amount = durability * material.value as f32 / (3.0 * material.needed as f32);
        let (count, repaired) = super::tconstruct_tx::repair_from_slot(remaining, i32::try_from(stack.count).ok()?, amount);
        if count > 0 { consumed.push((index, count as u32)); remaining = (remaining - repaired).max(0); }
    }
    if consumed.len() == 1 { return None; }
    super::tconstruct_tx::fixed_set_damage(&mut tool, remaining as u32);
    Some((tool, consumed))
}

pub(super) fn take_repair(slots: &mut [Option<ItemStack>], carried: &mut Option<ItemStack>, output: ItemStack, consumed: &[(usize, u32)]) -> bool {
    if carried.is_some() || consumed.iter().any(|&(index, count)| slots.get(index).and_then(Option::as_ref).is_none_or(|stack| stack.count < count)) { return false; }
    for &(index, count) in consumed {
        let stack = slots[index].as_mut().unwrap();
        stack.count -= count;
        if stack.count == 0 { slots[index] = None; }
    }
    *carried = Some(output);
    true
}
