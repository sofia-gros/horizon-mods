// Resolve ordinary Part Builder candidates against the current world's
// snapshots. This is preparation for gameplay integration; it fails closed
// when the original reuse or fake-material context is unavailable.
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
    choices_from(world, pattern, material, recipes)
}

fn choices_from(
    world: &super::tconstruct_world::World,
    pattern: Option<&ItemStack>,
    material: Option<&ItemStack>,
    recipes: &std::collections::BTreeMap<String, Vec<u8>>,
) -> Result<Vec<SourceAllPartChoice>, SourceLookupError> {
    let Some(pattern) = pattern else { return Ok(Vec::new()); };
    let chunks = &world.chunks;
    let items = chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let fluids = chunks.__horizon_mixins_tconstruct_resolved_fluid_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let config = chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let (definitions, _, material_tags) = chunks.__horizon_mixins_tconstruct_resolved_material_definitions().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let stats = chunks.__horizon_mixins_tconstruct_resolved_material_stats().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let registries: HashMap<_, _> = [("minecraft:item".to_owned(), items.clone()), ("minecraft:fluid".to_owned(), fluids.clone())].into_iter().collect();
    let instruments = HashSet::new();
    let instrument_tags = HashMap::new();
    let id = material.map_or("", |s| &*s.item);
    let count = material.and_then(|s| i32::try_from(s.count).ok()).unwrap_or(0);
    // The original takes a distinct route for tagged material-bearing parts.
    // Until that tag and casting costs are complete, do not silently use a
    // material recipe for them.
    if material.is_some() && (items.get("tconstruct:parts").is_some_and(|tag| tag.contains(id))
        || !items.contains_key("tconstruct:parts") && id.starts_with("tconstruct:")) {
        return Err(SourceLookupError::IncompleteContext);
    }
    // Instrument-based ingredient matches require the instrument component and
    // its registry, which are not present in the snapshot yet.
    if material.is_some() && (id == "minecraft:goat_horn" || items.get("minecraft:goat_horns").is_some_and(|tag| tag.contains(id))) {
        return Err(SourceLookupError::IncompleteContext);
    }
    let found = if material.is_some() {
        tx::source_material_lookup(recipes, id, None, items, &instruments, &instrument_tags, &registries, config, true)?
    } else { None };
    let mut supported = HashSet::new();
    let mut craftable = false;
    if let Some(found) = found.as_ref() {
        let base = found.material.split_once('#').map_or(found.material.as_str(), |(base, _)| base);
        let definition = definitions.get(base).ok_or(SourceLookupError::IncompleteContext)?;
        craftable = definition.0;
        if let Some(stats) = stats.get(base) {
            supported = tx::source_supported_original_parts(&stats.keys().cloned().collect(), stats.values().any(|v| *v));
        }
        // Fake ingot/storage eligibility also depends on the full item registry
        // and original visible material recipes, not only material membership.
        for tag in ["tconstruct:compatibility_metals", "tconstruct:compatibility_metals/blocks"] {
            let ids = material_tags.get(tag).ok_or(SourceLookupError::IncompleteContext)?;
            if ids.contains(base) { return Err(SourceLookupError::IncompleteContext); }
        }
    }
    let input = match (material, found.as_ref()) {
        (None, _) => SourcePartInput::Empty,
        (Some(_), Some(found)) => SourcePartInput::Material { input_item: id, material: &found.material, craftable, supported_outputs: &supported },
        (Some(_), None) => SourcePartInput::Invalid,
    };
    tx::source_all_part_choices(recipes, &*pattern.item, &input, id, count, items, &instruments, &instrument_tags, &registries, config, None, None, true)
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
        choices_from(world, Some(pattern), material, recipes)?.into_iter().find(|c| c.pattern == selected).map(|c| c.id)
    };
    let Some(choice_id) = choice_id else { return Ok(None); };
    let items = chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let fluids = chunks.__horizon_mixins_tconstruct_resolved_fluid_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let config = chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let (definitions, _, _) = chunks.__horizon_mixins_tconstruct_resolved_material_definitions().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let stats = chunks.__horizon_mixins_tconstruct_resolved_material_stats().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let bytes = recipes.get(&choice_id).ok_or(SourceLookupError::IncompleteContext)?;
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| SourceLookupError::UnsupportedRecipe)?;
    if value.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:part_builder_recycling") {
        let recipe = tx::SourceRecyclingRecipe::parse(choice_id, &value).ok_or(SourceLookupError::UnsupportedRecipe)?;
        let Some(stack) = material else { return Ok(None); };
        let max_damage = super::tconstruct_source_original_item_durability::max_damage(&stack.item).ok_or(SourceLookupError::IncompleteContext)?;
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
            if chunks.__horizon_mixins_tconstruct_source_selection_origin().get(&pos)
                != Some(&(selected.to_owned(), recipe.id.clone(), epoch)) {
                return Err(SourceLookupError::IncompleteContext);
            }
            for (key, other) in &recipe.results {
                if key != selected {
                    let count = (recipe.maximum_amount(max_damage, damage)? - original_count).min(other.count);
                    alternatives.push((other.clone(), count));
                }
            }
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
    let registries: HashMap<_, _> = [("minecraft:item".to_owned(), items.clone()), ("minecraft:fluid".to_owned(), fluids.clone())].into_iter().collect();
    let instruments = HashSet::new();
    let instrument_tags = HashMap::new();
    let id = material.map_or("", |s| &*s.item);
    let found = if material.is_some() {
        tx::source_material_lookup(recipes, id, None, items, &instruments, &instrument_tags, &registries, config, true)?
    } else { None };
    let mut supported = HashSet::new();
    let mut craftable = false;
    if let Some(found) = found.as_ref() {
        let base = found.material.split_once('#').map_or(found.material.as_str(), |(base, _)| base);
        let definition = definitions.get(base).ok_or(SourceLookupError::IncompleteContext)?;
        craftable = definition.0;
        if let Some(stats) = stats.get(base) {
            supported = tx::source_supported_original_parts(&stats.keys().cloned().collect(), stats.values().any(|v| *v));
        }
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
        let Ok(candidates) = choices_from(world, slots.first().and_then(Option::as_ref), slots.get(1).and_then(Option::as_ref), recipes) else { return false; };
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
    let candidates = recipes.as_ref().and_then(|recipes| choices_from(world, pattern.as_ref(), material.as_ref(), recipes).ok());
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

/// Obtain a current native crafting container, rejecting item-specific
/// component overrides until the original 1.20.1 remainder registry is ported.
pub(super) fn crafting_container(stack: Option<&ItemStack>, items: &horizon_items::items::Items) -> Result<Option<ItemStack>, SourceLookupError> {
    let Some(stack) = stack else { return Ok(None); };
    if stack.data().and_then(|data| data.other("minecraft:crafting_remainder")).is_some() { return Err(SourceLookupError::IncompleteContext); }
    let item = items.get(&stack.item).ok_or(SourceLookupError::IncompleteContext)?;
    Ok(item.remainder.as_deref().map(|id| ItemStack::new(id, 1)))
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

/// Seed the source's non-deterministic default Java Random from the Linux
/// monotonic clock and the first Java seed-uniquifier step. Initialization is
/// lazy in this port; the random sequence is not a cross-process fixed value.
#[cfg(unix)]
fn source_random_seed() -> i64 {
    let time = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    let nanos = (time.tv_sec as u64).wrapping_mul(1_000_000_000).wrapping_add(time.tv_nsec as u64);
    (8682522807148012u64.wrapping_mul(1181783497276652981) ^ nanos) as i64
}

/// Horizon exposes rustix's monotonic clock on Unix. Until the original
/// Windows clock is wired, fail the random transaction without consuming.
pub(super) fn source_random_draw(items: &horizon_items::items::Items, bound: usize) -> Option<usize> {
    #[cfg(unix)] { items.__horizon_mixins_tconstruct_recycling_draw(source_random_seed, bound) }
    #[cfg(not(unix))] { let _ = (items, bound); None }
}
