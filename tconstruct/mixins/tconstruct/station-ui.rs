use super::*;
pub(super) fn kind(game: &Game<'_>) -> Option<bool> {
    if game.remote.is_some() {
        return match game.remote_menu_title().map(|t| t.plain()).as_deref() {
            Some("TConstruct Part Builder" | "TConstruct Part Builder Double") => Some(false),
            Some("TConstruct Tinker Station") => Some(true),
            _ => None,
        };
    }
    let key = game.container?;
    match &*game.res.facts().get(game.world.block(key.0, key.1, key.2)).name {
        "tconstruct:part_builder" => Some(false),
        "tconstruct:tinker_station" => Some(true),
        _ => None,
    }
}

pub(super) fn side_count(game: &Game<'_>) -> usize {
    if game.remote.is_some() {
        game.container.and_then(|key| game.world.chunks.__horizon_mixins_tconstruct_builder_side_size().get(&key).copied()).unwrap_or(game.container_size().saturating_sub(27))
    } else { game.container_size().saturating_sub(27) }
}

pub(super) fn station_layouts(game: &Game<'_>) -> Vec<super::tconstruct_source_station_layout::StationLayout> {
    let Some(snapshot) = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref() else { return Vec::new(); };
    let Some(rest) = game.res.rest.get() else { return Vec::new(); };
    let mut layouts = Vec::new();
    for (id, layers) in snapshot {
        if !id.contains(":tinkering/station_layouts/") { continue; }
        let Some(layout) = layers.last().and_then(|bytes| super::tconstruct_source_station_layout::StationLayout::parse(id.clone(), bytes)) else { continue; };
        let item = layout.icon.get("item").and_then(serde_json::Value::as_str);
        let pattern = layout.icon.get("pattern").and_then(serde_json::Value::as_str);
        if (1..=3).contains(&layout.input_slots.len()) && layout.tool_slot.is_none()
            && (item.is_some_and(|item| rest.items.items.contains_key(item)) || pattern.is_some()) { layouts.push(layout); }
    }
    layouts.sort_by(|a, b| (a.sort_index, &a.id).cmp(&(b.sort_index, &b.id)));
    // The repair layout is a pattern button, rather than an item icon.
    if let Some(layout) = snapshot.get("tconstruct:tinkering/station_layouts/tinker_station")
        .and_then(|layers| layers.last())
        .and_then(|bytes| super::tconstruct_source_station_layout::StationLayout::parse(
            "tconstruct:tinkering/station_layouts/tinker_station".into(), bytes)) {
        layouts.insert(0, layout);
    }
    layouts
}

/// Prefer this player's selected source layout, then infer it from inputs.
pub(super) fn station_layout(game: &Game<'_>) -> Option<super::tconstruct_source_station_layout::StationLayout> {
    let BlockEntity::Chest(slots) = game.container_kind()? else { return None; };
    if super::tconstruct_source_station_world::has_operation_tool(slots) {
        let snapshot = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
        let bytes = snapshot.get("tconstruct:tinkering/station_layouts/tinker_station")?.last()?;
        return super::tconstruct_source_station_layout::StationLayout::parse("tconstruct:tinkering/station_layouts/tinker_station".into(), bytes);
    }
    let layouts = station_layouts(game);
    if let Some(selected) = game.container.and_then(|key| game.world.chunks.__horizon_mixins_tconstruct_part_selections().get(&key)) {
        if let Some(layout) = layouts.iter().find(|layout| &layout.id == selected) { return Some(layout.clone()); }
    }
    let mut candidates = Vec::new();
    for layout in layouts {
        let mut present = false;
        let matches = layout.input_slots.iter().zip(slots.iter()).all(|(spec, stack)| {
            let Some(stack) = stack else { return true; };
            present = true;
            spec.accepts_item(stack.item.as_ref())
        });
        if present && matches { candidates.push(layout); }
    }
    candidates.into_iter().next().or_else(|| station_layouts(game).into_iter().next())
}

pub(super) fn station_accepts_output(game: &Game<'_>, stack: &horizon_items::items::ItemStack) -> bool {
    let Some(key) = game.container else { return false; };
    let Some(selected) = game.world.chunks.__horizon_mixins_tconstruct_part_selections().get(&key) else { return true; };
    let Some(layout) = station_layouts(game).into_iter().find(|layout| &layout.id == selected) else { return false; };
    if let Some(item) = layout.icon.get("item").and_then(serde_json::Value::as_str) { return item == stack.item.as_ref(); }
    if layout.tool_slot.is_some() || layout.icon.get("pattern").is_none() { return false; }
    let Some(entity) = game.container_kind() else { return false; };
    layout.input_slots.iter().enumerate().all(|(index, spec)| entity.slots().get(index).and_then(Option::as_ref).is_some_and(|stack| spec.accepts_item(stack.item.as_ref())))
}

/// Select the first adjacent container that fits the existing menu protocol.
pub(super) fn side_inventory(game: &Game<'_>, key: (i32, i32, i32), allow_double: bool) -> Option<(i32, i32, i32)> {
    [(0, -1), (1, 0), (0, 1), (-1, 0)].into_iter().find_map(|(dx, dz)| {
        let pos = (key.0 + dx, key.1, key.2 + dz);
        let name = &game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name;
        if matches!(&**name, "tconstruct:part_builder" | "tconstruct:tinker_station") || (!allow_double && game.chest_partner(pos).is_some()) { return None; }
        match game.world.block_entities.get(&pos) {
            Some(BlockEntity::Chest(items)) if matches!(items.len(), 5 | 9 | 27) => Some(pos),
            _ => None,
        }
    })
}
