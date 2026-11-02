use super::*;
pub(super) fn is_worktable(game: &Game<'_>) -> bool {
    if game.remote.is_some() {
        return game.remote_menu_title().is_some_and(|title| title.plain() == "TConstruct Modifier Worktable");
    }
    game.container.is_some_and(|pos| &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name == "tconstruct:modifier_worktable")
}

pub(super) fn kind(game: &Game<'_>) -> Option<bool> {
    if game.remote.is_some() {
        return match game.remote_menu_title().map(|t| t.plain()).as_deref() {
            Some("TConstruct Part Builder" | "TConstruct Part Builder Double") => Some(false),
            Some("TConstruct Tinker Station" | "TConstruct Tinkers Anvil" | "TConstruct Scorched Anvil" | "TConstruct Modifier Worktable") => Some(true),
            _ => None,
        };
    }
    let key = game.container?;
    match &*game.res.facts().get(game.world.block(key.0, key.1, key.2)).name {
        "tconstruct:part_builder" => Some(false),
        "tconstruct:tinker_station" | "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil" | "tconstruct:modifier_worktable" => Some(true),
        _ => None,
    }
}

pub(super) fn station_capacity(game: &Game<'_>) -> usize {
    if is_worktable(game) { return 2; }
    if game.remote.is_some() {
        return match game.remote_menu_title().map(|title| title.plain()).as_deref() {
            Some("TConstruct Tinkers Anvil" | "TConstruct Scorched Anvil") => 5,
            _ => 3,
        };
    }
    game.container.and_then(|pos| super::tconstruct_source_anvil_capacity::input_capacity(
        &game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name)).unwrap_or(3)
}

pub(super) fn side_count(game: &Game<'_>) -> usize {
    if game.remote.is_some() {
        game.container.and_then(|key| game.world.chunks.__horizon_mixins_tconstruct_builder_side_size().get(&key).copied()).unwrap_or(game.container_size().saturating_sub(27))
    } else { game.container_size().saturating_sub(27) }
}

fn display_storage_name(name: &str) -> String {
    serde_json::from_str::<serde_json::Value>(name).ok()
        .filter(|value| value.is_string() || value.is_object() || value.is_array())
        .map(|value| horizon_client::packs::component(&value).plain())
        .unwrap_or_else(|| name.to_owned())
}

pub(super) fn side_title(game: &Game<'_>) -> String {
    if let Some(pos) = game.container {
        if game.remote.is_some() {
            if let Some(name) = game.world.chunks.__horizon_mixins_tconstruct_builder_info().get(&pos).and_then(|info| info.get(6)).filter(|name| !name.is_empty()) {
                return display_storage_name(name);
            }
        } else {
            let side = game.world.chunks.__horizon_mixins_tconstruct_builder_side().get(&pos);
            let partner = game.world.chunks.__horizon_mixins_tconstruct_builder_double_partner().get(&pos);
            if let Some(name) = side.and_then(|side| game.world.block_names.get(side)).or_else(|| partner.and_then(|partner| game.world.block_names.get(partner))) {
                return display_storage_name(name);
            }
        }
    }
    let code = if game.remote.is_some() {
        game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_builder_side_title().get(&pos).copied()).unwrap_or(0)
    } else {
        game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_builder_side().get(&pos).copied())
            .map(|pos| super::tconstruct_source_side_layout::title_code(&game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name)).unwrap_or(0)
    };
    let key = super::tconstruct_source_side_layout::title_key(code);
    game.pack.names.display_get(key).unwrap_or_else(|| "Inventory".to_owned())
}

pub(super) fn station_layouts(game: &Game<'_>) -> Vec<super::tconstruct_source_station_layout::StationLayout> {
    station_layouts_for_capacity(game, station_capacity(game))
}

pub(super) fn station_layouts_for_capacity(game: &Game<'_>, capacity: usize) -> Vec<super::tconstruct_source_station_layout::StationLayout> {
    let Some(snapshot) = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref() else { return Vec::new(); };
    let Some(rest) = game.res.rest.get() else { return Vec::new(); };
    let mut layouts = Vec::new();
    for (id, layers) in snapshot {
        if !id.contains(":tinkering/station_layouts/") { continue; }
        let Some(layout) = layers.last().and_then(|bytes| super::tconstruct_source_station_layout::StationLayout::parse(id.clone(), bytes)) else { continue; };
        let item = layout.icon.get("item").and_then(serde_json::Value::as_str);
        let pattern = layout.icon.get("pattern").and_then(serde_json::Value::as_str);
        if (1..=capacity).contains(&layout.input_slots.len()) && layout.tool_slot.is_none()
            && (item.is_some_and(|item| rest.items.items.contains_key(item)) || pattern.is_some()) { layouts.push(layout); }
    }
    layouts.sort_by(|a, b| (a.sort_index, &a.id).cmp(&(b.sort_index, &b.id)));
    // The repair layout is a pattern button, rather than an item icon.
    let repair_id = if capacity == 5 { "tconstruct:tinkering/station_layouts/tinkers_anvil" } else { "tconstruct:tinkering/station_layouts/tinker_station" };
    if let Some(layout) = snapshot.get(repair_id)
        .and_then(|layers| layers.last())
        .and_then(|bytes| super::tconstruct_source_station_layout::StationLayout::parse(repair_id.into(), bytes)) {
        layouts.insert(0, layout);
    }
    layouts
}

/// Prefer this player's selected source layout, then infer it from inputs.
pub(super) fn station_layout(game: &Game<'_>) -> Option<super::tconstruct_source_station_layout::StationLayout> {
    let BlockEntity::Chest(slots) = game.container_kind()? else { return None; };
    if super::tconstruct_source_station_world::has_operation_tool(slots) {
        let snapshot = game.world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
        let id = if station_capacity(game) == 5 { "tconstruct:tinkering/station_layouts/tinkers_anvil" } else { "tconstruct:tinkering/station_layouts/tinker_station" };
        let bytes = snapshot.get(id)?.last()?;
        return super::tconstruct_source_station_layout::StationLayout::parse(id.into(), bytes);
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

/// Original source chests need component-aware slot rules and paging even
/// when displayed alongside a Part Builder.
pub(super) fn side_chest_kind(game: &Game<'_>) -> Option<super::tconstruct_chest_rules::ChestKind> {
    use super::tconstruct_chest_rules::ChestKind;
    if kind(game) != Some(false) { return None; }
    let table=game.container?;
    if game.remote.is_some() {
        return match game.world.chunks.__horizon_mixins_tconstruct_builder_side_title().get(&table).copied()? {
            7=>Some(ChestKind::Part),8=>Some(ChestKind::Tinkers),_=>None,
        };
    }
    let side=*game.world.chunks.__horizon_mixins_tconstruct_builder_side().get(&table)?;
    match &*game.res.facts().get(game.world.block(side.0,side.1,side.2)).name {
        "tconstruct:part_chest"=>Some(ChestKind::Part),"tconstruct:tinkers_chest"=>Some(ChestKind::Tinkers),_=>None,
    }
}

pub(super) fn set_side_scroll(game: &mut Game<'_>, offset: usize) {
    if game.remote.is_some() && side_chest_kind(game).is_some() {
        if let Some((window,_))=game.remote.as_ref().and_then(|r|r.menu.as_ref()) {
            game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick { container:*window,button:42000+offset as i32 }));
        }
    } else if let Some(pos)=game.container {
        game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll_mut().entry(pos).or_default().0=offset;
    }
}

/// Select the first adjacent container that fits the existing menu protocol.
pub(super) fn side_inventory(game: &Game<'_>, key: (i32, i32, i32), allow_double: bool) -> Option<(i32, i32, i32)> {
    [(0, -1), (1, 0), (0, 1), (-1, 0)].into_iter().find_map(|(dx, dz)| {
        let pos = (key.0 + dx, key.1, key.2 + dz);
        let name = &game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name;
        if matches!(&**name, "tconstruct:part_builder" | "tconstruct:tinker_station" | "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil") || (!allow_double && game.chest_partner(pos).is_some()) { return None; }
        match game.world.block_entities.get(&pos) {
            Some(BlockEntity::Chest(items)) if matches!((&**name,items.len()), ("tconstruct:part_chest",256) | ("tconstruct:tinkers_chest",64)) => Some(pos),
            Some(BlockEntity::Chest(items)) if matches!(items.len(), 5 | 9 | 27) => Some(pos),
            _ => None,
        }
    })
}

/// Identify the Melter menu without changing other container screens.
pub(super) fn is_melter(game: &Game<'_>) -> bool {
    if game.remote.is_some() {
        return game.remote_menu_title().map(|title| title.plain()).as_deref() == Some("TConstruct Melter");
    }
    game.container.is_some_and(|pos| &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name == "tconstruct:seared_melter")
}
