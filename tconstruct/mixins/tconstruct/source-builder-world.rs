// Resolve Part Builder candidates against the current world's snapshots.
// Unavailable fake-material context cannot be inferred from an incomplete tag.
use super::tconstruct_tx::{self as tx, SourceAllPartChoice, SourceLookupError, SourcePartInput};
use horizon_items::items::ItemStack;
use std::collections::{HashMap, HashSet};

pub(super) fn choices(
    world: &super::tconstruct_world::World,
    pattern: Option<&ItemStack>,
    material: Option<&ItemStack>,
) -> Result<Vec<SourceAllPartChoice>, SourceLookupError> {
    if pattern.is_none() { return Ok(Vec::new()); }
    let recipes = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref().ok_or(SourceLookupError::IncompleteContext)?;
    choices_from(world, pattern, material, recipes, None)
}

fn choices_from(
    world: &super::tconstruct_world::World,
    pattern: Option<&ItemStack>,
    material: Option<&ItemStack>,
    recipes: &std::collections::BTreeMap<String, Vec<u8>>,
    cached_material: Option<&Option<tx::SourceMaterialMatch>>,
) -> Result<Vec<SourceAllPartChoice>, SourceLookupError> {
    let Some(pattern) = pattern else { return Ok(Vec::new()); };
    let chunks = &world.chunks;
    let items = chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let fluids = chunks.__horizon_mixins_tconstruct_resolved_fluid_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let config = chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let registries: HashMap<_, _> = [("minecraft:item".to_owned(), items.clone()), ("minecraft:fluid".to_owned(), fluids.clone())].into_iter().collect();
    let instruments = HashSet::new();
    let instrument_tags = HashMap::new();
    let id = material.map_or("", |s| &*s.item);
    let count = material.and_then(|s| i32::try_from(s.count).ok()).unwrap_or(0);
    // Instrument-based ingredient matches require the instrument component and
    // its registry, which are not present in the snapshot yet.
    if cached_material.is_none() && material.is_some() && (id == "minecraft:goat_horn" || items.get("minecraft:goat_horns").is_some_and(|tag| tag.contains(id))) {
        return Err(SourceLookupError::IncompleteContext);
    }
    let found = if let Some(cached) = cached_material {
        cached.clone()
    } else if let Some(material) = material {
        lookup_material(world, material)?
    } else { None };
    let mut supported = HashSet::new();
    let mut craftable = false;
    if let Some(found) = found.as_ref() {
        (craftable, supported) = supported_material(world, &found.material)?;
    }
    let input = match (material, found.as_ref()) {
        (None, _) => SourcePartInput::Empty,
        (Some(_), Some(found)) => SourcePartInput::Material { input_item: id, material: &found.material, craftable, supported_outputs: &supported },
        (Some(_), None) => SourcePartInput::Invalid,
    };
    tx::source_all_part_choices(recipes, &*pattern.item, &input, id, count, items, &instruments, &instrument_tags, &registries, config, None, None, true)
}

/// Cached `IMaterialValue` is independent of the pattern/recipe list and is
/// invalidated only when the material item or its data changes.
fn cached_material(world: &super::tconstruct_world::World, pos: (i32, i32, i32), material: Option<&ItemStack>) -> Option<Option<tx::SourceMaterialMatch>> {
    let material = material?;
    let (old, bytes) = world.chunks.__horizon_mixins_tconstruct_builder_material_cache().get(&pos)?;
    if !old.same_item_same_components(material) { return None; }
    serde_json::from_slice(bytes).ok()
}

/// The selected recipe cost and the exact value of the current input stack.
/// Values are sent to LAN viewers from the authoritative recipe snapshot.
pub(super) fn display_info(world: &super::tconstruct_world::World, pos: (i32, i32, i32)) -> Vec<String> {
    let chunks = &world.chunks;
    let recipe_value: Option<serde_json::Value> = chunks.__horizon_mixins_tconstruct_source_selection_origin().get(&pos)
        .and_then(|(_, id, _)| chunks.__horizon_mixins_tconstruct_builder_recipe_context().get(&pos)
            .and_then(|(_, recipes, _)| recipes.get(id)))
        .and_then(|bytes| serde_json::from_slice(bytes).ok());
    let recipe = recipe_value.as_ref().and_then(|value| tx::SourcePartRecipe::parse(String::new(), value));
    let title = recipe_value.as_ref().and_then(|value| value.get("title_key")).and_then(serde_json::Value::as_str).unwrap_or("-");
    let cost = recipe.as_ref().map_or(0, |recipe| recipe.cost);
    let material = world.block_entities.get(&pos).and_then(|entity| entity.slots().get(1)).and_then(Option::as_ref);
    let found = cached_material(world, pos, material).flatten();
    let (id, value) = if let (Some(stack), Some(found)) = (material, found) {
        if found.needed > 0 { (found.material, stack.count as f64 * f64::from(found.value) / f64::from(found.needed)) }
        else { (String::new(), 0.0) }
    } else { (String::new(), 0.0) };
    let mut stats = std::collections::BTreeMap::new();
    let mut traits = std::collections::BTreeMap::new();
    if let (Some((namespace, material)), Some(snapshot)) = (id.split_once(':'), chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()) {
        let material = material.split('#').next().unwrap_or(material);
        if let Some(layers) = snapshot.get(&format!("{namespace}:tinkering/materials/stats/{material}")) {
            if let Ok(merged) = tx::source_merge_material_stats(layers) {
                stats = merged;
            }
        }
        if let Some(layers) = snapshot.get(&format!("{namespace}:tinkering/materials/traits/{material}")) {
            if let Ok(raw) = tx::source_merge_material_traits(layers) {
                for stat in stats.keys() {
                    if let Some(kind) = stat.strip_prefix("tconstruct:") {
                        let mut selected = std::collections::BTreeMap::new();
                        super::tconstruct_source_material_trait_select::add_traits(&mut selected, raw.default.as_ref(), &raw.per_stat, kind);
                        if !selected.is_empty() { traits.insert(stat.clone(), selected); }
                    }
                }
            }
        }
    }
    vec![cost.to_string(), if id.is_empty() { "-".into() } else { id }, value.to_string(),
        serde_json::to_string(&stats).unwrap_or_else(|_| "{}".into()), serde_json::to_string(&traits).unwrap_or_else(|_| "{}".into()), title.to_owned()]
}

pub(super) fn lookup_material(world: &super::tconstruct_world::World, material: &ItemStack) -> Result<Option<tx::SourceMaterialMatch>, SourceLookupError> {
    let chunks = &world.chunks;
    let recipes = chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref().ok_or(SourceLookupError::IncompleteContext)?;
    let items = chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let fluids = chunks.__horizon_mixins_tconstruct_resolved_fluid_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let config = chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let id = &*material.item;
    if !items.contains_key("tconstruct:parts") && id.starts_with("tconstruct:")
        || id == "minecraft:goat_horn" || items.get("minecraft:goat_horns").is_some_and(|tag| tag.contains(id)) {
        return Err(SourceLookupError::IncompleteContext);
    }
    let registries = [("minecraft:item".to_owned(), items.clone()), ("minecraft:fluid".to_owned(), fluids.clone())].into_iter().collect();
    if items.get("tconstruct:parts").is_some_and(|tag| tag.contains(id)) {
        let registered = [id.to_owned()].into_iter().collect();
        let relevant = recipes.iter().filter_map(|(recipe_id, bytes)| {
            let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
            (value.get("result").and_then(serde_json::Value::as_str) == Some(id)).then(|| (recipe_id.clone(), bytes.clone()))
        }).collect();
        let costs = tx::source_material_casting_costs(&relevant, &registered, &registries, config, true)?;
        let material_id = material.data().and_then(|data| data.other("minecraft:custom_data"))
            .and_then(|tag| tag.get("Material"))
            .and_then(|tag| match tag { horizon_world::nbt::Tag::String(value) => Some(value.to_string_lossy()), _ => None });
        return match tx::source_reused_part_material(true, true, material_id.as_deref(), costs.get(id).copied(), true)? {
            tx::SourceReusedPart::Material { material, value, needed } => Ok(Some(tx::SourceMaterialMatch { id: id.to_owned(), material, value, needed, leftover: None })),
            _ => Ok(None),
        };
    }
    tx::source_material_lookup(recipes, id, None, items, &HashSet::new(), &HashMap::new(), &registries, config, true)
}

pub(super) fn supported_material(world: &super::tconstruct_world::World, material: &str) -> Result<(bool, HashSet<String>), SourceLookupError> {
    let chunks = &world.chunks;
    let (definitions, _, tags) = chunks.__horizon_mixins_tconstruct_resolved_material_definitions().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let stats = chunks.__horizon_mixins_tconstruct_resolved_material_stats().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let base = material.split_once('#').map_or(material, |(base, _)| base);
    let craftable = definitions.get(base).ok_or(SourceLookupError::IncompleteContext)?.0;
    let mut supported = stats.get(base).map_or_else(HashSet::new, |stats| tx::source_supported_original_parts(&stats.keys().cloned().collect(), stats.values().any(|v| *v)));
    let ingots = tags.get("tconstruct:compatibility_metals").ok_or(SourceLookupError::IncompleteContext)?;
    let blocks = tags.get("tconstruct:compatibility_metals/blocks").ok_or(SourceLookupError::IncompleteContext)?;
    if ingots.contains(base) || blocks.contains(base) {
        let recipes = chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref().ok_or(SourceLookupError::IncompleteContext)?;
        let item_tags = chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
        let fluid_tags = chunks.__horizon_mixins_tconstruct_resolved_fluid_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
        let config = chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
        let known = chunks.__horizon_mixins_tconstruct_source_native_durability().as_ref().ok_or(SourceLookupError::IncompleteContext)?.keys().cloned().collect();
        let visible = definitions.iter().filter(|(_, value)| !value.1).map(|(id, _)| id.clone()).collect();
        let registries = [("minecraft:item".to_owned(), item_tags.clone()), ("minecraft:fluid".to_owned(), fluid_tags.clone())].into_iter().collect();
        let values = tx::source_visible_base_recipe_values(base, recipes, &visible, &known, &registries, config, true)?;
        supported.extend(tx::source_supported_fake_items(base, tags, &values, true)?);
    }
    Ok((craftable, supported))
}

/// Compute one source result from the current authoritative world. Ordinary
/// parts and supported damageable-item recycling are handled; inventory mutation
/// and unsupported recycling cases remain separate.
pub(super) fn plan(
    world: &super::tconstruct_world::World,
    pos: (i32, i32, i32),
    pattern: Option<&ItemStack>,
    material: Option<&ItemStack>,
    selected: &str,
) -> Result<Option<tx::SourcePartPlan>, SourceLookupError> {
    let Some(pattern) = pattern else { return Ok(None); };
    let chunks = &world.chunks;
    let context = chunks.__horizon_mixins_tconstruct_builder_recipe_context().get(&pos);
    let recipes = context.map(|(_, recipes, _)| recipes.as_ref())
        .or_else(|| chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref())
        .ok_or(SourceLookupError::IncompleteContext)?;
    let choice_id = if let Some((_, _, entries)) = context {
        entries.iter().find(|(pattern, _)| pattern == selected).map(|(_, id)| id.clone())
    } else {
        choices_from(world, Some(pattern), material, recipes, cached_material(world, pos, material).as_ref())?.into_iter().find(|c| c.pattern == selected).map(|c| c.id)
    };
    let Some(choice_id) = choice_id else { return Ok(None); };
    let items = chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let bytes = recipes.get(&choice_id).ok_or(SourceLookupError::IncompleteContext)?;
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| SourceLookupError::UnsupportedRecipe)?;
    if value.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:part_builder_recycling") {
        let recipe = tx::SourceRecyclingRecipe::parse(choice_id, &value).ok_or(SourceLookupError::UnsupportedRecipe)?;
        let Some(stack) = material else { return Ok(None); };
        let durability = chunks.__horizon_mixins_tconstruct_source_native_durability().as_ref().ok_or(SourceLookupError::IncompleteContext)?;
        let override_damage = stack.data().and_then(|data| data.other("minecraft:max_damage")).is_some();
        let max_damage = super::tconstruct_source_native_durability::max_damage(&stack.item, durability, override_damage).ok_or(SourceLookupError::IncompleteContext)?;
        let damage = i32::try_from(stack.damage).map_err(|_| SourceLookupError::UnsupportedRecipe)?;
        // The original applies different checks to modifiable items. Do not
        // infer non-membership from an unresolved world tag.
        let modifiable = if let Some(values) = items.get("tconstruct:modifiable") {
            values.contains(&*stack.item)
        } else {
            let raw = chunks.__horizon_mixins_tconstruct_recipe_tag_snapshot().as_ref().ok_or(SourceLookupError::IncompleteContext)?;
            if !super::tconstruct_source_tags::definitely_absent("tconstruct:modifiable", &stack.item, raw)
                .map_err(|_| SourceLookupError::IncompleteContext)? { return Err(SourceLookupError::IncompleteContext); }
            false
        };
        if modifiable { return Err(SourceLookupError::IncompleteContext); }
        if !recipe.matches(true, max_damage, damage, false, false, !stack.enchantments().is_empty())? { return Ok(None); }
        let Some(output) = recipe.output(selected, max_damage, damage)? else { return Ok(None); };
        let original_count = recipe.results.get(selected).ok_or(SourceLookupError::IncompleteContext)?.count;
        let mut alternatives = Vec::new();
        if recipe.results.len() > 1 && recipe.maximum_amount(max_damage, damage)? > original_count {
            let epoch = context.map_or(*chunks.__horizon_mixins_tconstruct_source_recipe_generation(), |(epoch, _, _)| *epoch);
            let origin = chunks.__horizon_mixins_tconstruct_source_selection_origin().get(&pos)
                .filter(|(pattern, _, _)| pattern == selected).ok_or(SourceLookupError::IncompleteContext)?;
            let same_object = origin.1 == recipe.id && origin.2 == epoch;
            alternatives = recipe.alternatives(selected, same_object, max_damage, damage)?;
        }
        let reusable = if let Some(values) = items.get("tconstruct:patterns/reusable") {
            values.contains(&*pattern.item)
        } else {
            let raw = chunks.__horizon_mixins_tconstruct_recipe_tag_snapshot().as_ref().ok_or(SourceLookupError::IncompleteContext)?;
            if !super::tconstruct_source_tags::definitely_absent("tconstruct:patterns/reusable", &pattern.item, raw)
                .map_err(|_| SourceLookupError::IncompleteContext)? { return Err(SourceLookupError::IncompleteContext); }
            false
        };
        let count = output.count;
        return Ok(Some(tx::SourcePartPlan {
            output: tx::SourcePartOutput::Item { output, count }, materials_used: 1,
            pattern_used: i32::from(!reusable), leftover: None, random_alternatives: alternatives,
        }));
    }
    let recipe = tx::SourcePartRecipe::parse(choice_id, &value).ok_or(SourceLookupError::UnsupportedRecipe)?;
    let id = material.map_or("", |s| &*s.item);
    let found = if let Some(cached) = cached_material(world, pos, material) {
        cached
    } else if let Some(material) = material {
        lookup_material(world, material)?
    } else { None };
    let mut supported = HashSet::new();
    let mut craftable = false;
    if let Some(found) = found.as_ref() {
        (craftable, supported) = supported_material(world, &found.material)?;
    }
    let input = match (material, found.as_ref()) {
        (None, _) => SourcePartInput::Empty,
        (Some(_), Some(found)) => SourcePartInput::Material { input_item: id, material: &found.material, craftable, supported_outputs: &supported },
        (Some(_), None) => SourcePartInput::Invalid,
    };
    let available = material.and_then(|s| i32::try_from(s.count).ok()).unwrap_or(0);
    // An unrelated missing cast can prevent the whole tag from resolving.
    // False is safe only if the raw, pack-merged tag graph cannot contain
    // this specific pattern; a possible match remains explicitly unknown.
    let reusable = if let Some(values) = items.get("tconstruct:patterns/reusable") {
        values.contains(&*pattern.item)
    } else {
        let raw = chunks.__horizon_mixins_tconstruct_recipe_tag_snapshot().as_ref().ok_or(SourceLookupError::IncompleteContext)?;
        if !super::tconstruct_source_tags::definitely_absent("tconstruct:patterns/reusable", &*pattern.item, raw)
            .map_err(|_| SourceLookupError::IncompleteContext)? { return Err(SourceLookupError::IncompleteContext); }
        false
    };
    tx::source_part_plan(&recipe, &input, found.as_ref(), available, reusable)
}

/// Record the original selected Pattern's source recipe identity and the
/// current recipe generation. The source keeps the selected object across
/// input changes, so a later recipe with the same Pattern may be a new object.
pub(super) fn select(world: &mut super::tconstruct_world::World, pos: (i32, i32, i32), selected: String) -> bool {
    let Some(slots) = world.block_entities.get(&pos).map(|entity| entity.slots()) else { return false; };
    let context = world.chunks.__horizon_mixins_tconstruct_builder_recipe_context().get(&pos);
    let Some(recipes) = context.map(|(_, recipes, _)| recipes.as_ref())
        .or_else(|| world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()) else { return false; };
    let choice_id = if let Some((_, _, entries)) = context {
        entries.iter().find(|(pattern, _)| pattern == &selected).map(|(_, id)| id.clone())
    } else {
        let Ok(candidates) = choices_from(world, slots.first().and_then(Option::as_ref), slots.get(1).and_then(Option::as_ref), recipes, cached_material(world, pos, slots.get(1).and_then(Option::as_ref)).as_ref()) else { return false; };
        candidates.into_iter().find(|choice| choice.pattern == selected).map(|choice| choice.id)
    };
    let Some(choice_id) = choice_id else { return false; };
    let epoch = context.map_or(*world.chunks.__horizon_mixins_tconstruct_source_recipe_generation(), |(epoch, _, _)| *epoch);
    world.chunks.__horizon_mixins_tconstruct_source_selection_origin_mut().insert(pos, (selected.clone(), choice_id, epoch));
    world.chunks.__horizon_mixins_tconstruct_part_selections_mut().insert(pos, selected);
    world.chunks.__horizon_mixins_tconstruct_builder_preview_mut().remove(&pos);
    true
}

/// Cache source candidates per world and table, recomputing only after either
/// input changes. Block/chunk removal clears the table cache; recipe reload
/// retains the source recipe objects already held by an active table.
pub(super) fn refresh(world: &mut super::tconstruct_world::World, pos: (i32, i32, i32)) -> Option<Vec<String>> {
    let Some(slots) = world.block_entities.get(&pos).map(|entity| entity.slots()) else {
        world.chunks.__horizon_mixins_tconstruct_builder_choices_mut().remove(&pos);
        return None;
    };
    let pattern = slots.first().cloned().flatten();
    let material = slots.get(1).cloned().flatten();
    if let Some((old_pattern, old_material, result)) = world.chunks.__horizon_mixins_tconstruct_builder_choices().get(&pos) {
        if tx::source_candidate_inputs_equal(old_pattern.as_ref(), pattern.as_ref(), old_material.as_ref(), material.as_ref()) { return result.clone(); }
    }
    let epoch = *world.chunks.__horizon_mixins_tconstruct_source_recipe_generation();
    let recipes = world.chunks.__horizon_mixins_tconstruct_source_shared_recipes().clone();
    let mut cached = cached_material(world, pos, material.as_ref());
    if cached.is_none() {
        world.chunks.__horizon_mixins_tconstruct_builder_material_cache_mut().remove(&pos);
        if let (Some(material), Some(recipes)) = (material.as_ref(), recipes.as_ref()) {
            let chunks = &world.chunks;
            let items = chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok());
            let fluids = chunks.__horizon_mixins_tconstruct_resolved_fluid_tags().as_ref().and_then(|v| v.as_ref().ok());
            let config = chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref().and_then(|v| v.as_ref().ok());
            let needs_lookup = if let (Some(items), Some(fluids), Some(config)) = (items, fluids, config) {
                let registries = [("minecraft:item".to_owned(), items.clone()), ("minecraft:fluid".to_owned(), fluids.clone())].into_iter().collect();
                pattern.as_ref().is_none_or(|pattern| tx::source_needs_material_lookup(recipes, &pattern.item, &material.item, items, &registries, config).ok() == Some(true))
            } else { false };
            if needs_lookup {
                if let Ok(value) = lookup_material(world, material) {
                    if let Ok(bytes) = serde_json::to_vec(&value) {
                        world.chunks.__horizon_mixins_tconstruct_builder_material_cache_mut().insert(pos, (material.clone(), bytes));
                        cached = Some(value);
                    }
                }
            }
        }
    }
    let candidates = recipes.as_ref().and_then(|recipes| choices_from(world, pattern.as_ref(), material.as_ref(), recipes, cached.as_ref()).ok());
    let result = candidates.as_ref().map(|items| items.iter().map(|c| c.pattern.clone()).collect());
    if let (Some(recipes), Some(candidates)) = (recipes, candidates) {
        let entries = candidates.into_iter().map(|c| (c.pattern, c.id)).collect();
        world.chunks.__horizon_mixins_tconstruct_builder_recipe_context_mut().insert(pos, (epoch, recipes, entries));
    } else {
        world.chunks.__horizon_mixins_tconstruct_builder_recipe_context_mut().remove(&pos);
    }
    world.chunks.__horizon_mixins_tconstruct_builder_choices_mut().insert(pos, (pattern, material, result.clone()));
    result
}

/// Cache the selected source result without mutating either input slot. The
/// caller supplies the authoritative item registry, and revalidates on take.
pub(super) fn refresh_preview(
    world: &mut super::tconstruct_world::World, pos: (i32, i32, i32),
    registered: impl FnOnce() -> HashSet<String>,
) -> Option<ItemStack> {
    let Some(slots) = world.block_entities.get(&pos).map(|entity| entity.slots()) else {
        world.chunks.__horizon_mixins_tconstruct_builder_preview_mut().remove(&pos);
        return None;
    };
    let pattern = slots.first().cloned().flatten();
    let material = slots.get(1).cloned().flatten();
    let selected = world.chunks.__horizon_mixins_tconstruct_part_selections().get(&pos).cloned();
    if let Some((old_pattern, old_material, old_selected, result)) = world.chunks.__horizon_mixins_tconstruct_builder_preview().get(&pos) {
        if old_pattern == &pattern && old_material == &material && old_selected == &selected { return result.clone(); }
    }
    let result = selected.as_deref().and_then(|selected| {
        let plan = plan(world, pos, pattern.as_ref(), material.as_ref(), selected).ok().flatten()?;
        let items = world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
        tx::source_resolve_part_plan(&plan, items, None, &registered()).ok().flatten().map(|(output, _, _)| output)
    });
    world.chunks.__horizon_mixins_tconstruct_builder_preview_mut().insert(pos, (pattern, material, selected, result.clone()));
    result
}

/// Revalidate and resolve the currently selected source recipe immediately
/// before a player takes its output. No inventory has been mutated yet.
pub(super) fn prepare_take(
    world: &super::tconstruct_world::World, pos: (i32, i32, i32),
    items: &horizon_items::items::Items,
) -> Result<Option<(tx::SourcePartPlan, ItemStack, Option<ItemStack>, Vec<ItemStack>)>, SourceLookupError> {
    let Some(slots) = world.block_entities.get(&pos).map(|entity| entity.slots()) else { return Ok(None); };
    let Some(selected) = world.chunks.__horizon_mixins_tconstruct_part_selections().get(&pos) else { return Ok(None); };
    let Some(plan) = plan(world, pos, slots.first().and_then(Option::as_ref), slots.get(1).and_then(Option::as_ref), selected)? else { return Ok(None); };
    let tags = world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let registered = items.registry_order.iter().map(|id| id.to_string()).collect();
    let Some((output, leftover, alternatives)) = tx::source_resolve_part_plan(&plan, tags, None, &registered)? else { return Ok(None); };
    Ok(Some((plan, output, leftover, alternatives)))
}

/// Obtain the crafting container from Horizon item data. Item-specific
/// component overrides and non-vanilla custom container behavior remain incomplete.
pub(super) fn crafting_container(stack: Option<&ItemStack>, items: &horizon_items::items::Items) -> Result<Option<ItemStack>, SourceLookupError> {
    let Some(stack) = stack else { return Ok(None); };
    if stack.data().and_then(|data| data.other("minecraft:crafting_remainder")).is_some() { return Err(SourceLookupError::IncompleteContext); }
    let item = items.get(&stack.item).ok_or(SourceLookupError::IncompleteContext)?;
    // Crafting remainder and use remainder have distinct meanings in Horizon.
    // TConstruct items can currently provide their container through use remainder.
    let remainder = if stack.item.starts_with("minecraft:") {
        item.crafting_remainder.as_deref()
    } else { item.crafting_remainder.as_deref().or(item.remainder.as_deref()) };
    Ok(remainder.map(|id| ItemStack::new(id, 1)))
}

/// Until the original adjacent inventory modules are available, do not run a
/// player-only quick move if a neighbor might supply one, or is not loaded.
pub(super) fn player_only_quick_move(world: &super::tconstruct_world::World, pos: (i32, i32, i32)) -> bool {
    [(0, -1), (1, 0), (0, 1), (-1, 0)].into_iter().all(|(dx, dz)| {
        let neighbor = (pos.0 + dx, pos.1, pos.2 + dz);
        world.chunks.contains_key(&(neighbor.0 >> 4, neighbor.2 >> 4))
            && !world.block_entities.contains_key(&neighbor)
    })
}

// Seed the shared Rust generator from the system clock on every platform.
fn source_random_seed() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as i64
}

pub(super) fn source_random_draw(items: &horizon_items::items::Items, bound: usize) -> Option<usize> {
    items.__horizon_mixins_tconstruct_recycling_draw(source_random_seed, bound)
}

/// The fake block item accepts only compatibility materials that do not have
/// a visible nine-unit material recipe, as in the source item.
pub(super) fn fake_storage_can_use(world: &super::tconstruct_world::World, items: &horizon_items::items::Items, material: &str) -> Result<bool, SourceLookupError> {
    let chunks = &world.chunks;
    let (definitions, _, material_tags) = chunks.__horizon_mixins_tconstruct_resolved_material_definitions().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let item_tags = chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let fluid_tags = chunks.__horizon_mixins_tconstruct_resolved_fluid_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let config = chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let recipes = chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref().ok_or(SourceLookupError::IncompleteContext)?;
    let base = material.split_once('#').map_or(material, |(base, _)| base);
    let visible = definitions.iter().filter(|(_, value)| !value.1).map(|(id, _)| id.clone()).collect();
    let known = items.registry_order.iter().map(|id| id.to_string()).collect();
    let registries = [("minecraft:item".to_owned(), item_tags.clone()), ("minecraft:fluid".to_owned(), fluid_tags.clone())].into_iter().collect();
    let values = tx::source_visible_base_recipe_values(base, recipes, &visible, &known, &registries, config, true)?;
    Ok(tx::source_supported_fake_items(base, material_tags, &values, true)?.contains("tconstruct:fake_storage_block"))
}
