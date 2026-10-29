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
    let Some(pattern) = pattern else { return Ok(Vec::new()); };
    let chunks = &world.chunks;
    let recipes = chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref().ok_or(SourceLookupError::IncompleteContext)?;
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

/// Compute one original normal-part result from the current authoritative
/// world. Recycling, result resolution and inventory mutation remain separate.
pub(super) fn plan(
    world: &super::tconstruct_world::World,
    pattern: Option<&ItemStack>,
    material: Option<&ItemStack>,
    selected: &str,
) -> Result<Option<tx::SourcePartPlan>, SourceLookupError> {
    let Some(pattern) = pattern else { return Ok(None); };
    let Some(choice) = choices(world, Some(pattern), material)?.into_iter().find(|c| c.pattern == selected) else { return Ok(None); };
    let chunks = &world.chunks;
    let recipes = chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref().ok_or(SourceLookupError::IncompleteContext)?;
    let items = chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let fluids = chunks.__horizon_mixins_tconstruct_resolved_fluid_tags().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let config = chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let (definitions, _, _) = chunks.__horizon_mixins_tconstruct_resolved_material_definitions().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let stats = chunks.__horizon_mixins_tconstruct_resolved_material_stats().as_ref().and_then(|v| v.as_ref().ok()).ok_or(SourceLookupError::IncompleteContext)?;
    let bytes = recipes.get(&choice.id).ok_or(SourceLookupError::IncompleteContext)?;
    let value = serde_json::from_slice(bytes).map_err(|_| SourceLookupError::UnsupportedRecipe)?;
    let recipe = tx::SourcePartRecipe::parse(choice.id, &value).ok_or(SourceLookupError::UnsupportedRecipe)?;
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

/// Cache source candidates per world and table, recomputing only after either
/// input changes. Pack reload and block/chunk removal clear the cache.
pub(super) fn refresh(world: &mut super::tconstruct_world::World, pos: (i32, i32, i32)) -> Option<Vec<String>> {
    let Some(slots) = world.block_entities.get(&pos).map(|entity| entity.slots()) else {
        world.chunks.__horizon_mixins_tconstruct_builder_choices_mut().remove(&pos);
        return None;
    };
    let pattern = slots.first().cloned().flatten();
    let material = slots.get(1).cloned().flatten();
    if let Some((old_pattern, old_material, result)) = world.chunks.__horizon_mixins_tconstruct_builder_choices().get(&pos) {
        if old_pattern == &pattern && old_material == &material { return result.clone(); }
    }
    let result = choices(world, pattern.as_ref(), material.as_ref()).ok()
        .map(|candidates| candidates.into_iter().map(|c| c.pattern).collect());
    world.chunks.__horizon_mixins_tconstruct_builder_choices_mut().insert(pos, (pattern, material, result.clone()));
    result
}
