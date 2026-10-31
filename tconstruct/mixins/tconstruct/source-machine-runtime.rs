// Authoritative machine state uses normal live inventories plus custom NBT
// fields preserved by the source-machine save hooks.
use horizon_client::containers::BlockEntity;
use horizon_world::nbt::Tag;
use super::{tconstruct_source_melter_state::{Casting, Faucet, Melter, Tank}, tconstruct_source_melter_store as store};
use super::tconstruct_source_smeltery_state::Smeltery;

type Pos = (i32, i32, i32);

fn smeltery_fluids(tank: &super::tconstruct_source_smeltery_tank::SmelteryTank) -> Vec<(String, u32)> {
    tank.fluids.iter().map(|fluid| (fluid.id.clone(), fluid.amount)).collect()
}

pub(super) fn kind(block: &str) -> Option<&'static str> {
    match block {
        "tconstruct:seared_melter" => Some("tconstruct:melter"),
        "tconstruct:seared_heater" => Some("tconstruct:heater"),
        "tconstruct:seared_faucet" => Some("tconstruct:faucet"),
        "tconstruct:seared_table" => Some("tconstruct:table"),
        "tconstruct:seared_basin" => Some("tconstruct:basin"),
        "tconstruct:smeltery_controller" | "tconstruct:foundry_controller" => Some("tconstruct:smeltery"),
        "tconstruct:seared_fuel_tank" | "tconstruct:seared_fuel_gauge" | "tconstruct:scorched_fuel_tank" | "tconstruct:scorched_fuel_gauge" => Some("tconstruct:fuel_tank"),
        "tconstruct:seared_ingot_tank" | "tconstruct:seared_ingot_gauge" | "tconstruct:scorched_ingot_tank" | "tconstruct:scorched_ingot_gauge" => Some("tconstruct:ingot_tank"),
        _ => None,
    }
}

fn initial(kind: &str) -> Option<Tag> {
    Some(match kind {
        "tconstruct:melter" => Melter::new().to_nbt(),
        "tconstruct:heater" => Tag::compound(Vec::<(&str, Tag)>::new()),
        "tconstruct:faucet" => Faucet::new().to_nbt(),
        "tconstruct:table" | "tconstruct:basin" => Casting::new().to_nbt(),
        "tconstruct:smeltery" => Smeltery::new().to_nbt(),
        "tconstruct:fuel_tank" => Tank::new(4000).to_nbt(),
        "tconstruct:ingot_tank" => Tank::new(4320).to_nbt(),
        _ => return None,
    })
}

/// Update the current candidate and capacity from loaded world blocks. Items
/// displaced by shrinking are spilled only after the state update commits.
pub(super) fn refresh_smeltery(game: &mut super::Game<'_>, pos: Pos) {
    let bounds = super::tconstruct_source_smeltery_world::detect(game, pos);
    super::tconstruct_source_smeltery_world::claim(game, pos, bounds);
    let Some(saved) = store::read(&game.world, pos, "tconstruct:smeltery") else { return; };
    let Some(mut state) = Smeltery::from_nbt(&saved) else { return; };
    let Some((min, max, capacity)) = bounds else {
        state.tick_invalid_structure();
        let _ = store::write(&mut game.world, pos, "tconstruct:smeltery", state.to_nbt());
        return;
    };
    let foundry = &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name == "tconstruct:foundry_controller";
    let dx = (max[0] - min[0] - 1) as u32; let dy = (max[1] - min[1]) as u32; let dz = (max[2] - min[2] - 1) as u32;
    let volume = (dx * dy * dz) as usize;
    state.fuel_cost = if foundry { 1 + (2 * (dx + 2) * dy + 2 * dy * dz + (dx + 2) * (dz + 2)) / 18 }
        else { 1 + (2 * dx * dy + 2 * dy * dz + dx * dz) / 15 };
    let displaced = if state.slots.len() != volume || state.tank.capacity != capacity {
        state.resize(volume, capacity)
    } else { Vec::new() };
    if !matches!(game.world.block_entities.get(&pos), Some(BlockEntity::Chest(_))) { return; }
    let recipes: Vec<_> = if state.phase == 0 || state.phase == 1 {
        state.slots.iter().map(|slot| slot.as_ref().and_then(|stack| {
            if foundry {
                super::tconstruct_source_melter_world::foundry_melting(&game.world, &stack.item)
                    .map(|(id, recipe, fluid, byproducts)| (id, recipe.melting, fluid, byproducts))
            } else {
                super::tconstruct_source_melter_world::melting(&game.world, &stack.item)
                    .map(|(id, recipe, fluid)| (id, recipe, fluid, Vec::new()))
            }
        })).collect()
    } else { vec![None; state.slots.len()] };
    let mut fuel_update = None;
    if state.phase == 0 && state.fuel == 0 {
        let mut tanks: Vec<_> = game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners().iter()
            .filter_map(|(at, owner)| (*owner == pos).then_some(*at)).collect();
        tanks.sort_unstable();
        for at in tanks {
            let name = game.res.facts().get(game.world.block(at.0, at.1, at.2)).name.clone();
            let Some(tank_capacity) = super::tconstruct_source_smeltery_fuel::capacity(&name) else { continue; };
            if !ensure(game, at, &name) { continue; }
            let Some(kind) = kind(&name) else { continue; };
            let Some(mut tank) = store::read(&game.world, at, kind).and_then(|tag| Tank::from_nbt(&tag, tank_capacity)) else { continue; };
            let Some(fluid) = tank.fluid.as_deref() else { continue; };
            let Some(fuel) = super::tconstruct_source_melter_world::fluid_fuel(&game.world, fluid, tank.amount) else { continue; };
            let can_heat = recipes.iter().enumerate().any(|(slot, recipe)| recipe.as_ref().is_some_and(|(_, recipe, _, _)|
                state.can_heat_slot(slot, fuel.temperature, recipe.temperature)));
            let can_alloy = !foundry && super::tconstruct_source_melter_world::alloy(&game.world, &smeltery_fluids(&state.tank), state.tank.capacity, fuel.temperature)
                .is_some_and(|(_, inputs, output, amount)| {
                    let mut candidate = state.tank.clone();
                    let inputs = inputs.into_iter().map(|(id, amount)| super::tconstruct_source_smeltery_tank::FluidAmount { id, amount }).collect::<Vec<_>>();
                    candidate.alloy(&inputs, &super::tconstruct_source_smeltery_tank::FluidAmount { id: output, amount })
                });
            if !can_heat && !can_alloy { continue; }
            if tank.take(fuel.fluid.amount).is_none() { continue; }
            state.fuel = fuel.duration; state.fuel_quality = fuel.duration; state.temperature = fuel.temperature; state.rate = fuel.rate;
            fuel_update = Some((at, kind, tank.to_nbt()));
            break;
        }
    }
    if state.phase == 1 {
        for (index, recipe) in recipes.iter().enumerate() {
            if let Some((_, recipe, fluid, byproducts)) = recipe {
                if state.fuel > 0 || state.progress[index] == -1 {
                    if let Some(time) = recipe.time.checked_mul(10) {
                        if foundry {
                            let primary = super::tconstruct_source_smeltery_tank::FluidAmount { id: fluid.clone(), amount: recipe.output.amount };
                            let extras = byproducts.iter().map(|(id, amount)| super::tconstruct_source_smeltery_tank::FluidAmount { id: id.clone(), amount: *amount }).collect::<Vec<_>>();
                            state.heat_foundry_slot(index, recipe.temperature, time, &primary, &extras);
                        } else { state.heat_slot(index, recipe.temperature, time, fluid, recipe.output.amount); }
                    }
                } else { state.cool_slot(index); }
            } else { state.progress[index] = 0; }
        }
    }
    if !foundry && state.phase == 2 && state.fuel > 0 {
        if let Some((_, inputs, output, amount)) = super::tconstruct_source_melter_world::alloy(&game.world, &smeltery_fluids(&state.tank), state.tank.capacity, state.temperature) {
            let inputs = inputs.into_iter().map(|(id, amount)| super::tconstruct_source_smeltery_tank::FluidAmount { id, amount }).collect::<Vec<_>>();
            state.tank.alloy(&inputs, &super::tconstruct_source_smeltery_tank::FluidAmount { id: output, amount });
        }
    }
    if state.phase == 3 && state.fuel > 0 {
        state.fuel = state.fuel.saturating_sub(state.fuel_cost);
    }
    state.phase = (state.phase + 1) % 4;
    if state.phase == 1 {
        let mut positions: Vec<_> = game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners().iter()
            .filter_map(|(at, owner)| (*owner == pos).then_some(*at)).collect();
        positions.sort_unstable();
        let tanks: Vec<_> = positions.into_iter().filter_map(|at| {
            let name = game.res.facts().get(game.world.block(at.0, at.1, at.2)).name.clone();
            let capacity = super::tconstruct_source_smeltery_fuel::capacity(&name)?;
            let kind = kind(&name)?;
            let staged = fuel_update.as_ref().filter(|(updated, _, _)| *updated == at).map(|(_, _, tag)| tag.clone());
            let tank = staged.or_else(|| store::read(&game.world, at, kind))
                .and_then(|tag| Tank::from_nbt(&tag, capacity)).unwrap_or_else(|| Tank::new(capacity));
            Some((at, tank))
        }).collect();
        let last = fuel_update.as_ref().map(|(at, _, _)| *at).or(state.fuel_view.last);
        state.fuel_view = super::tconstruct_source_smeltery_fuel::FuelView::collect(&tanks, last,
            |fluid| super::tconstruct_source_melter_world::fluid_fuel(&game.world, fluid, u32::MAX).map_or(0, |fuel| fuel.temperature));
    }
    let mut updates = vec![(pos, "tconstruct:smeltery", state.to_nbt())];
    if let Some(update) = fuel_update { updates.push(update); }
    if !store::write_many(&mut game.world, &updates) { return; }
    super::spill::tconstruct_source_smeltery_spill::spill(game, pos, displaced);
    if game.container == Some(pos) && game.remote.is_none() {
        let offset = game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages().get(&pos).map_or(0, |page| page.1);
        let containers = game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers().get(&pos)
            .map(|(slots, _)| slots.clone()).unwrap_or([None, None]);
        if let Some(BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&pos) {
            *live = super::tconstruct_source_smeltery_page::project_with_containers(&state.slots, offset, &containers);
        }
    }
}

/// Exchange a standard lava bucket with a source tank, staging the NBT before
/// changing the held stack. Other fluid containers are not handled here yet.
pub(super) fn interact_fuel_tank(game: &mut super::Game<'_>, pos: Pos, block: &str) -> bool {
    let Some(capacity) = super::tconstruct_source_smeltery_fuel::capacity(block) else { return false; };
    let Some(kind) = kind(block) else { return false; };
    if !ensure(game, pos, block) { return false; }
    let Some(mut tank) = store::read(&game.world, pos, kind).and_then(|tag| Tank::from_nbt(&tag, capacity)) else { return false; };
    let Some(mut held) = game.held().cloned() else { return true; };
    if held.count == 0 { return false; }
    let Some(replacement) = super::tconstruct_source_smeltery_fuel::exchange(&mut tank, &held.item) else { return true; };
    if !store::write(&mut game.world, pos, kind, tank.to_nbt()) { return false; }
    if game.creative && replacement == "minecraft:lava_bucket" {
        // The held empty bucket remains in creative; do not silently discard
        // the fluid that was removed from the tank.
        game.give_or_drop(horizon_items::items::ItemStack::new(replacement, 1));
    } else if !game.creative {
        held.count -= 1;
        if held.count == 0 {
            *game.held_mut() = Some(horizon_items::items::ItemStack::new(replacement, 1));
        } else {
            *game.held_mut() = Some(held);
            game.give_or_drop(horizon_items::items::ItemStack::new(replacement, 1));
        }
    }
    true
}

pub(super) fn ensure(game: &mut super::Game<'_>, pos: Pos, block: &str) -> bool {
    let Some(kind) = kind(block) else { return false; };
    if !game.world.chunks.contains_key(&(pos.0 >> 4, pos.2 >> 4)) { return false; }
    if game.world.block_entities.get(&pos).is_some_and(|entity| !matches!(entity, BlockEntity::Chest(_))) { return false; }
    if game.world.block_entities.get(&pos).is_none() { store::clear_different(&mut game.world, pos, kind); }
    if store::read(&game.world, pos, kind).is_none() {
        let Some(data) = initial(kind) else { return false; };
        if !store::write(&mut game.world, pos, kind, data) { return false; }
    }
    game.world.block_entities.get_or_insert_with(pos, || BlockEntity::Chest(vec![None; 27]));
    game.world.modified.insert((pos.0 >> 4, pos.2 >> 4));
    true
}

fn same_material(a: &Option<horizon_items::items::ItemStack>, b: &Option<horizon_items::items::ItemStack>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.item == b.item && a.damage == b.damage && a.extra == b.extra,
        _ => false,
    }
}

pub(super) fn solid_fuel_rule(game: &super::Game<'_>) -> Option<(u32, u32)> {
    let recipes = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let bytes = recipes.get("tconstruct:smeltery/melting/fuel/solid")?;
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    if value.get("type")?.as_str()? != "tconstruct:melting_fuel" { return None; }
    Some((u32::try_from(value.get("temperature")?.as_u64()?).ok()?, u32::try_from(value.get("rate")?.as_u64()?).ok()?))
}

fn lit(game: &mut super::Game<'_>, pos: Pos, active: bool) {
    let id = game.world.block(pos.0, pos.1, pos.2);
    let Some(mut state) = game.pack.assets.registry.state(id).cloned() else { return; };
    let Some(previous) = state.properties.get_mut("lit") else { return; };
    if previous == if active { "true" } else { "false" } { return; }
    *previous = active.to_string();
    if let Ok(id) = game.pack.assets.registry.resolve(&state) { let _ = game.world.set(pos.0, pos.1, pos.2, id); }
}

/// Tick an already loaded source melter. The heater below provides one solid
/// fuel item through Horizon's existing fuel provider; a container remainder
/// is never discarded. Recipes are resolved from the current world snapshot.
pub(super) fn tick_melter(game: &mut super::Game<'_>, pos: Pos) {
    let Some(saved) = store::read(&game.world, pos, "tconstruct:melter") else { return; };
    let Some(mut melter) = Melter::from_nbt(&saved) else { return; };
    let Some(BlockEntity::Chest(live)) = game.world.block_entities.get(&pos) else { return; };
    if live.len() < 3 { return; }
    for (index, slot) in live.iter().take(3).enumerate() {
        if !same_material(&melter.slots[index], slot) { melter.progress[index] = 0; }
        melter.slots[index] = slot.clone();
    }
    let below = (pos.0, pos.1 - 1, pos.2);
    let heater_state = game.world.block(below.0, below.1, below.2);
    let has_heater = &*game.res.facts().get(heater_state).name == "tconstruct:seared_heater";
    // Recipe matches are only used on the fuel and heat phases of the
    // source's four-tick cycle. Avoid scanning the source recipe snapshot on
    // the intervening ticks.
    let recipes: Vec<_> = if melter.phase == 0 || melter.phase == 2 {
        melter.slots.iter().map(|slot| slot.as_ref().and_then(|stack|
            super::tconstruct_source_melter_world::melting(&game.world, &stack.item))).collect()
    } else { vec![None; melter.slots.len()] };
    if melter.phase == 0 && has_heater && melter.fuel == 0 {
        if let Some((temperature, rate)) = solid_fuel_rule(game) {
            if recipes.iter().enumerate().any(|(slot, recipe)| recipe.as_ref().is_some_and(|(_, recipe, _)|
                melter.can_heat_slot(slot, temperature, recipe.temperature))) {
                let fuel = match game.world.block_entities.get(&below) {
                    Some(BlockEntity::Chest(slots)) => slots.first().and_then(Clone::clone), _ => None,
                };
                if let Some(fuel) = fuel {
                    let duration = game.res.providers().burn_time(game.res.items(), &fuel, false).max(0) as u32 / 4;
                    let remainder = game.res.items().get(&fuel.item).and_then(|info| info.crafting_remainder.clone());
                    if duration > 0 && (remainder.is_none() || fuel.count == 1) {
                        if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&below) {
                            if let Some(stack) = slots.first_mut().and_then(Option::as_mut) {
                                stack.count -= 1;
                                if stack.count == 0 {
                                    slots[0] = remainder.map(|id| horizon_items::items::ItemStack::new(&id, 1));
                                }
                                melter.fuel = duration; melter.fuel_quality = duration; melter.temperature = temperature; melter.rate = rate;
                                game.world.modified.insert((below.0 >> 4, below.2 >> 4));
                            }
                        }
                    }
                }
            }
        }
    }
    if melter.phase == 2 {
        let burning = has_heater && melter.fuel > 0;
        for (index, recipe) in recipes.iter().enumerate() {
            if let Some((_, recipe, fluid)) = recipe {
                if burning || melter.progress[index] == -1 {
                    // MeltingModule stores recipe time in tenths of a heat step.
                    melter.heat_slot(index, recipe.temperature, recipe.time * 10, fluid, recipe.output.amount);
                } else { melter.cool_slot(index); }
            } else { melter.progress[index] = 0; }
        }
        if melter.fuel > 0 { melter.fuel -= 1; }
        lit(game, pos, burning);
        if has_heater { lit(game, below, burning); }
    }
    melter.phase = (melter.phase + 1) % 4;
    if let Some(BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&pos) {
        for (dst, src) in live.iter_mut().take(3).zip(&melter.slots) { *dst = src.clone(); }
    }
    let _ = store::write(&mut game.world, pos, "tconstruct:melter", melter.to_nbt());
}

fn casting_at(game: &super::Game<'_>, pos: Pos) -> Option<(&'static str, Casting)> {
    let name = game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name.clone();
    let kind = kind(&name)?;
    if !matches!(kind, "tconstruct:table" | "tconstruct:basin") { return None; }
    let mut casting = Casting::from_nbt(&store::read(&game.world, pos, kind)?)?;
    let BlockEntity::Chest(slots) = game.world.block_entities.get(&pos)? else { return None; };
    if slots.len() < 2 { return None; }
    casting.input = slots[0].clone(); casting.output = slots[1].clone();
    Some((kind, casting))
}

/// Interact with the two casting slots in the source's order: insert one item
/// when empty, otherwise take the result first and then the cast.
pub(super) fn interact_casting(game: &mut super::Game<'_>, pos: Pos, block: &str) -> bool {
    if !ensure(game, pos, block) { return false; }
    let Some((kind, mut casting)) = casting_at(game, pos) else { return false; };
    if casting.tank.amount != 0 || casting.recipe.is_some() { return true; }
    let held = game.held().cloned();
    let mut taken = None;
    let mut remaining = None;
    if casting.input.is_none() && casting.output.is_none() {
        let Some(mut stack) = held else { return true; };
        let mut item = stack.clone(); item.count = 1;
        stack.count -= 1;
        remaining = Some((stack.count > 0).then_some(stack));
        casting.input = Some(item);
    } else if casting.output.is_some() { taken = casting.output.take(); }
    else { taken = casting.input.take(); }
    if !store::write(&mut game.world, pos, kind, casting.to_nbt()) { return false; }
    if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&pos) {
        slots[0] = casting.input.clone(); slots[1] = casting.output.clone();
    }
    if let Some(remaining) = remaining { *game.held_mut() = remaining; }
    if let Some(stack) = taken { game.give_or_drop(stack); }
    true
}

fn faucet_neighbors(game: &super::Game<'_>, pos: Pos) -> Option<(Pos, Pos)> {
    let state = game.pack.assets.registry.state(game.world.block(pos.0, pos.1, pos.2))?;
    let (dx, dy, dz) = match state.properties.get("facing")?.as_str() {
        "north" => (0, 0, -1), "south" => (0, 0, 1), "west" => (-1, 0, 0), "east" => (1, 0, 0), "down" => (0, -1, 0), _ => return None,
    };
    Some(((pos.0 - dx, pos.1 - dy, pos.2 - dz), (pos.0, pos.1 - 1, pos.2)))
}

fn smeltery_source(game: &super::Game<'_>, io: Pos) -> Option<(Pos, Smeltery)> {
    let block = &game.res.facts().get(game.world.block(io.0, io.1, io.2)).name;
    if !matches!(&**block, "tconstruct:seared_drain" | "tconstruct:seared_duct" | "tconstruct:scorched_drain" | "tconstruct:scorched_duct") { return None; }
    let controller = *game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners().get(&io)?;
    let state = store::read(&game.world, controller, "tconstruct:smeltery").and_then(|tag| Smeltery::from_nbt(&tag))?;
    Some((controller, state))
}

fn selected_smeltery_fluid(state: &Smeltery) -> Option<Tank> {
    let fluid = state.tank.fluids.first()?;
    Some(Tank { fluid: Some(fluid.id.clone()), amount: fluid.amount, capacity: state.tank.capacity.max(fluid.amount) })
}

fn apply_smeltery_take(state: &mut Smeltery, before: &Tank, after: &Tank) -> bool {
    let Some(amount) = before.amount.checked_sub(after.amount) else { return false; };
    if amount == 0 { return true; }
    before.fluid.as_deref().is_some_and(|id| state.tank.take(id, amount))
}

fn activate_smeltery_faucet(game: &mut super::Game<'_>, pos: Pos, source: Pos, target: Pos, mut faucet: Faucet) -> bool {
    let Some((controller, mut smeltery)) = smeltery_source(game, source) else { return true; };
    let Some(mut fluid) = selected_smeltery_fluid(&smeltery) else { return true; };
    let before = fluid.clone();
    let target_name = game.res.facts().get(game.world.block(target.0, target.1, target.2)).name.clone();
    if !matches!(&*target_name, "tconstruct:seared_table" | "tconstruct:seared_basin") { return true; }
    if !ensure(game, target, &target_name) { return false; }
    let Some((kind, mut casting)) = casting_at(game, target) else { return false; };
    if casting.recipe.is_none() {
        let input = casting.input.as_ref().map(|stack| &*stack.item);
        let Some(id) = fluid.fluid.as_deref() else { return true; };
        let Some((recipe_id, recipe, output)) = super::tconstruct_source_melter_world::casting(&game.world, kind == "tconstruct:basin", input, id) else { return true; };
        if !casting.prepare(&recipe_id, recipe.fluid.amount, recipe.cooling_time, horizon_items::items::ItemStack::new(&output, 1), recipe.consumes_cast, recipe.switch_slots) { return true; }
    }
    if !faucet.activate(&mut fluid, &casting.tank) { return true; }
    if !apply_smeltery_take(&mut smeltery, &before, &fluid) { return false; }
    store::write_many(&mut game.world, &[(controller, "tconstruct:smeltery", smeltery.to_nbt()), (target, kind, casting.to_nbt()), (pos, "tconstruct:faucet", faucet.to_nbt())])
}

fn tick_smeltery_faucet(game: &mut super::Game<'_>, pos: Pos, source: Pos, target: Pos, mut faucet: Faucet) {
    let Some((controller, mut smeltery)) = smeltery_source(game, source) else { return; };
    let mut fluid = selected_smeltery_fluid(&smeltery).unwrap_or_else(|| Tank::new(0));
    let before = fluid.clone();
    let Some((kind, mut casting)) = casting_at(game, target) else { return; };
    faucet.tick(&mut fluid, Some(&mut casting.tank));
    if !apply_smeltery_take(&mut smeltery, &before, &fluid) { return; }
    let _ = store::write_many(&mut game.world, &[(controller, "tconstruct:smeltery", smeltery.to_nbt()), (target, kind, casting.to_nbt()), (pos, "tconstruct:faucet", faucet.to_nbt())]);
}

/// Start or stop a source faucet. A destination recipe is resolved before any
/// liquid leaves the melter; all three entity edits are staged together.
pub(super) fn activate_faucet(game: &mut super::Game<'_>, pos: Pos, block: &str) -> bool {
    if !ensure(game, pos, block) { return false; }
    let Some(mut faucet) = store::read(&game.world, pos, "tconstruct:faucet").and_then(|tag| Faucet::from_nbt(&tag)) else { return false; };
    if faucet.active {
        faucet.stop_after_packet = true;
        return store::write(&mut game.world, pos, "tconstruct:faucet", faucet.to_nbt());
    }
    let Some((source, target)) = faucet_neighbors(game, pos) else { return false; };
    let source_name = game.res.facts().get(game.world.block(source.0, source.1, source.2)).name.clone();
    if matches!(&*source_name, "tconstruct:seared_drain" | "tconstruct:seared_duct" | "tconstruct:scorched_drain" | "tconstruct:scorched_duct") {
        return activate_smeltery_faucet(game, pos, source, target, faucet);
    }
    if &*source_name != "tconstruct:seared_melter" { return true; }
    let target_name = game.res.facts().get(game.world.block(target.0, target.1, target.2)).name.clone();
    if !matches!(&*target_name, "tconstruct:seared_table" | "tconstruct:seared_basin") { return true; }
    if !ensure(game, source, &source_name) || !ensure(game, target, &target_name) { return false; }
    let Some(mut melter) = store::read(&game.world, source, "tconstruct:melter").and_then(|tag| Melter::from_nbt(&tag)) else { return false; };
    let Some((casting_kind, mut casting)) = casting_at(game, target) else { return false; };
    let Some(fluid) = melter.tank.fluid.clone() else { return true; };
    if casting.recipe.is_none() {
        let input = casting.input.as_ref().map(|stack| &*stack.item);
        let Some((id, recipe, output)) = super::tconstruct_source_melter_world::casting(&game.world, casting_kind == "tconstruct:basin", input, &fluid) else { return true; };
        if !casting.prepare(&id, recipe.fluid.amount, recipe.cooling_time, horizon_items::items::ItemStack::new(&output, 1), recipe.consumes_cast, recipe.switch_slots) { return true; }
    }
    if !faucet.activate(&mut melter.tank, &casting.tank) { return true; }
    store::write_many(&mut game.world, &[(source, "tconstruct:melter", melter.to_nbt()), (target, casting_kind, casting.to_nbt()), (pos, "tconstruct:faucet", faucet.to_nbt())])
}

pub(super) fn tick_faucet(game: &mut super::Game<'_>, pos: Pos) {
    let Some(mut faucet) = store::read(&game.world, pos, "tconstruct:faucet").and_then(|tag| Faucet::from_nbt(&tag)) else { return; };
    if !faucet.active { return; }
    let Some((source, target)) = faucet_neighbors(game, pos) else { return; };
    // Wait if either chunk is unloaded; absence is not proof of a broken block.
    if !game.world.chunks.contains_key(&(source.0 >> 4, source.2 >> 4)) || !game.world.chunks.contains_key(&(target.0 >> 4, target.2 >> 4)) { return; }
    let source_name = game.res.facts().get(game.world.block(source.0, source.1, source.2)).name.clone();
    if matches!(&*source_name, "tconstruct:seared_drain" | "tconstruct:seared_duct" | "tconstruct:scorched_drain" | "tconstruct:scorched_duct") {
        tick_smeltery_faucet(game, pos, source, target, faucet);
        return;
    }
    let mut melter = (&*source_name == "tconstruct:seared_melter").then(||
        store::read(&game.world, source, "tconstruct:melter").and_then(|tag| Melter::from_nbt(&tag))).flatten();
    let mut empty = Tank::new(0);
    let input = melter.as_mut().map_or(&mut empty, |melter| &mut melter.tank);
    let Some((kind, mut casting)) = casting_at(game, target) else {
        faucet.tick(input, None);
        let _ = store::write(&mut game.world, pos, "tconstruct:faucet", faucet.to_nbt());
        return;
    };
    faucet.tick(input, Some(&mut casting.tank));
    let mut updates = vec![(target, kind, casting.to_nbt()), (pos, "tconstruct:faucet", faucet.to_nbt())];
    if let Some(melter) = melter { updates.push((source, "tconstruct:melter", melter.to_nbt())); }
    let _ = store::write_many(&mut game.world, &updates);
}

pub(super) fn tick_casting(game: &mut super::Game<'_>, pos: Pos) {
    let Some((kind, mut casting)) = casting_at(game, pos) else { return; };
    let Some(fluid) = casting.tank.fluid.as_deref() else { return; };
    let input = casting.input.as_ref().map(|stack| &*stack.item);
    let valid = super::tconstruct_source_melter_world::casting(&game.world, kind == "tconstruct:basin", input, fluid)
        .is_some_and(|(id, _, _)| casting.recipe.as_deref() == Some(&id));
    let previous_timer = casting.timer;
    if !casting.tick(valid) {
        if casting.timer != previous_timer { let _ = store::write(&mut game.world, pos, kind, casting.to_nbt()); }
        return;
    }
    if !store::write(&mut game.world, pos, kind, casting.to_nbt()) { return; }
    if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&pos) {
        slots[0] = casting.input; slots[1] = casting.output;
    }
}
