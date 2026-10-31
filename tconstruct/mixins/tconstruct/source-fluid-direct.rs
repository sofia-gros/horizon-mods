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
