// Station recipe selection using the current world's source data.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use super::tconstruct_source_station_recipes::{BuildingMatch, BuildingRecipe};

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
    building_plan_with_capacity(world, slots, 3)
}

/// The authoritative caller must select the capacity from the workstation block.
pub(super) fn building_plan_with_capacity(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>], input_capacity: usize) -> Option<(BuildingMatch, ItemStack)> {
    building_plan_diagnostic(world, slots, input_capacity, &mut None)
}

pub(super) fn building_plan_diagnostic(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>], input_capacity: usize, reason: &mut Option<String>) -> Option<(BuildingMatch, ItemStack)> {
    if !matches!(input_capacity, 3 | 5) { return None; }
    let recipes = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let registered = world.chunks.__horizon_mixins_tconstruct_source_native_durability().as_ref()?;
    for bytes in recipes.values() {
        let Some(recipe) = BuildingRecipe::parse(bytes) else { continue; };
        if !registered.contains_key(&recipe.result) { continue; }
        let Some((namespace, name)) = recipe.result.split_once(':') else { continue; };
        let Some(definition) = materials.get(&format!("{namespace}:tinkering/tool_definitions/{name}")).and_then(|layers| layers.last()) else { continue; };
        let Some(parts) = recipe.parts(definition) else { continue; };
        let Some(inputs) = slots.get(..input_capacity) else { return None; };
        let Some(matched) = recipe.match_inputs(&parts, parts.len(), inputs, |stack| {
            match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|tag| tag.get("Material")) {
                Some(Tag::String(material)) => Some(material.to_string_lossy().to_string()),
                _ => None,
            }
        }, |part, material| {
            super::tconstruct_source_builder_world::supported_material(world, material)
                .is_ok_and(|(_, supported)| supported.contains(part))
        }, |_, _| false) else { continue; };
        let mut output = assemble(world, definition, &matched.result, &matched.materials, None, reason)?;
        output.count = matched.result_count;
        return Some((matched, output));
    }
    None
}

fn assemble(world: &super::tconstruct_world::World, definition: &[u8], item: &str, identities: &[String], original: Option<&ItemStack>, reason: &mut Option<String>) -> Option<ItemStack> {
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let mut stat_maps: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();
    for material in identities {
        let (namespace, id) = material.split_once(':')?;
        let id = id.split('#').next()?;
        let layers = materials.get(&format!("{namespace}:tinkering/materials/stats/{id}"))?;
        let merged = super::tconstruct_tx::source_merge_material_stats(layers).ok()?;
        stat_maps.push(merged.into_iter().map(|(key, fields)| (key, serde_json::Value::Object(fields))).collect());
    }
    // Existing inputs are checked before mutation by the operation planner.
    let mut output = assemble_with_stats(definition, item, identities, &stat_maps, original)?;
    let stats = super::tconstruct_source_station_stats::base_stats(definition, &stat_maps)?;
    let mut levels = std::collections::BTreeMap::new();
    let mut order = Vec::new();
    if let Some(traits) = super::tconstruct_source_material_traits::levels(world, &output) {
        levels = traits.levels;
        order = traits.order;
        super::tconstruct_source_material_effects::apply_stat_traits(&mut output, &stats, &levels)?;
    }
    if !super::tconstruct_source_upgrade_rules::valid_unbreakable(&output, &levels) { return None; }
    let tags = world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let has_tag = |tag: &str| tags.get(tag).is_some_and(|items| items.contains(item));
    super::tconstruct_source_upgrade_rules::apply_gem_durability_and_armor(&mut output, &stats, &levels, has_tag("tconstruct:modifiable/armor"), has_tag("tconstruct:modifiable/armor/worn"))?;
    super::tconstruct_source_upgrade_rules::apply_stats(&mut output, &stats, &levels,
        has_tag("tconstruct:modifiable/melee"), has_tag("tconstruct:modifiable/melee/weapon"))?;
    super::tconstruct_source_mining_stack::apply(&mut output, &stats, &levels, &order, has_tag("tconstruct:modifiable/harvest"))?;
    super::tconstruct_source_overslime::rebuild(&mut output, &stats, &levels, has_tag("tconstruct:modifiable/melee"), has_tag("tconstruct:modifiable/armor"), has_tag("tconstruct:modifiable/ranged"), has_tag("tconstruct:modifiable/harvest"))?;
    let block_ids = world.chunks.__horizon_mixins_tconstruct_resolved_block_ids().as_ref().and_then(|ids| ids.as_ref().ok());
    if let Err(error) = super::tconstruct_source_tool_normalize::finalize_tool(&mut output, |id| block_ids.and_then(|ids| ids.get(id).copied())) { *reason = Some(format!("Cannot modify tool: {error}")); return None; }
    Some(output)
}

/// Build supported tools from resolved source stats, shared by the station and Creative.
pub fn assemble_with_stats(definition: &[u8], item: &str, identities: &[String], stat_maps: &[serde_json::Map<String, serde_json::Value>], original: Option<&ItemStack>) -> Option<ItemStack> {
    let stats = super::tconstruct_source_station_stats::base_stats(definition, &stat_maps)?;
    let durability = stats.durability as i32;
    if durability < 1 { return None; }
    let original_upgrades = original.and_then(|s| s.data()).and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_upgrades"));
    let has_upgrade = |id| matches!(original_upgrades.and_then(|u| u.get(id)), Some(Tag::Int(v)) if *v > 0);
    let tier = super::tconstruct_source_station_stats::gem_tier(super::tconstruct_source_station_stats::mining_tier(definition, stat_maps)?, has_upgrade("tconstruct:diamond"), has_upgrade("tconstruct:emerald"));
    let tier = if has_upgrade("tconstruct:netherite") && tier.is_some() { Some("netherite") } else { tier };
    let definition_json: serde_json::Value = serde_json::from_slice(definition).ok()?;
    let mut rules = Vec::new();
    let source_effective = definition_json.get("modules")?.as_array()?.iter().find(|module|
        module.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:is_effective"));
    let ignore_tier = source_effective.and_then(|module| module.get("ignore_tier")).and_then(serde_json::Value::as_bool).unwrap_or(false);
    if !ignore_tier {
        if let Some(tier) = tier {
            rules.push(Tag::compound([("blocks", Tag::String(format!("#minecraft:incorrect_for_{tier}_tool").as_str().into())), ("correct_for_drops", Tag::Byte(0))]));
        }
    }
    if let Some(module) = source_effective {
        if module.get("predicate_type").and_then(serde_json::Value::as_str) != Some("mantle:tag") { return None; }
        if tier.is_none() && !ignore_tier { return None; }
        let tag = module.get("tag").and_then(serde_json::Value::as_str)?;
        let alias = super::tconstruct_source_harvest_tags::alias(tag)?;
        rules.push(Tag::compound([("blocks", Tag::String(format!("#{alias}").as_str().into())), ("speed", Tag::Float(stats.mining_speed.to_bits())), ("correct_for_drops", Tag::Byte(1))]));
    } else {
        // Keep the previously supported action-based fallback for definitions
        // without an original effective predicate; it is not source parity.
        for action in definition_json.get("modules")?.as_array()?.iter()
            .filter(|module| module.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:tool_actions"))
            .flat_map(|module| module.get("tool_actions").and_then(serde_json::Value::as_array).into_iter().flatten()) {
            let tag = match action.as_str()? { "pickaxe_dig" => "pickaxe", "axe_dig" => "axe", "shovel_dig" => "shovel", "hoe_dig" => "hoe", _ => continue };
            if tier.is_none() { return None; }
            rules.push(Tag::compound([("blocks", Tag::String(format!("#minecraft:mineable/{tag}").as_str().into())), ("speed", Tag::Float(stats.mining_speed.to_bits())), ("correct_for_drops", Tag::Byte(1))]));
        }
    }
    if !rules.iter().any(|rule| rule.get("speed").is_some()) { rules.clear(); }
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
    // Narrow source-equivalent tank support: the pinned pickaxe has a fixed
    // pierce definition trait. Only material traits known not to affect tank
    // capacity are accepted, and unknown upgrades do not create a tank.
    let tank_level = match fields.get(&"tic_upgrades".into()).and_then(|v| v.get("tconstruct:tank")) {
        None => 0,
        Some(Tag::Int(level)) if *level == 1 => 1,
        _ => return None,
    };
    if tank_level > 0 {
        if item != "tconstruct:pickaxe" { return None; }
        let mut probe = output.clone();
        probe.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(fields.clone())));
        let stored = super::pluto_stack_fluid::read(&probe, "tic_persistent", "tconstruct:tank_fluid")?;
        if stored.as_ref().is_some_and(|fluid| fluid.amount > 1000) { return None; }
        let traits = super::tconstruct_source_material_traits::levels(world, &probe)?;
        if traits.levels.keys().any(|id| !matches!(id.as_str(), "tconstruct:cultivated" | "tconstruct:jagged")) { return None; }
        let upgrades = fields.get(&"tic_upgrades".into())?;
        let Tag::Compound(upgrades) = upgrades else { return None; };
        if upgrades.keys().any(|id| !matches!(id.to_string_lossy().as_str(), "tconstruct:tank" | "tconstruct:diamond" | "tconstruct:emerald" | "tconstruct:haste" | "tconstruct:sharpness" | "tconstruct:reinforced" | "tconstruct:netherite" | "tconstruct:unbreakable" | "tconstruct:glowing")) { return None; }
        fields.insert("tic_tank_capacity".into(), Tag::Int(1000));
    } else {
        // Never discard fluid when a tank-bearing tool is rebuilt without it.
        if fields.get(&"tic_tank_capacity".into()).is_some()
            && fields.get(&"tic_persistent".into()).and_then(|v| v.get("tconstruct:tank_fluid")).is_some() { return None; }
        fields.remove(&"tic_tank_capacity".into());
    }
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
    swap_plan_with_capacity(world, slots, 3, &mut None)
}

fn swap_plan_with_capacity(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>], input_capacity: usize, reason: &mut Option<String>) -> Option<(ItemStack, Vec<(usize, u32)>)> {
    if !matches!(input_capacity, 3 | 5) { return None; }
    let tool = slots.first()?.as_ref()?;
    if !tool.item.starts_with("tconstruct:") || tool.count != 1 { return None; }
    let mut inputs = (1..=input_capacity).filter_map(|index| slots.get(index).and_then(Option::as_ref).map(|stack| (index, stack)));
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
    let output = rebuild_material_swap(world, definition, tool, &identities, repair, false, reason)?;
    Some((output, vec![(0, 1), (slot, 1)]))
}

/// Rebuild using the station's existing normalization, stats and slot checks.
/// Callers lacking original modifier hooks request unchanged traits so no
/// trait removal or changed-level validation is silently skipped.
pub(super) fn rebuild_material_swap(world: &super::tconstruct_world::World, definition: &[u8], tool: &ItemStack,
    identities: &[String], repair: u32, reject_removed_traits: bool, reason: &mut Option<String>) -> Option<ItemStack> {
    let old_traits = if reject_removed_traits { Some(super::tconstruct_source_material_traits::levels(world, tool)?.levels) } else { None };
    let verified = match super::tconstruct_source_tool_normalize::prepare_for_tool_mutation(tool) {
        Ok(value) => value, Err(error) => { *reason = Some(format!("Cannot modify tool: {error}")); return None; }
    };
    let mut output = assemble(world, definition, &tool.item, identities, Some(&verified), reason)?;
    let parsed: serde_json::Value = serde_json::from_slice(definition).ok()?;
    let modules = parsed.get("modules")?.as_array()?;
    let base_slots = |name: &str| -> Option<i32> {
        let total = modules.iter().filter(|module| module.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:modifier_slots"))
            .filter_map(|module| module.get("slots")?.get(name)?.as_i64()).sum::<i64>();
        i32::try_from(total).ok()
    };
    let traits = super::tconstruct_source_material_traits::levels(world, &output)?.levels;
    // These upstream material traits have no removal hook; their supported
    // stat and repair effects are rebuilt by the station path above. Other
    // changed traits remain closed until their removal hooks are handled.
    if old_traits.as_ref().is_some_and(|old| old.iter().chain(traits.iter()).any(|(id, _)| {
        old.get(id).copied().unwrap_or(0) != traits.get(id).copied().unwrap_or(0)
            && !matches!(id.as_str(), "tconstruct:preserved" | "tconstruct:ductile" | "tconstruct:cultivated" | "tconstruct:dense")
    })) { return None; }
    if !super::tconstruct_source_upgrade_rules::valid_slots(&output, &traits, base_slots("upgrades")?, base_slots("abilities")?)
        || !super::tconstruct_source_upgrade_rules::valid_unbreakable(&output, &traits) { return None; }
    super::tconstruct_tx::fixed_set_damage(&mut output, tool.damage.saturating_sub(repair));
    Some(output)
}

/// Compute material repair from the tool definition and world recipes.
pub(super) fn repair_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>]) -> Option<(ItemStack, Vec<(usize, u32)>)> {
    repair_plan_with_capacity(world, slots, 3, &mut None)
}

fn repair_plan_with_capacity(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>], input_capacity: usize, reason: &mut Option<String>) -> Option<(ItemStack, Vec<(usize, u32)>)> {
    if !matches!(input_capacity, 3 | 5) { return None; }
    let original = slots.first()?.as_ref()?;
    if !original.item.starts_with("tconstruct:") || original.count != 1 || original.damage == 0 { return None; }
    let mut tool = match super::tconstruct_source_tool_normalize::prepare_for_tool_mutation(original) {
        Ok(value) => value, Err(error) => { *reason = Some(format!("Cannot modify tool: {error}")); return None; }
    };
    let empty_ids = Vec::new();
    let ids = match tool.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|custom| custom.get("tic_materials")) {
        Some(Tag::List(ids)) => ids,
        None => &empty_ids,
        _ => return None,
    };
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let (namespace, name) = tool.item.split_once(':')?;
    let definition = materials.get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?.last()?;
    let recipe = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?.values()
        .filter_map(|bytes| BuildingRecipe::parse(bytes)).find(|recipe| recipe.result == &*tool.item);
    let mut repair_parts: Vec<(String, String)> = Vec::new();
    if let Some(parts) = recipe.as_ref().and_then(|recipe| recipe.parts(definition)) {
        for (part, id) in parts.iter().zip(ids) {
            let Some(kind) = super::tconstruct_source_station_stats::part_kind(part) else { continue; };
            if !matches!(kind, "head" | "limb" | "slime" | "skull") && !kind.starts_with("plating_") { continue; }
            if let Tag::String(id) = id { repair_parts.push((id.to_string_lossy().to_string(), kind.to_owned())); }
        }
    } else {
        let definition: serde_json::Value = serde_json::from_slice(definition).ok()?;
        for module in definition.get("modules")?.as_array()? {
            if module.get("type").and_then(serde_json::Value::as_str) != Some("tconstruct:material_stats") { continue; }
            for (entry, id) in module.get("stat_types")?.as_array()?.iter().zip(ids) {
                let name = entry.as_str().or_else(|| entry.get("stat").and_then(serde_json::Value::as_str))?;
                let Some(kind) = name.strip_prefix("tconstruct:") else { continue; };
                if !matches!(kind, "head" | "limb" | "slime" | "skull") && !kind.starts_with("plating_") { continue; }
                if let Tag::String(id) = id { repair_parts.push((id.to_string_lossy().to_string(), kind.to_owned())); }
            }
        }
    }
    // Independent source material-repair hooks use fixed material/value pairs.
    let parsed: serde_json::Value = serde_json::from_slice(definition).ok()?;
    let mut fixed_repairs = Vec::<(String, f32)>::new();
    for module in parsed.get("modules")?.as_array()? {
        match module.get("type").and_then(serde_json::Value::as_str) {
            Some("tconstruct:material_repair") => {
                if let Some(id) = module.get("material").and_then(serde_json::Value::as_str) {
                    let value = module.get("durability").and_then(serde_json::Value::as_f64).or_else(|| {
                        let stat = module.get("stat_type")?.as_str()?;
                        let (namespace, path) = id.split_once(':')?;
                        let layers = materials.get(&format!("{namespace}:tinkering/materials/stats/{}", path.split('#').next()?))?;
                        let stats = super::tconstruct_tx::source_merge_material_stats(layers).ok()?;
                        stats.get(stat)?.get("durability")?.as_f64()
                    });
                    if let Some(value) = value { fixed_repairs.push((id.to_owned(), value as f32)); }
                }
            }
            Some("tconstruct:statless_part_repair") => {
                if let (Some(index), Some(value)) = (module.get("part_index").and_then(serde_json::Value::as_u64), module.get("repair_amount").and_then(serde_json::Value::as_f64)) {
                    if let Some(Tag::String(id)) = ids.get(index as usize) { fixed_repairs.push((id.to_string_lossy().to_string(), value as f32)); }
                }
            }
            _ => {}
        }
    }
    let traits = super::tconstruct_source_material_traits::levels(world, &tool);
    let repair_factor = station_repair_factor(&tool, traits.as_ref().map(|traits| &traits.levels));
    let mut remaining = i32::try_from(tool.damage).ok()?;
    let mut consumed = vec![(0, 1)];
    for index in 1..=input_capacity {
        let Some(stack) = slots.get(index).and_then(Option::as_ref) else { continue; };
        let material = super::tconstruct_source_builder_world::lookup_material(world, stack).ok()??;
        if material.needed <= 0 { return None; }
        let stat_durability = material.material.split_once(':').and_then(|(namespace, path)| {
            let layers = materials.get(&format!("{namespace}:tinkering/materials/stats/{}", path.split('#').next()?))?;
            let stats = super::tconstruct_tx::source_merge_material_stats(layers).ok()?;
            repair_parts.iter().filter(|(id, _)| id.split('#').next() == material.material.split('#').next())
                .filter_map(|(_, kind)| stats.get(&format!("tconstruct:{kind}"))?.get("durability")?.as_f64())
                .max_by(f64::total_cmp).map(|v| v as f32)
        }).unwrap_or(0.0);
        let fixed_durability = fixed_repairs.iter().filter(|(id, _)| id.split('#').next() == material.material.split('#').next()).map(|(_, value)| *value).fold(0.0_f32, f32::max);
        let stringy = traits.as_ref().and_then(|traits| traits.levels.get("tconstruct:stringy").copied()).unwrap_or(0);
        let durability = stat_durability.max(fixed_durability).max(super::tconstruct_source_material_effects::stringy_repair(&material.material, stringy));
        if durability <= 0.0 { continue; }
        let amount = durability * material.value as f32 * repair_factor / (3.0 * material.needed as f32);
        let (count, repaired) = super::tconstruct_tx::repair_from_slot(remaining, i32::try_from(stack.count).ok()?, amount);
        if count > 0 { consumed.push((index, count as u32)); remaining = (remaining - repaired).max(0); }
    }
    if consumed.len() == 1 { return None; }
    super::tconstruct_tx::fixed_set_damage(&mut tool, remaining as u32);
    let block_ids = world.chunks.__horizon_mixins_tconstruct_resolved_block_ids().as_ref().and_then(|ids| ids.as_ref().ok());
    if let Err(error) = super::tconstruct_source_tool_normalize::finalize_tool(&mut tool, |id| block_ids.and_then(|ids| ids.get(id).copied())) { *reason = Some(format!("Cannot modify tool: {error}")); return None; }
    Some((tool, consumed))
}

fn station_repair_factor(tool: &ItemStack, traits: Option<&std::collections::BTreeMap<String, i32>>) -> f32 {
    let level = |id: &str| traits.and_then(|levels| levels.get(id).copied()).unwrap_or(0);
    super::tconstruct_source_material_effects::repair_factor(level("tconstruct:cultivated"), level("tconstruct:preserved"))
        * super::tconstruct_source_material_effects::dense_repair_factor(level("tconstruct:dense"))
        * if matches!(tool.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_upgrades")).and_then(|d| d.get("tconstruct:emerald")), Some(Tag::Int(v)) if *v > 0) { 1.5 } else { 1.0 }
}

/// Source modifier_repair recipes require only matching inputs and consume
/// enough items to cover the actual repaired damage, in input-slot order.
fn modifier_repair_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>], input_capacity: usize, reason: &mut Option<String>) -> Option<StationOperation> {
    let original = slots.first()?.as_ref()?;
    if original.count != 1 || original.damage == 0 { return None; }
    let tags = world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    if !tags.get("tconstruct:modifiable/durability")?.contains(original.item.as_ref()) { return None; }
    let traits = super::tconstruct_source_material_traits::levels(world, original)?;
    if traits.levels.get("tconstruct:emerald").copied().unwrap_or(0) > 0 { return None; }
    let custom = original.data()?.other("minecraft:custom_data")?;
    let upgrades = custom.get("tic_upgrades");
    if upgrades.is_some_and(|value| !matches!(value, Tag::Compound(_))) { return None; }
    let mut repair_traits = traits.levels.clone();
    for id in ["tconstruct:cultivated", "tconstruct:preserved", "tconstruct:dense"] {
        let level = match upgrades.and_then(|value| value.get(id)) { Some(Tag::Int(value)) if *value >= 0 => *value, None => 0, _ => return None };
        let entry = repair_traits.entry(id.to_owned()).or_default();
        *entry = entry.checked_add(level)?;
    }
    let definitions = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let (namespace, name) = original.item.split_once(':')?;
    let definition: serde_json::Value = serde_json::from_slice(definitions.get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?.last()?).ok()?;
    let modules = definition.get("modules")?.as_array()?;
    let inputs = slots.get(1..=input_capacity)?;
    let recipes = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let mut found: Option<StationOperation> = None;
    for bytes in recipes.values() {
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else { continue; };
        if value.get("type").and_then(serde_json::Value::as_str) != Some("tconstruct:modifier_repair") { continue; }
        let mut conditions_ok = true;
        for key in ["conditions", "forge:conditions"] {
            let Some(raw) = value.get(key) else { continue; };
            let Some(conditions) = raw.as_array() else { conditions_ok = false; break; };
            if conditions.is_empty() { continue; }
            let Some((_, fluid_tags)) = world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref().and_then(|v| v.as_ref().ok()) else { conditions_ok = false; break; };
            let Some(config) = world.chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref().and_then(|v| v.as_ref().ok()) else { conditions_ok = false; break; };
            let Some(items) = world.chunks.__horizon_mixins_tconstruct_melter_known_items().as_ref() else { conditions_ok = false; break; };
            let registries = [("minecraft:item".to_owned(), tags.clone()), ("minecraft:fluid".to_owned(), fluid_tags.clone())].into_iter().collect();
            if super::tconstruct_source_conditions::conditional_conditions(conditions, &registries, config, items) != Some(true) { conditions_ok = false; break; }
        }
        if !conditions_ok { continue; }
        let Some(modifier) = value.get("modifier").and_then(serde_json::Value::as_str) else { continue; };
        // Default trait modules contribute to TOOL_TRAITS. Explicit hook
        // overrides contribute only when they include that hook.
        let mut definition_modifier = 0i32;
        let mut definition_repair_traits = repair_traits.clone();
        let mut known = true;
        for module in modules.iter().filter(|module| module.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:traits")) {
            if let Some(hooks) = module.get("hooks") {
                let Some(hooks) = hooks.as_array() else { known = false; break; };
                if hooks.iter().any(|hook| hook.as_str().is_none()) { known = false; break; }
                if !hooks.iter().any(|hook| hook.as_str() == Some("tconstruct:tool_traits")) { continue; }
            }
            let Some(entries) = module.get("traits").and_then(serde_json::Value::as_array) else { known = false; break; };
            for entry in entries {
                let Some(id) = entry.get("name").and_then(serde_json::Value::as_str) else { known = false; break; };
                if id != modifier && !matches!(id, "tconstruct:cultivated" | "tconstruct:preserved" | "tconstruct:dense" | "tconstruct:emerald") { continue; }
                if id == "tconstruct:emerald" || !matches!(entry.get("optional"), None | Some(serde_json::Value::Bool(false))) { known = false; break; }
                let value = match entry.get("level") { None => Some(1), Some(value) => value.as_i64().and_then(|value| i32::try_from(value).ok()).filter(|value| *value > 0) };
                let Some(value) = value else { known = false; break; };
                if id == modifier { definition_modifier = definition_modifier.checked_add(value)?; }
                if matches!(id, "tconstruct:cultivated" | "tconstruct:preserved" | "tconstruct:dense") {
                    let level = definition_repair_traits.entry(id.to_owned()).or_default();
                    *level = level.checked_add(value)?;
                }
            }
            if !known { break; }
        }
        if !known { continue; }
        let Some(per_level) = value.get("repair_amount").and_then(serde_json::Value::as_i64).and_then(|v| i32::try_from(v).ok()).filter(|v| *v > 0) else { continue; };
        let Some(ingredient) = value.get("ingredient") else { continue; };
        let mut count = 0u32;
        let mut eligible = true;
        for stack in inputs.iter().flatten() {
            if super::tconstruct_source_station_recipes::repair_ingredient_matches(ingredient, &stack.item, tags) != Some(true) { eligible = false; break; }
            count = count.checked_add(stack.count)?;
        }
        if !eligible || count == 0 { continue; }
        let upgrade = match upgrades.and_then(|upgrades| upgrades.get(modifier)) { Some(Tag::Int(level)) if *level >= 0 => *level, None => 0, _ => return None };
        let level = traits.levels.get(modifier).copied().unwrap_or(0).checked_add(upgrade)?.checked_add(definition_modifier)?;
        let raw = level.checked_mul(per_level)?;
        let factor = station_repair_factor(original, Some(&definition_repair_traits));
        if raw <= 0 || !factor.is_finite() || factor <= 0.0 { continue; }
        let per_item = ((raw as f32 * factor) as i32).max(0) as u32;
        if per_item == 0 { continue; }
        let repaired = original.damage.min(count.saturating_mul(per_item));
        let mut needed = repaired.div_ceil(per_item);
        let mut consumed = vec![(0, 1)];
        for (index, stack) in inputs.iter().enumerate() {
            if let Some(stack) = stack {
                let take = needed.min(stack.count);
                if take > 0 { consumed.push((index + 1, take)); needed -= take; }
            }
        }
        if needed != 0 { return None; }
        let mut output = match super::tconstruct_source_tool_normalize::prepare_for_tool_mutation(original) {
            Ok(value) => value, Err(error) => { *reason = Some(format!("Cannot modify tool: {error}")); return None; }
        };
        super::tconstruct_tx::fixed_set_damage(&mut output, original.damage - repaired);
        let block_ids = world.chunks.__horizon_mixins_tconstruct_resolved_block_ids().as_ref().and_then(|ids| ids.as_ref().ok());
        if super::tconstruct_source_tool_normalize::finalize_tool(&mut output, |id| block_ids.and_then(|ids| ids.get(id).copied())).is_err() { return None; }
        if found.is_some() { return None; }
        found = Some(StationOperation { output, consumed, leftovers: Vec::new() });
    }
    found
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
    pub leftovers: Vec<ItemStack>,
}

fn upgrade_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>], input_capacity: usize, reason: &mut Option<String>) -> Option<StationOperation> {
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
    let abilities_available = parsed.get("modules")?.as_array()?.iter()
        .filter(|module| module.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:modifier_slots"))
        .filter_map(|module| module.pointer("/slots/abilities").and_then(serde_json::Value::as_i64)).sum::<i64>();
    let traits = super::tconstruct_source_material_traits::levels(world, tool).map(|v| v.levels).unwrap_or_default();
    let plan = super::tconstruct_source_upgrade_rules::plan_with_traits(recipes, tags, tool, slots.get(1..=input_capacity)?, i32::try_from(slots_available).ok()?, i32::try_from(abilities_available).ok()?, &traits)?;
    if !super::tconstruct_source_upgrade_rules::provisional_upgrade_enabled(&plan.id) {
        *reason = Some(format!("Unavailable upgrade in this development build: {}", plan.id));
        return None;
    }
    let mut upgraded = match super::tconstruct_source_tool_normalize::prepare_for_tool_mutation(tool) {
        Ok(value) => value, Err(error) => { *reason = Some(format!("Cannot modify tool: {error}")); return None; }
    };
    let mut custom = upgraded.data()?.other("minecraft:custom_data")?.clone();
    let mut upgrades = custom.get("tic_upgrades").cloned().unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
    let Tag::Compound(ref mut fields) = custom else { return None; };
    let Tag::Compound(ref mut entries) = upgrades else { return None; };
    entries.insert(plan.id.as_str().into(), Tag::Int(plan.amount));
    fields.insert("tic_upgrades".into(), upgrades);
    upgraded.data_mut().set_other("minecraft:custom_data", Some(custom));
    let output = assemble(world, definition, &tool.item, &identities, Some(&upgraded), reason)?;
    Some(StationOperation { output, consumed: plan.consumed, leftovers: plan.leftover.into_iter().collect() })
}

/// Apply source overslime refills using only the matching input items.
fn overslime_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>], input_capacity: usize, reason: &mut Option<String>) -> Option<StationOperation> {
    let tool = slots.first()?.as_ref()?;
    if tool.count != 1 { return None; }
    let tags = world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    if !tags.get("tconstruct:modifiable/durability").is_some_and(|items| items.contains(tool.item.as_ref())) { return None; }
    let inputs = slots.get(1..=input_capacity)?.iter().enumerate().filter_map(|(i,s)| s.as_ref().map(|s| (i+1,s))).collect::<Vec<_>>();
    if inputs.is_empty() { return None; }
    let item = inputs[0].1.item.to_string();
    if inputs.iter().any(|(_,s)| s.item.as_ref() != item) { return None; }
    let recipes = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let restore = recipes.values().filter_map(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok()).find_map(|value| {
        (value.get("type")?.as_str()? == "tconstruct:overslime_modifier" && value.pointer("/ingredient/item")?.as_str()? == item.as_str())
            .then(|| value.get("restore_amount")?.as_u64().and_then(|n| u32::try_from(n).ok())).flatten()
    })?;
    let Tag::List(materials) = tool.data()?.other("minecraft:custom_data")?.get("tic_materials")? else { return None; };
    let identities = materials.iter().map(|v| if let Tag::String(id)=v {Some(id.to_string_lossy().to_string())} else {None}).collect::<Option<Vec<_>>>()?;
    let definitions = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let (namespace,name) = tool.item.split_once(':')?;
    let definition = definitions.get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?.last()?;
    let traits = super::tconstruct_source_material_traits::levels(world, tool).map(|t|t.levels).unwrap_or_default();
    let mut upgraded = match super::tconstruct_source_tool_normalize::prepare_for_tool_mutation(tool) {
        Ok(value) => value, Err(error) => { *reason = Some(format!("Cannot modify tool: {error}")); return None; }
    };
    let mut custom = upgraded.data()?.other("minecraft:custom_data")?.clone();
    let mut upgrades = custom.get("tic_upgrades").cloned().unwrap_or_else(|| Tag::compound(Vec::<(&str,Tag)>::new()));
    let Tag::Compound(ref mut fields) = custom else { return None; };
    let Tag::Compound(ref mut entries) = upgrades else { return None; };
    if traits.get("tconstruct:overslime").copied().unwrap_or(0) <= 0 && !matches!(entries.get(&"tconstruct:overslime".into()), Some(Tag::Int(level)) if *level > 0) {
        entries.insert("tconstruct:overslime".into(), Tag::Int(1));
    }
    fields.insert("tic_upgrades".into(),upgrades);
    upgraded.data_mut().set_other("minecraft:custom_data",Some(custom));
    let mut output = assemble(world,definition,&tool.item,&identities,Some(&upgraded),reason)?;
    let cap = super::tconstruct_source_overslime::amount(&output,"tic_overslime_capacity") as u32;
    let before = super::tconstruct_source_overslime::amount(tool,"tic_overslime") as u32;
    let available = inputs.iter().map(|(_,s)|s.count).sum();
    let overworked = traits.get("tconstruct:overworked").copied().unwrap_or(0).max(0) as u32;
    let (after, mut remaining) = super::tconstruct_source_overslime::refill(before,cap,restore,available,overworked)?;
    let mut consumed = vec![(0,1)];
    for (index,stack) in inputs { let count = stack.count.min(remaining); if count>0 {consumed.push((index,count));remaining-=count;} }
    let bottles = if item.ends_with("_slime_bottle") { consumed.iter().skip(1).map(|(_,n)|*n).sum() } else { 0 };
    let leftovers = if bottles>0 {vec![ItemStack::new("minecraft:glass_bottle",bottles)]} else {Vec::new()};
    let mut custom=output.data()?.other("minecraft:custom_data")?.clone();
    let Tag::Compound(ref mut fields)=custom else {return None;};
    fields.insert("tic_overslime".into(),Tag::Int(after as i32));
    output.data_mut().set_other("minecraft:custom_data",Some(custom));
    Some(StationOperation {output,consumed,leftovers})
}

/// Construct the worktable extraction output without changing inventory.
pub(super) fn extraction_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>], id: &str) -> Option<StationOperation> {
    let tool = slots.first()?.as_ref()?;
    let Tag::List(materials) = tool.data()?.other("minecraft:custom_data")?.get("tic_materials")? else { return None; };
    let identities = materials.iter().map(|value| match value {
        Tag::String(id) => Some(id.to_string_lossy().to_string()), _ => None,
    }).collect::<Option<Vec<_>>>()?;
    let definitions = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let (namespace, name) = tool.item.split_once(':')?;
    let definition = definitions.get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?.last()?;
    let tags = world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let traits = super::tconstruct_source_material_traits::levels(world, tool).map(|v| v.levels).unwrap_or_default();
    let plan = super::tconstruct_source_upgrade_rules::extract_upgrade_with_traits(tags, tool, slots.get(1..=2)?, id, &traits)?;
    let mut modified = super::tconstruct_source_tool_normalize::prepare_for_tool_mutation(tool).ok()?;
    let mut custom = modified.data()?.other("minecraft:custom_data")?.clone();
    let Tag::Compound(ref mut fields) = custom else { return None; };
    let Tag::Compound(upgrades) = fields.get_mut(&"tic_upgrades".into())? else { return None; };
    if plan.upgrade.amount == 0 { upgrades.remove(&id.into()); }
    else { upgrades.insert(id.into(), Tag::Int(plan.upgrade.amount)); }
    modified.data_mut().set_other("minecraft:custom_data", Some(custom));
    let output = assemble(world, definition, &tool.item, &identities, Some(&modified), &mut None)?;
    let mut leftovers = plan.upgrade.leftover.into_iter().collect::<Vec<_>>();
    leftovers.push(plan.sponge);
    Some(StationOperation { output, consumed: plan.upgrade.consumed, leftovers })
}

/// Revalidate and commit a planned operation; return any item to give to the player.
pub(super) fn take_operation(slots: &mut [Option<ItemStack>], carried: &mut Option<ItemStack>, plan: StationOperation) -> Option<Vec<ItemStack>> {
    let bottle_slots: Vec<(usize, u32)> = plan.consumed.iter().filter_map(|&(slot, count)|
        slots.get(slot).and_then(Option::as_ref).filter(|s| &*s.item == "minecraft:dragon_breath" || s.item.ends_with("_slime_bottle")).map(|_| (slot, count))).collect();
    if !take_repair(slots, carried, plan.output, &plan.consumed) { return None; }
    let mut leftovers = plan.leftovers;
    if let Some(index) = leftovers.iter().position(|s| &*s.item == "minecraft:glass_bottle") {
        for (slot, count) in bottle_slots {
            if slots[slot].is_none() && leftovers[index].count > 0 {
                let count = count.min(leftovers[index].count);
                slots[slot] = Some(ItemStack::new("minecraft:glass_bottle", count));
                leftovers[index].count -= count;
            }
        }
        if leftovers[index].count == 0 { leftovers.remove(index); }
    }
    Some(leftovers)
}

/// Plan an upgrade, replacement or repair without consuming inputs.
pub(super) fn operation_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>]) -> Option<StationOperation> {
    operation_plan_with_capacity(world, slots, 3)
}

pub(super) fn operation_plan_with_capacity(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>], input_capacity: usize) -> Option<StationOperation> {
    operation_plan_diagnostic(world, slots, input_capacity, &mut None)
}

pub(super) fn operation_plan_diagnostic(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>], input_capacity: usize, reason: &mut Option<String>) -> Option<StationOperation> {
    if !matches!(input_capacity, 3 | 5) { return None; }
    if let Some(plan) = overslime_plan(world, slots, input_capacity, reason) { *reason = None; return Some(plan); }
    if let Some(plan) = modifier_repair_plan(world, slots, input_capacity, reason) { *reason = None; return Some(plan); }
    if let Some(plan) = upgrade_plan(world, slots, input_capacity, reason) { *reason = None; return Some(plan); }
    if reason.as_deref().is_some_and(|message| message.starts_with("Unavailable upgrade in this development build:")) { return None; }
    if let Some((output, consumed)) = swap_plan_with_capacity(world, slots, input_capacity, reason).or_else(|| repair_plan_with_capacity(world, slots, input_capacity, reason)) {
        *reason = None; return Some(StationOperation { output, consumed, leftovers: Vec::new() });
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
    Some(StationOperation { output: output?, consumed, leftovers: Vec::new() })
}
