// Station recipe selection using the current world's source data.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use super::tconstruct_source_station_recipes::{BuildingMatch, BuildingRecipe};

/// Apply a name only to a newly assembled result. An empty field keeps the
/// normal material-derived name.
pub(super) fn name_result(stack: &mut ItemStack, name: Option<&str>) {
    if let Some(name) = name.filter(|name| !name.is_empty()) {
        stack.data_mut().custom_name = Some(name.to_owned());
    }
}

/// Whether slot zero contains a tool displayed in the station's repair layout.
pub(super) fn has_operation_tool(slots: &[Option<ItemStack>]) -> bool {
    slots.first().and_then(Option::as_ref).is_some_and(|stack| {
        stack.item.as_ref() == "tconstruct:flint_pickaxe"
            || stack.data().and_then(|data| data.other("minecraft:custom_data"))
                .and_then(|tag| tag.get("tic_max_damage")).is_some()
    })
}

/// Resolve supported tools against source recipes and material stats.
pub(super) fn building_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>]) -> Option<(BuildingMatch, ItemStack)> {
    let recipes = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let registered = world.chunks.__horizon_mixins_tconstruct_source_native_durability().as_ref()?;
    for bytes in recipes.values() {
        let Some(recipe) = BuildingRecipe::parse(bytes) else { continue; };
        if !registered.contains_key(&recipe.result) { continue; }
        let Some((namespace, name)) = recipe.result.split_once(':') else { continue; };
        let Some(definition) = materials.get(&format!("{namespace}:tinkering/tool_definitions/{name}")).and_then(|layers| layers.last()) else { continue; };
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
        let output = assemble(world, definition, &matched.result, &matched.materials, None)?;
        return Some((matched, output));
    }
    None
}

fn assemble(world: &super::tconstruct_world::World, definition: &[u8], item: &str, identities: &[String], original: Option<&ItemStack>) -> Option<ItemStack> {
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let mut stat_maps: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();
    for material in identities {
        let (namespace, id) = material.split_once(':')?;
        let id = id.split('#').next()?;
        let layers = materials.get(&format!("{namespace}:tinkering/materials/stats/{id}"))?;
        let merged = super::tconstruct_tx::source_merge_material_stats(layers).ok()?;
        stat_maps.push(merged.into_iter().map(|(key, fields)| (key, serde_json::Value::Object(fields))).collect());
    }
    let mut output = assemble_with_stats(definition, item, identities, &stat_maps, original)?;
    let stats = super::tconstruct_source_station_stats::base_stats(definition, &stat_maps)?;
    let mut ductile = 0;
    if let Some(traits) = super::tconstruct_source_material_traits::levels(world, &output) {
        ductile = traits.levels.get("tconstruct:ductile").copied().unwrap_or(0);
        super::tconstruct_source_material_effects::apply_stat_traits(&mut output, &stats, &traits.levels)?;
    }
    super::tconstruct_source_upgrade_rules::apply_stats(&mut output, &stats, ductile)?;
    Some(output)
}

/// Build supported tools from resolved source stats, shared by the station and Creative.
pub(super) fn assemble_with_stats(definition: &[u8], item: &str, identities: &[String], stat_maps: &[serde_json::Map<String, serde_json::Value>], original: Option<&ItemStack>) -> Option<ItemStack> {
    let stats = super::tconstruct_source_station_stats::base_stats(definition, &stat_maps)?;
    let durability = stats.durability as i32;
    if durability < 1 { return None; }
    let tier = stat_maps.iter().filter_map(|map| map.get("tconstruct:head"))
        .filter_map(|head| head.get("mining_tier").and_then(serde_json::Value::as_str))
        .filter_map(|tier| match tier { "minecraft:wood" => Some((0, "wooden")), "minecraft:gold" => Some((0, "gold")),
            "minecraft:stone" => Some((1, "stone")), "minecraft:iron" => Some((2, "iron")),
            "minecraft:diamond" => Some((3, "diamond")), "minecraft:netherite" => Some((4, "netherite")), _ => None })
        .max_by_key(|(level, _)| *level).map(|(_, tier)| tier);
    let definition_json: serde_json::Value = serde_json::from_slice(definition).ok()?;
    let mut rules = Vec::new();
    if let Some(tier) = tier {
        rules.push(Tag::compound([("blocks", Tag::String(format!("#minecraft:incorrect_for_{tier}_tool").as_str().into())), ("correct_for_drops", Tag::Byte(0))]));
    }
    for action in definition_json.get("modules")?.as_array()?.iter()
        .filter(|module| module.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:tool_actions"))
        .flat_map(|module| module.get("tool_actions").and_then(serde_json::Value::as_array).into_iter().flatten()) {
        let tag = match action.as_str()? { "pickaxe_dig" => "pickaxe", "axe_dig" => "axe", "shovel_dig" => "shovel", "hoe_dig" => "hoe", _ => continue };
        if tier.is_none() { return None; }
        rules.push(Tag::compound([("blocks", Tag::String(format!("#minecraft:mineable/{tag}").as_str().into())), ("speed", Tag::Float(stats.mining_speed.to_bits())), ("correct_for_drops", Tag::Byte(1))]));
    }
    if rules.len() == 1 { rules.clear(); }
    let mut output = original.cloned().unwrap_or_else(|| ItemStack::new(item, 1));
    let mut custom = output.data().and_then(|data| data.other("minecraft:custom_data")).cloned()
        .unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
    let Tag::Compound(ref mut fields) = custom else { return None; };
    fields.insert("tic_materials".into(), Tag::List(identities.iter().map(|m| Tag::String(m.as_str().into())).collect()));
    fields.insert("tic_max_damage".into(), Tag::Int(durability));
    fields.insert("tic_attack_damage".into(), Tag::Float(stats.attack_damage.to_bits()));
    fields.insert("tic_attack_speed".into(), Tag::Float(stats.attack_speed.to_bits()));
    fields.insert("tic_draw_speed".into(), Tag::Float(stats.draw_speed.to_bits()));
    fields.insert("tic_velocity".into(), Tag::Float(stats.velocity.to_bits()));
    fields.insert("tic_accuracy".into(), Tag::Float(stats.accuracy.to_bits()));
    fields.insert("tic_armor".into(), Tag::Float(stats.armor.to_bits()));
    fields.insert("tic_armor_toughness".into(), Tag::Float(stats.armor_toughness.to_bits()));
    fields.insert("tic_knockback_resistance".into(), Tag::Float(stats.knockback_resistance.to_bits()));
    output.data_mut().set_other("minecraft:custom_data", Some(custom));
    output.data_mut().set_other("minecraft:max_damage", Some(Tag::Int(durability)));
    if !rules.is_empty() {
        output.data_mut().set_other("minecraft:tool", Some(Tag::compound([
            ("rules", Tag::List(rules)),
            ("default_mining_speed", Tag::Float(1.0f32.to_bits())),
            ("damage_per_block", Tag::Int(1)),
        ])));
    }
    Some(output)
}

/// Replace one part, rebuilding base stats and preserving the original stack.
pub(super) fn swap_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>]) -> Option<(ItemStack, Vec<(usize, u32)>)> {
    let tool = slots.first()?.as_ref()?;
    if !tool.item.starts_with("tconstruct:") || tool.count != 1 { return None; }
    let mut inputs = (1..=3).filter_map(|index| slots.get(index).and_then(Option::as_ref).map(|stack| (index, stack)));
    let (slot, part) = inputs.next()?;
    if inputs.next().is_some() { return None; }
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let (namespace, name) = tool.item.split_once(':')?;
    let definition = materials.get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?.last()?;
    let recipe = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?.values()
        .filter_map(|bytes| BuildingRecipe::parse(bytes)).find(|recipe| recipe.result == &*tool.item)?;
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
    let repair = if super::tconstruct_source_station_stats::part_kind(&parts[index])? == "head" && cost > 0 {
        let (namespace, name) = material.split_once(':')?;
        let layers = materials.get(&format!("{namespace}:tinkering/materials/stats/{}", name.split('#').next()?))?;
        let stats = super::tconstruct_tx::source_merge_material_stats(layers).ok()?;
        (stats.get("tconstruct:head")?.get("durability")?.as_f64()? as f32 * cost as f32 / 3.0) as u32
    } else { 0 };
    if !changed && (tool.damage == 0 || repair == 0) { return None; }
    let mut output = assemble(world, definition, &tool.item, &identities, Some(tool))?;
    super::tconstruct_tx::fixed_set_damage(&mut output, tool.damage.saturating_sub(repair));
    Some((output, vec![(0, 1), (slot, 1)]))
}

/// Compute material repair from the tool definition and world recipes.
pub(super) fn repair_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>]) -> Option<(ItemStack, Vec<(usize, u32)>)> {
    let mut tool = slots.first()?.as_ref()?.clone();
    if !tool.item.starts_with("tconstruct:") || tool.count != 1 || tool.damage == 0 { return None; }
    let custom = tool.data()?.other("minecraft:custom_data")?;
    let Tag::List(ids) = custom.get("tic_materials")? else { return None; };
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let (namespace, name) = tool.item.split_once(':')?;
    let definition = materials.get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?.last()?;
    let recipe = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?.values()
        .filter_map(|bytes| BuildingRecipe::parse(bytes)).find(|recipe| recipe.result == &*tool.item)?;
    let parts = recipe.parts(definition)?;
    let repair_parts: Vec<_> = parts.iter().zip(ids).filter_map(|(part, id)| {
        let kind = super::tconstruct_source_station_stats::part_kind(part)?;
        if !matches!(kind, "head" | "limb") { return None; }
        let Tag::String(id) = id else { return None; };
        Some((id.to_string_lossy().to_string(), kind))
    }).collect();
    let traits = super::tconstruct_source_material_traits::levels(world, &tool);
    let cultivated = traits.as_ref().and_then(|traits| traits.levels.get("tconstruct:cultivated").copied()).unwrap_or(0);
    let preserved = traits.as_ref().and_then(|traits| traits.levels.get("tconstruct:preserved").copied()).unwrap_or(0);
    let repair_factor = super::tconstruct_source_material_effects::repair_factor(cultivated, preserved);
    let mut remaining = i32::try_from(tool.damage).ok()?;
    let mut consumed = vec![(0, 1)];
    for index in 1..=3 {
        let Some(stack) = slots.get(index).and_then(Option::as_ref) else { continue; };
        let material = super::tconstruct_source_builder_world::lookup_material(world, stack).ok()??;
        if material.needed <= 0 { return None; }
        let (namespace, path) = material.material.split_once(':')?;
        let stat_layers = materials.get(&format!("{namespace}:tinkering/materials/stats/{}", path.split('#').next()?))?;
        let stats = super::tconstruct_tx::source_merge_material_stats(stat_layers).ok()?;
        // Original material repair uses the strongest matching repairable stat.
        let durability = repair_parts.iter().filter(|(id, _)| id == &material.material)
            .filter_map(|(_, kind)| stats.get(&format!("tconstruct:{kind}"))?.get("durability")?.as_f64())
            .max_by(f64::total_cmp)? as f32;
        let amount = durability * material.value as f32 * repair_factor / (3.0 * material.needed as f32);
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

pub(super) struct StationOperation {
    pub output: ItemStack,
    pub consumed: Vec<(usize, u32)>,
    pub leftover: Option<ItemStack>,
}

fn upgrade_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>]) -> Option<StationOperation> {
    let tool = slots.first()?.as_ref()?;
    if tool.count != 1 { return None; }
    let Tag::List(materials) = tool.data()?.other("minecraft:custom_data")?.get("tic_materials")? else { return None; };
    let identities = materials.iter().map(|value| match value {
        Tag::String(id) => Some(id.to_string_lossy().to_string()), _ => None,
    }).collect::<Option<Vec<_>>>()?;
    let definitions = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let (namespace, name) = tool.item.split_once(':')?;
    let definition = definitions.get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?.last()?;
    let parsed: serde_json::Value = serde_json::from_slice(definition).ok()?;
    let slots_available = parsed.get("modules")?.as_array()?.iter()
        .filter(|module| module.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:modifier_slots"))
        .filter_map(|module| module.pointer("/slots/upgrades").and_then(serde_json::Value::as_i64))
        .sum::<i64>();
    let recipes = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let tags = world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let plan = super::tconstruct_source_upgrade_rules::plan(recipes, tags, tool, slots.get(1..4)?, i32::try_from(slots_available).ok()?)?;
    let mut upgraded = tool.clone();
    let mut custom = upgraded.data()?.other("minecraft:custom_data")?.clone();
    let mut upgrades = custom.get("tic_upgrades").cloned().unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
    let Tag::Compound(ref mut fields) = custom else { return None; };
    let Tag::Compound(ref mut entries) = upgrades else { return None; };
    entries.insert(plan.id.as_str().into(), Tag::Int(plan.amount));
    fields.insert("tic_upgrades".into(), upgrades);
    upgraded.data_mut().set_other("minecraft:custom_data", Some(custom));
    let output = assemble(world, definition, &tool.item, &identities, Some(&upgraded))?;
    Some(StationOperation { output, consumed: plan.consumed, leftover: plan.leftover })
}

/// Revalidate and commit a planned operation; return any item to give to the player.
pub(super) fn take_operation(slots: &mut [Option<ItemStack>], carried: &mut Option<ItemStack>, plan: StationOperation) -> Option<Vec<ItemStack>> {
    if !take_repair(slots, carried, plan.output, &plan.consumed) { return None; }
    Some(plan.leftover.into_iter().collect())
}

/// Plan an upgrade, replacement or repair without consuming inputs.
pub(super) fn operation_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>]) -> Option<StationOperation> {
    if let Some(plan) = upgrade_plan(world, slots) { return Some(plan); }
    if let Some((output, consumed)) = swap_plan(world, slots).or_else(|| repair_plan(world, slots)) {
        return Some(StationOperation { output, consumed, leftover: None });
    }
    let tool = slots.first()?.as_ref()?;
    if tool.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|data| data.get("tic_max_damage")).is_some() { return None; }
    let mut preview = slots.to_vec();
    let mut output = None;
    if !super::tconstruct_tx::take(&mut preview, &mut output, true, 25) { return None; }
    let consumed = slots.iter().zip(&preview).enumerate().filter_map(|(index, (before, after))| {
        let before = before.as_ref()?.count;
        let after = after.as_ref().map_or(0, |stack| stack.count);
        (before > after).then_some((index, before - after))
    }).collect();
    Some(StationOperation { output: output?, consumed, leftover: None })
}
