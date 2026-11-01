// Source controller tank clicks transfer the carried container, or select a
// fluid when the cursor is empty. Both variants stage tank changes first.
use super::{Game, lan::LanHost};
use super::tconstruct_source_fluid_container::{self as containers, Direction};

type Pos = (i32, i32, i32);

pub(super) fn local(game: &mut Game<'_>, pos: Pos, selected: Option<usize>) {
    let Some(mut state) = super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
        .and_then(|tag| super::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) else { return; };
    let fluid = selected.and_then(|index| state.tank.fluids.get(index).map(|f| f.id.clone()));
    if selected.is_some() && fluid.is_none() { return; }
    let mut slots = [game.carried.clone(), None];
    let known_fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
        .and_then(|v| v.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
    let direction = if selected.is_some() { Direction::FillItem } else { Direction::EmptyItem };
    if !containers::transfer_selected(&mut state.tank, &mut slots, direction, fluid.as_deref(),
        &|id| game.res.items().get(id).is_some(), &|id| known_fluids.contains(id)) { return; }
    if !super::tconstruct_source_melter_store::write(&mut game.world, pos, "tconstruct:smeltery", state.to_nbt()) { return; }
    if let Some(rest) = slots[0].take() {
        game.carried = Some(rest);
        if let Some(result) = slots[1].take() { game.give_or_drop(result); }
    } else { game.carried = slots[1].take(); }
}

pub(super) fn guest(game: &mut Game<'_>, host: &mut LanHost, gi: usize, pos: Pos, selected: Option<usize>) {
    let Some(mut state) = super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
        .and_then(|tag| super::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) else { return; };
    let fluid = selected.and_then(|index| state.tank.fluids.get(index).map(|f| f.id.clone()));
    if selected.is_some() && fluid.is_none() { return; }
    let mut slots = [host.hub.guests[gi].carried.clone(), None];
    let known_fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
        .and_then(|v| v.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
    let direction = if selected.is_some() { Direction::FillItem } else { Direction::EmptyItem };
    if !containers::transfer_selected(&mut state.tank, &mut slots, direction, fluid.as_deref(),
        &|id| game.res.items().get(id).is_some(), &|id| known_fluids.contains(id)) { return; }
    if !super::tconstruct_source_melter_store::write(&mut game.world, pos, "tconstruct:smeltery", state.to_nbt()) { return; }
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
fn melter_exchange(game: &mut Game<'_>, pos: Pos, carried: &Option<horizon_items::items::ItemStack>, fuel: bool, right: bool)
    -> Option<[Option<horizon_items::items::ItemStack>; 2]> {
    use super::tconstruct_source_melter_state::{Melter, Tank};
    let mut melter = Melter::from_nbt(&super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:melter")?)?;
    let below = (pos.0, pos.1.checked_sub(1)?, pos.2);
    let name = game.res.facts().get(game.world.block(below.0, below.1, below.2)).name.clone();
    let (target, kind, mut tank) = if fuel {
        let capacity = super::tconstruct_source_smeltery_fuel::capacity(&name)?;
        let kind = super::tconstruct_source_machine_runtime::kind(&name)?;
        if !super::tconstruct_source_machine_runtime::ensure(game, below, &name) { return None; }
        let tank = Tank::from_nbt(&super::tconstruct_source_melter_store::read(&game.world, below, kind)?, capacity)?;
        (below, kind, tank)
    } else { (pos, "tconstruct:melter", melter.tank.clone()) };
    let mut slots = [carried.clone(), None];
    let fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
        .and_then(|value| value.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
    let direction = if right { Direction::EmptyItem } else { Direction::FillItem };
    if !containers::transfer_single(&mut tank, &mut slots, direction,
        &|item| game.res.items().get(item).is_some(), &|fluid| fluids.contains(fluid)) { return None; }
    let tag = if fuel { tank.to_nbt() } else { melter.tank = tank; melter.to_nbt() };
    super::tconstruct_source_melter_store::write(&mut game.world, target, kind, tag).then_some(slots)
}

pub(super) fn melter_local(game: &mut Game<'_>, pos: Pos, fuel: bool, right: bool) {
    let carried = game.carried.clone();
    let Some(mut slots) = melter_exchange(game, pos, &carried, fuel, right) else { return; };
    if let Some(rest) = slots[0].take() {
        game.carried = Some(rest);
        if let Some(result) = slots[1].take() { game.give_or_drop(result); }
    } else { game.carried = slots[1].take(); }
}
pub(super) fn melter_guest(game: &mut Game<'_>, host: &mut LanHost, gi: usize, pos: Pos, fuel: bool, right: bool) {
    let carried = host.hub.guests[gi].carried.clone();
    let Some(mut slots) = melter_exchange(game, pos, &carried, fuel, right) else { return; };
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
fn fuel_exchange(game: &mut Game<'_>, pos: Pos, carried: &Option<horizon_items::items::ItemStack>, right: bool)
    -> Option<[Option<horizon_items::items::ItemStack>; 2]> {
    use super::tconstruct_source_melter_state::Tank;
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
        let Some(mut tank) = super::tconstruct_source_melter_store::read(&game.world, at, kind)
            .and_then(|tag| Tank::from_nbt(&tag, capacity)) else { continue; };
        let mut slots = [carried.clone(), None];
        let direction = if right { Direction::EmptyItem } else { Direction::FillItem };
        if !containers::transfer_single(&mut tank, &mut slots, direction,
            &|item| game.res.items().get(item).is_some(), &|fluid| fluids.contains(fluid)) { continue; }
        if !super::tconstruct_source_melter_store::write(&mut game.world, at, kind, tank.to_nbt()) { return None; }
        return Some(slots);
    }
    None
}

pub(super) fn fuel_local(game: &mut Game<'_>, pos: Pos, right: bool) {
    let carried = game.carried.clone();
    let Some(mut slots) = fuel_exchange(game, pos, &carried, right) else { return; };
    if let Some(rest) = slots[0].take() {
        game.carried = Some(rest);
        if let Some(result) = slots[1].take() { game.give_or_drop(result); }
    } else { game.carried = slots[1].take(); }
}

pub(super) fn fuel_guest(game: &mut Game<'_>, host: &mut LanHost, gi: usize, pos: Pos, right: bool) {
    let carried = host.hub.guests[gi].carried.clone();
    let Some(mut slots) = fuel_exchange(game, pos, &carried, right) else { return; };
    if let Some(rest) = slots[0].take() {
        host.hub.guests[gi].carried = Some(rest);
        if let Some(result) = slots[1].take() { game.lan_place_back(host, gi, vec![result]); host.hub.send_inventory(gi); }
    } else { host.hub.guests[gi].carried = slots[1].take(); }
    let carried = host.hub.guests[gi].carried.clone();
    let item = host.hub.stack(&carried);
    host.hub.guests[gi].send(&horizon_protocol::packets::play::Clientbound::SetCursorItem(
        horizon_protocol::packets::play::SetCursorItem { item }));
}
