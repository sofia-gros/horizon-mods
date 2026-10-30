use super::*;
pub(super) fn kind(game: &Game<'_>) -> Option<bool> {
    if game.remote.is_some() {
        return match game.remote_menu_title().map(|t| t.plain()).as_deref() {
            Some("TConstruct Part Builder") => Some(false),
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

/// Resolve a source layout from the ordered items visible in this menu.
/// Selection controls and synchronization of an explicitly selected layout are
/// separate from this input-driven fallback.
pub(super) fn station_layout(game: &Game<'_>) -> Option<super::tconstruct_source_station_layout::StationLayout> {
    let BlockEntity::Chest(slots) = game.container_kind()? else { return None; };
    let snapshot = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let mut candidates = Vec::new();
    for (id, layers) in snapshot {
        if !id.contains(":tinkering/station_layouts/") { continue; }
        let Some(layout) = layers.last().and_then(|bytes| super::tconstruct_source_station_layout::StationLayout::parse(id.clone(), bytes)) else { continue; };
        if layout.input_slots.len() != 3 { continue; }
        let mut present = false;
        let matches = layout.input_slots.iter().zip(slots.iter()).all(|(spec, stack)| {
            let Some(stack) = stack else { return true; };
            present = true;
            spec.filter.as_ref().and_then(|filter| filter.get("item")).and_then(serde_json::Value::as_str) == Some(stack.item.as_ref())
        });
        if present && matches { candidates.push(layout); }
    }
    candidates.sort_by(|a, b| (a.sort_index, &a.id).cmp(&(b.sort_index, &b.id)));
    candidates.into_iter().next()
}

/// Select the first adjacent container that fits the existing menu protocol.
pub(super) fn side_inventory(game: &Game<'_>, key: (i32, i32, i32)) -> Option<(i32, i32, i32)> {
    [(0, -1), (1, 0), (0, 1), (-1, 0)].into_iter().find_map(|(dx, dz)| {
        let pos = (key.0 + dx, key.1, key.2 + dz);
        let name = &game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name;
        if matches!(&**name, "tconstruct:part_builder" | "tconstruct:tinker_station") || game.chest_partner(pos).is_some() { return None; }
        match game.world.block_entities.get(&pos) {
            Some(BlockEntity::Chest(items)) if matches!(items.len(), 5 | 9 | 27) => Some(pos),
            _ => None,
        }
    })
}
