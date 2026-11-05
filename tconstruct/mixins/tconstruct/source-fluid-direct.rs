// Source controller tank clicks transfer the carried container, or select a
// fluid when the cursor is empty. Both variants stage tank changes first.
use super::{Game, lan::LanHost};
use super::tconstruct_source_fluid_container::{self as containers, Direction};

type Pos = (i32, i32, i32);

pub(super) fn local(game: &mut Game<'_>, pos: Pos, selected: Option<usize>) {
    let Some(raw) = super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery") else { return; };
    let Some(mut state) = super::tconstruct_source_smeltery_state::Smeltery::from_nbt(&raw) else { return; };
    if state.tank_to_nbt_preserving(&raw).as_ref() != Some(&raw) { return; }
    let Some(carried) = game.carried.as_ref() else { return; };
    if carried.wire_patch().is_some() || carried.data().is_some_and(|data| data.wire.patch().is_some()) { return; }
    let mut slots = [Some(carried.clone()), None];
    let known_fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
        .and_then(|v| v.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()).map(|all| all.iter().filter(|(id, _)| matches!(id.as_str(), "forge:bottles/splash" | "forge:bottles/lingering")).map(|(id, items)| (id.clone(), items.clone())).collect());
    let transferred = if let Some(index) = selected {
        containers::fill_from_index_with_color(&mut state.tank, &mut slots, index,
            &|id| game.res.items().get(id).is_some(), &|id| known_fluids.contains(id), tags.as_ref(), game.lan.is_none())
    } else {
        containers::transfer_selected_with_color(&mut state.tank, &mut slots, Direction::EmptyItem, None,
            &|id| game.res.items().get(id).is_some(), &|id| known_fluids.contains(id), tags.as_ref(), game.lan.is_none())
    };
    if !transferred { return; }
    let Some(saved) = state.tank_to_nbt_preserving(&raw) else { return; };
    if super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery").as_ref() != Some(&raw) { return; }
    if !super::tconstruct_source_melter_store::write(&mut game.world, pos, "tconstruct:smeltery", saved) { return; }
    if let Some(rest) = slots[0].take() {
        game.carried = Some(rest);
        if let Some(result) = slots[1].take() { game.give_or_drop(result); }
    } else { game.carried = slots[1].take(); }
}

pub(super) fn guest(game: &mut Game<'_>, host: &mut LanHost, gi: usize, pos: Pos, selected: Option<usize>) {
    let Some(raw) = super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery") else { return; };
    let Some(mut state) = super::tconstruct_source_smeltery_state::Smeltery::from_nbt(&raw) else { return; };
    if state.tank_to_nbt_preserving(&raw).as_ref() != Some(&raw) { return; }
    let Some(carried) = host.hub.guests.get(gi).and_then(|guest| guest.carried.as_ref()) else { return; };
    if carried.wire_patch().is_some() || carried.data().is_some_and(|data| data.wire.patch().is_some()) { return; }
    let mut slots = [Some(carried.clone()), None];
    let known_fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
        .and_then(|v| v.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()).map(|all| all.iter().filter(|(id, _)| matches!(id.as_str(), "forge:bottles/splash" | "forge:bottles/lingering")).map(|(id, items)| (id.clone(), items.clone())).collect());
    let transferred = if let Some(index) = selected {
        containers::fill_from_index_with_color(&mut state.tank, &mut slots, index,
            &|id| game.res.items().get(id).is_some(), &|id| known_fluids.contains(id), tags.as_ref(), false)
    } else {
        containers::transfer_selected_with_color(&mut state.tank, &mut slots, Direction::EmptyItem, None,
            &|id| game.res.items().get(id).is_some(), &|id| known_fluids.contains(id), tags.as_ref(), false)
    };
    if !transferred { return; }
    let Some(saved) = state.tank_to_nbt_preserving(&raw) else { return; };
    if super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery").as_ref() != Some(&raw) { return; }
    if !super::tconstruct_source_melter_store::write(&mut game.world, pos, "tconstruct:smeltery", saved) { return; }
    if let Some(rest) = slots[0].take() {
        host.hub.guests[gi].carried = Some(rest);
        if let Some(result) = slots[1].take() { game.lan_place_back(host, gi, vec![result]); host.hub.send_inventory(gi); }
    } else { host.hub.guests[gi].carried = slots[1].take(); }
    let carried_stack = host.hub.guests[gi].carried.clone();
    let carried = host.hub.stack(&carried_stack);
    host.hub.guests[gi].send(&horizon_protocol::packets::play::Clientbound::SetCursorItem(
        horizon_protocol::packets::play::SetCursorItem { item: carried }));
}

/// Stage and commit a held-bucket transfer for the Melter output or the
/// registered fluid tank immediately below it.
fn melter_exchange(game: &mut Game<'_>, pos: Pos, carried: &Option<horizon_items::items::ItemStack>, fuel: bool, right: bool, allow_color: bool)
    -> Option<[Option<horizon_items::items::ItemStack>; 2]> {
    use super::tconstruct_source_melter_state::Melter;
    let raw_melter = super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:melter")?;
    let melter = Melter::from_nbt(&raw_melter)?;
    let (target, kind, raw, capacity) = if fuel {
        let below = (pos.0, pos.1.checked_sub(1)?, pos.2);
        let name = game.res.facts().get(game.world.block(below.0, below.1, below.2)).name.clone();
        let capacity = super::tconstruct_source_smeltery_fuel::capacity(&name)?;
        let kind = super::tconstruct_source_machine_runtime::kind(&name)?;
        if !super::tconstruct_source_machine_runtime::ensure(game, below, &name) { return None; }
        (below, kind, super::tconstruct_source_melter_store::read(&game.world, below, kind)?, capacity)
    } else {
        for alias in ["Tank", "Fluid"] { if raw_melter.get(alias).is_some() { return None; } }
        (pos, "tconstruct:melter", raw_melter.clone(), melter.tank.capacity)
    };
    let block = game.world.block(target.0,target.1,target.2);
    let raw_tank = if fuel { &raw } else { raw.get("tank")? };
    let fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
        .and_then(|value| value.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
    let direction = if right { Direction::EmptyItem } else { Direction::FillItem };
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()).map(|all| all.iter().filter(|(id, _)| matches!(id.as_str(), "forge:bottles/splash" | "forge:bottles/lingering")).map(|(id, items)| (id.clone(), items.clone())).collect());
    let (tank, slots) = containers::plan_single_tank_container(raw_tank, capacity, carried, direction,
        &|item| game.res.items().get(item).is_some(), &|fluid| fluids.contains(fluid), tags.as_ref(), allow_color)?;
    let saved = if fuel { tank } else {
        let horizon_world::nbt::Tag::Compound(mut fields) = raw.clone() else { return None; };
        fields.insert("tank".into(), tank); horizon_world::nbt::Tag::Compound(fields)
    };
    if !game.world.chunks.contains_key(&(target.0 >> 4,target.2 >> 4))
        || game.world.block(target.0,target.1,target.2) != block
        || super::tconstruct_source_melter_store::read(&game.world,target,kind).as_ref() != Some(&raw) { return None; }
    super::tconstruct_source_melter_store::write(&mut game.world, target, kind, saved).then_some(slots)
}

pub(super) fn melter_local(game: &mut Game<'_>, pos: Pos, fuel: bool, right: bool) {
    let carried = game.carried.clone();
    let allow_color = game.lan.is_none();
    let Some(mut slots) = melter_exchange(game, pos, &carried, fuel, right, allow_color) else { return; };
    if let Some(rest) = slots[0].take() {
        game.carried = Some(rest);
        if let Some(result) = slots[1].take() { game.give_or_drop(result); }
    } else { game.carried = slots[1].take(); }
}
pub(super) fn melter_guest(game: &mut Game<'_>, host: &mut LanHost, gi: usize, pos: Pos, fuel: bool, right: bool) {
    let carried = host.hub.guests[gi].carried.clone();
    let Some(mut slots) = melter_exchange(game, pos, &carried, fuel, right, false) else { return; };
    if let Some(rest) = slots[0].take() {
        host.hub.guests[gi].carried = Some(rest);
        if let Some(result) = slots[1].take() { game.lan_place_back(host, gi, vec![result]); host.hub.send_inventory(gi); }
    } else { host.hub.guests[gi].carried = slots[1].take(); }
    let carried = host.hub.guests[gi].carried.clone();
    let item = host.hub.stack(&carried);
    host.hub.guests[gi].send(&horizon_protocol::packets::play::Clientbound::SetCursorItem(horizon_protocol::packets::play::SetCursorItem { item }));
}

/// Exchange a held container with a fuel tank belonging to this structure.
/// Try the displayed tank first, then other owned tanks; each attempt is
/// staged before its metadata or the carried item is changed.
fn fuel_exchange(game: &mut Game<'_>, pos: Pos, carried: &Option<horizon_items::items::ItemStack>, right: bool, allow_color: bool)
    -> Option<[Option<horizon_items::items::ItemStack>; 2]> {
    let state = super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
        .and_then(|tag| super::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag))?;
    let mut positions: Vec<_> = game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners().iter()
        .filter_map(|(at, owner)| (*owner == pos).then_some(*at)).collect();
    positions.sort_unstable();
    if let Some(first) = state.fuel_view.last.and_then(|at| positions.iter().position(|candidate| *candidate == at)) {
        let at = positions.remove(first); positions.insert(0, at);
    }
    let fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
        .and_then(|value| value.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
    for at in positions {
        let name = game.res.facts().get(game.world.block(at.0, at.1, at.2)).name.clone();
        let Some(capacity) = super::tconstruct_source_smeltery_fuel::capacity(&name) else { continue; };
        let Some(kind) = super::tconstruct_source_machine_runtime::kind(&name) else { continue; };
        if !super::tconstruct_source_machine_runtime::ensure(game, at, &name) { continue; }
        let Some(raw) = super::tconstruct_source_melter_store::read(&game.world, at, kind) else { continue; };
        let block = game.world.block(at.0,at.1,at.2);
        let direction = if right { Direction::EmptyItem } else { Direction::FillItem };
        let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok()).map(|all| all.iter().filter(|(id, _)| matches!(id.as_str(), "forge:bottles/splash" | "forge:bottles/lingering")).map(|(id, items)| (id.clone(), items.clone())).collect());
        let Some((saved, slots)) = containers::plan_single_tank_container(&raw, capacity, carried, direction,
            &|item| game.res.items().get(item).is_some(), &|fluid| fluids.contains(fluid), tags.as_ref(), allow_color) else { continue; };
        if !game.world.chunks.contains_key(&(at.0 >> 4,at.2 >> 4))
            || game.world.block(at.0,at.1,at.2) != block
            || game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners().get(&at) != Some(&pos)
            || super::tconstruct_source_melter_store::read(&game.world,at,kind).as_ref() != Some(&raw) { return None; }
        if !super::tconstruct_source_melter_store::write(&mut game.world, at, kind, saved) { return None; }
        return Some(slots);
    }
    None
}

pub(super) fn fuel_local(game: &mut Game<'_>, pos: Pos, right: bool) {
    let carried = game.carried.clone();
    let allow_color = game.lan.is_none();
    let Some(mut slots) = fuel_exchange(game, pos, &carried, right, allow_color) else { return; };
    if let Some(rest) = slots[0].take() {
        game.carried = Some(rest);
        if let Some(result) = slots[1].take() { game.give_or_drop(result); }
    } else { game.carried = slots[1].take(); }
}

pub(super) fn fuel_guest(game: &mut Game<'_>, host: &mut LanHost, gi: usize, pos: Pos, right: bool) {
    let carried = host.hub.guests[gi].carried.clone();
    let Some(mut slots) = fuel_exchange(game, pos, &carried, right, false) else { return; };
    if let Some(rest) = slots[0].take() {
        host.hub.guests[gi].carried = Some(rest);
        if let Some(result) = slots[1].take() { game.lan_place_back(host, gi, vec![result]); host.hub.send_inventory(gi); }
    } else { host.hub.guests[gi].carried = slots[1].take(); }
    let carried = host.hub.guests[gi].carried.clone();
    let item = host.hub.stack(&carried);
    host.hub.guests[gi].send(&horizon_protocol::packets::play::Clientbound::SetCursorItem(
        horizon_protocol::packets::play::SetCursorItem { item }));
}
