// Authoritative machine state uses normal live inventories plus custom NBT
// fields preserved by the source-machine save hooks.
use horizon_client::containers::BlockEntity;
use horizon_world::nbt::Tag;
use super::{tconstruct_source_melter_state::{Casting, Faucet, Melter, Tank}, tconstruct_source_melter_store as store};
use super::tconstruct_source_smeltery_state::Smeltery;

type Pos = (i32, i32, i32);

fn alloy_snapshot(tank: &super::tconstruct_source_smeltery_tank::SmelteryTank) -> (Vec<(String, u32)>, Vec<String>, u32) {
    let mut fluids = Vec::new();
    let mut tagged = Vec::new();
    let mut capacity = tank.capacity;
    for fluid in &tank.fluids {
        if fluid.tag.is_some() {
            tagged.push(fluid.id.clone());
            capacity = capacity.saturating_sub(fluid.amount);
        } else { fluids.push((fluid.id.clone(), fluid.amount)); }
    }
    (fluids, tagged, capacity)
}

pub(super) fn kind(block: &str) -> Option<&'static str> {
    match block {
        "tconstruct:seared_melter" => Some("tconstruct:melter"),
        "tconstruct:seared_heater" => Some("tconstruct:heater"),
        "tconstruct:seared_faucet" | "tconstruct:scorched_faucet" => Some("tconstruct:faucet"),
        "tconstruct:seared_table" | "tconstruct:scorched_table" => Some("tconstruct:table"),
        "tconstruct:seared_basin" => Some("tconstruct:basin"),
        "tconstruct:smeltery_controller" | "tconstruct:foundry_controller" => Some("tconstruct:smeltery"),
        "tconstruct:seared_fuel_tank" | "tconstruct:seared_fuel_gauge" | "tconstruct:scorched_fuel_tank" | "tconstruct:scorched_fuel_gauge" => Some("tconstruct:fuel_tank"),
        "tconstruct:seared_ingot_tank" | "tconstruct:seared_ingot_gauge" | "tconstruct:scorched_ingot_tank" | "tconstruct:scorched_ingot_gauge" => Some("tconstruct:ingot_tank"),
        "tconstruct:seared_lantern" | "tconstruct:scorched_lantern" => Some("tconstruct:lantern"),
        "tconstruct:seared_casting_tank" => Some("tconstruct:casting_tank"),
        "tconstruct:seared_fluid_cannon" => Some("tconstruct:fluid_cannon"),
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
        "tconstruct:lantern" => Tank::new(50).to_nbt(),
        "tconstruct:casting_tank" => super::tconstruct_source_casting_tank::CastingTank::new().to_nbt(),
        "tconstruct:fluid_cannon" => super::tconstruct_source_fluid_cannon::Cannon::new().to_nbt(),
        _ => return None,
    })
}

fn no_crafting_remainder(game: &super::Game<'_>, stack: &horizon_items::items::ItemStack) -> Option<bool> {
    if stack.item.as_ref() == "tconstruct:copper_can" || super::tconstruct_source_smeltery_fuel::item_capacity(&stack.item).is_some() {
        return super::tconstruct_source_fluid_container::no_crafting_remainder(stack);
    }
    let info = game.res.items().get(&stack.item)?;
    horizon_client::pluto_crafting_remainder::plain_has_no_remainder(stack, info.crafting_remainder.as_deref())
}

fn damaged_recipe(game: &super::Game<'_>, stack: &horizon_items::items::ItemStack)
    -> Option<(String, super::tconstruct_source_melting_recipes::FoundryRecipe, String, Vec<(String, u32)>)> {
    use horizon_world::nbt::Tag;
    let maximum = match stack.data().and_then(|data| data.other("minecraft:max_damage")) {
        Some(Tag::Int(value)) if *value >= 0 => u32::try_from(*value).ok()?,
        Some(_) => return None,
        None => game.res.items().get(&stack.item)?.max_damage,
    };
    super::tconstruct_source_melter_world::damaged_melting(&game.world, &stack.item, stack.damage, maximum)
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
    // Source EntityMeltingModule inserts dropped items on tick 12 of its
    // twenty-tick cycle, one item per empty melting slot, when it has a tank.
    // Stage both inventories; dropped entities change only after the BE write.
    let mut item_updates = Vec::new();
    let has_tank = game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners().iter().any(|(at, owner)| {
        if *owner != pos { return false; }
        let name = &game.res.facts().get(game.world.block(at.0, at.1, at.2)).name;
        super::tconstruct_source_smeltery_fuel::capacity(name).is_some()
    });
    if state.entity_phase == 12 && has_tank {
        use horizon_core::geometry::{Aabb, Vec3};
        let area = Aabb::new(Vec3::new(f64::from(min[0]+1), f64::from(min[1]+1), f64::from(min[2]+1)),
            Vec3::new(f64::from(max[0]), f64::from(max[1]+1), f64::from(max[2])));
        for (index, item) in game.dropped.iter().enumerate() {
            if item.health <= 0 || item.stack.count == 0 || !area.intersects(item.bounds()) { continue; }
            let mut remaining = item.stack.count;
            for slot in state.slots.iter_mut().filter(|slot| slot.is_none()) {
                if remaining == 0 { break; }
                let mut one = item.stack.clone(); one.count = 1;
                *slot = Some(one); remaining -= 1;
            }
            if remaining != item.stack.count { item_updates.push((index, remaining)); }
            if state.slots.iter().all(Option::is_some) { break; }
        }
    }
    state.entity_phase = (state.entity_phase + 1) % 20;
    let recipes: Vec<_> = if state.phase == 0 || state.phase == 1 {
        state.slots.iter().map(|slot| slot.as_ref().and_then(|stack| {
            if foundry {
                super::tconstruct_source_melter_world::foundry_melting_stack(&game.world, stack, no_crafting_remainder(game, stack))
                    .or_else(|| damaged_recipe(game, stack))
                    .or_else(|| super::tconstruct_source_melter_world::material_melting(&game.world, stack))
                    .map(|(id, recipe, fluid, byproducts)| (id, recipe.melting, fluid, byproducts))
            } else {
                super::tconstruct_source_melter_world::melting_stack(&game.world, stack, no_crafting_remainder(game, stack))
                    .or_else(|| damaged_recipe(game, stack).map(|(id, recipe, fluid, _)| (id, recipe.melting, fluid)))
                    .or_else(|| super::tconstruct_source_melter_world::material_melting(&game.world, stack).map(|(id, recipe, fluid, _)| (id, recipe.melting, fluid)))
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
            let (fluids, tagged, capacity) = alloy_snapshot(&state.tank);
            let can_alloy = !foundry && super::tconstruct_source_melter_world::alloy(&game.world, &fluids, &tagged, capacity, fuel.temperature)
                .is_some_and(|(_, inputs, output, amount)| {
                    let mut candidate = state.tank.clone();
                    let inputs = inputs.into_iter().map(|(id, amount)| super::tconstruct_source_smeltery_tank::FluidAmount { id, tag: None, amount }).collect::<Vec<_>>();
                    candidate.alloy(&inputs, &super::tconstruct_source_smeltery_tank::FluidAmount { id: output, tag: None, amount })
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
                            let primary = super::tconstruct_source_smeltery_tank::FluidAmount { id: fluid.clone(), tag: None, amount: recipe.output.amount };
                            let extras = byproducts.iter().map(|(id, amount)| super::tconstruct_source_smeltery_tank::FluidAmount { id: id.clone(), tag: None, amount: *amount }).collect::<Vec<_>>();
                            state.heat_foundry_slot(index, recipe.temperature, time, &primary, &extras);
                        } else { state.heat_slot(index, recipe.temperature, time, fluid, recipe.output.amount); }
                    }
                } else { state.cool_slot(index); }
            } else { state.progress[index] = 0; }
        }
    }
    if !foundry && state.phase == 2 && state.fuel > 0 {
        let (fluids, tagged, capacity) = alloy_snapshot(&state.tank);
        if let Some((_, inputs, output, amount)) = super::tconstruct_source_melter_world::alloy(&game.world, &fluids, &tagged, capacity, state.temperature) {
            let inputs = inputs.into_iter().map(|(id, amount)| super::tconstruct_source_smeltery_tank::FluidAmount { id, tag: None, amount }).collect::<Vec<_>>();
            state.tank.alloy(&inputs, &super::tconstruct_source_smeltery_tank::FluidAmount { id: output, tag: None, amount });
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
    let heat_codes = if state.phase == 1 || state.phase == 2 {
        Some(Tag::IntArray(state.slots.iter().enumerate().map(|(index, slot)| {
            if slot.is_none() { return -1; }
            let recipe = recipes.get(index).and_then(Option::as_ref).map(|(_, recipe, _, _)| (recipe.temperature, recipe.time));
            super::tconstruct_source_melter_render::encode_heat(recipe, state.temperature, state.progress[index])
        }).collect()))
    } else { saved.get("heat_codes").cloned() };
    let mut state_tag = state.to_nbt();
    if let (Tag::Compound(fields), Some(codes)) = (&mut state_tag, heat_codes) { fields.insert("heat_codes".into(), codes); }
    let mut updates = vec![(pos, "tconstruct:smeltery", state_tag)];
    if let Some(update) = fuel_update { updates.push(update); }
    if !store::write_many(&mut game.world, &updates) { return; }
    for (index, remaining) in item_updates.into_iter().rev() {
        let item = &game.dropped[index];
        let chunk = ((item.position.x.floor() as i32) >> 4, (item.position.z.floor() as i32) >> 4);
        game.world.modified.insert(chunk);
        if remaining == 0 { game.remove_dropped(index); }
        else { game.dropped[index].stack.count = remaining; game.dropped[index].needs_sync = true; }
    }
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

/// Exchange one supported container with a source tank. Stage both sides of
/// the transfer before persisting the tank or changing the held stack.
pub(super) fn interact_fuel_tank(game: &mut super::Game<'_>, pos: Pos, block: &str) -> bool {
    let Some(capacity) = super::tconstruct_source_smeltery_fuel::capacity(block) else { return false; };
    let Some(kind) = kind(block) else { return false; };
    if !ensure(game, pos, block) { return false; }
    let Some(mut tank) = store::read(&game.world, pos, kind).and_then(|tag| Tank::from_nbt(&tag, capacity)) else { return false; };
    let Some(held) = game.held().cloned() else { return true; };
    if held.count == 0 { return false; }
    game.ensure_datapacks();
    let fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
        .and_then(|value| value.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|value| value.as_ref().ok())
        .map(|all| all.iter().filter(|(id, _)| matches!(id.as_str(), "forge:bottles/splash" | "forge:bottles/lingering"))
            .map(|(id, items)| (id.clone(), items.clone())).collect());
    let before = tank.amount;
    let mut slots = [Some(held), None];
    if !super::tconstruct_source_fluid_container::transfer_single_with_color(&mut tank, &mut slots,
        super::tconstruct_source_fluid_container::Direction::Auto,
        &|item| game.res.items().get(item).is_some(),
        &|fluid| fluid == "minecraft:lava" || fluids.contains(fluid), tags.as_ref(), game.lan.is_none() && game.lan_acting_uuid.is_none()) { return true; }
    if !store::write(&mut game.world, pos, kind, tank.to_nbt()) { return false; }
    if game.creative {
        // Preserve the held container in creative, but retain fluid removed
        // from the tank by giving the filled result to the player.
        if tank.amount < before {
            if let Some(result) = slots[1].take() { game.give_or_drop(result); }
        }
    } else if !game.creative {
        if let Some(rest) = slots[0].take() {
            *game.held_mut() = Some(rest);
            if let Some(result) = slots[1].take() { game.give_or_drop(result); }
        } else {
            *game.held_mut() = slots[1].take();
        }
    }
    true
}

pub(super) fn casting_tank_at(game: &super::Game<'_>, pos: Pos) -> Option<super::tconstruct_source_casting_tank::CastingTank> {
    let mut state = super::tconstruct_source_casting_tank::CastingTank::from_nbt(&store::read(&game.world, pos, "tconstruct:casting_tank")?)?;
    let BlockEntity::Chest(slots) = game.world.block_entities.get(&pos)? else { return None; };
    if slots.len() < 2 { return None; }
    state.slots = [slots[0].clone(), slots[1].clone()];
    state.lossless().then_some(state)
}
pub(super) fn save_casting_tank(game: &mut super::Game<'_>, pos: Pos, state: &super::tconstruct_source_casting_tank::CastingTank) -> bool {
    if !state.lossless() { return false; }
    if !matches!(game.world.block_entities.get(&pos), Some(BlockEntity::Chest(slots)) if slots.len() >= 2) { return false; }
    let before = store::read(&game.world,pos,"tconstruct:casting_tank")
        .and_then(|tag| super::tconstruct_source_casting_tank::CastingTank::from_nbt(&tag)).map_or(0, |state| state.comparator());
    if !store::write(&mut game.world, pos, "tconstruct:casting_tank", state.to_nbt()) { return false; }
    if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&pos) {
        slots[0] = state.slots[0].clone(); slots[1] = state.slots[1].clone();
    }
    notify_casting_comparator(game, pos, before, state.comparator());
    game.container_changed(pos);
    true
}
fn notify_casting_comparator(game: &mut super::Game<'_>, pos: Pos, before: i32, after: i32) {
    if before == after { return; }
    let at = horizon_core::position::BlockPos::new(pos.0,pos.1,pos.2);
    game.with_level(|behaviours, level, ticks| behaviours.update_neighbour_for_output_signal(level,ticks,at));
}
pub(super) fn process_casting_tank(game: &mut super::Game<'_>, state: &mut super::tconstruct_source_casting_tank::CastingTank) -> bool {
    game.ensure_datapacks();
    let Some((fluids, _)) = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref().and_then(|v| v.as_ref().ok()) else { return false; };
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok());
    state.process(&|item| game.res.items().get(item).is_some(), &|fluid| fluids.contains(fluid), tags,
        game.lan.is_none() && game.lan_acting_uuid.is_none())
}
/// The source uses the lower hit region for a held fluid container and the
/// upper region for its input/output inventory.
pub(super) fn interact_casting_tank(game: &mut super::Game<'_>, pos: Pos, clicked_tank: bool) -> bool {
    if !ensure(game, pos, "tconstruct:seared_casting_tank") { return false; }
    let Some(mut state) = casting_tank_at(game, pos) else { return false; };
    if clicked_tank {
        let Some(held) = game.held().cloned() else { return true; };
        if held.wire_patch().is_some() { return true; }
        game.ensure_datapacks();
        let Some((fluids, _)) = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref().and_then(|v| v.as_ref().ok()) else { return true; };
        let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok());
        let mut slots = [Some(held), None];
        let before = state.tank.amount;
        if !super::tconstruct_source_fluid_container::transfer_single_with_color(&mut state.tank, &mut slots,
            super::tconstruct_source_fluid_container::Direction::Auto, &|item| game.res.items().get(item).is_some(),
            &|fluid| fluids.contains(fluid), tags, game.lan.is_none() && game.lan_acting_uuid.is_none()) { return true; }
        let drained = state.tank.amount < before;
        let _ = process_casting_tank(game, &mut state);
        if !save_casting_tank(game, pos, &state) { return false; }
        if game.creative { if drained { if let Some(result) = slots[1].take() { game.give_or_drop(result); } } }
        else if let Some(rest) = slots[0].take() { *game.held_mut() = Some(rest); if let Some(result) = slots[1].take() { game.give_or_drop(result); } }
        else { *game.held_mut() = slots[1].take(); }
        return true;
    }
    let taken = if state.slots[1].is_some() { state.slots[1].take() } else { state.slots[0].take() };
    if let Some(stack) = taken {
        if !save_casting_tank(game, pos, &state) { return false; }
        game.give_or_drop(stack); return true;
    }
    let Some(mut held) = game.held().cloned() else { return true; };
    let fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref().and_then(|v| v.as_ref().ok());
    let Some((fluids, _)) = fluids else { return true; };
    if held.count == 0 || held.wire_patch().is_some() || !super::tconstruct_source_fluid_container::accepts_with_color(&held, &|id| fluids.contains(id), game.lan.is_none() && game.lan_acting_uuid.is_none()) { return true; }
    let mut one = held.clone(); one.count = 1; held.count -= 1;
    state.slots[0] = Some(one);
    let _ = process_casting_tank(game, &mut state);
    if !save_casting_tank(game, pos, &state) { return false; }
    *game.held_mut() = (held.count > 0).then_some(held);
    true
}

pub(super) fn cannon_at(game: &super::Game<'_>, pos: Pos) -> Option<super::tconstruct_source_fluid_cannon::Cannon> {
    let mut state = super::tconstruct_source_fluid_cannon::Cannon::from_nbt(&store::read(&game.world,pos,"tconstruct:fluid_cannon")?)?;
    let BlockEntity::Chest(slots) = game.world.block_entities.get(&pos)? else { return None; };
    state.item = slots.first()?.clone();
    state.lossless().then_some(state)
}
pub(super) fn save_cannon(game: &mut super::Game<'_>, pos: Pos, state: &super::tconstruct_source_fluid_cannon::Cannon) -> bool {
    if !state.lossless() || !matches!(game.world.block_entities.get(&pos), Some(BlockEntity::Chest(slots)) if !slots.is_empty()) { return false; }
    let before = store::read(&game.world,pos,"tconstruct:fluid_cannon")
        .and_then(|tag| super::tconstruct_source_fluid_cannon::Cannon::from_nbt(&tag)).map_or(0, |state| state.comparator());
    if !store::write(&mut game.world,pos,"tconstruct:fluid_cannon",state.to_nbt()) { return false; }
    if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&pos) { slots[0] = state.item.clone(); }
    notify_casting_comparator(game, pos, before, state.comparator());
    game.container_changed(pos); true
}
pub(super) fn interact_cannon(game: &mut super::Game<'_>, pos: Pos, clicked_tank: bool) -> bool {
    if !ensure(game,pos,"tconstruct:seared_fluid_cannon") { return false; }
    let Some(mut state) = cannon_at(game,pos) else { return false; };
    if clicked_tank {
        let Some(held) = game.held().cloned() else { return true; };
        if held.wire_patch().is_some() { return true; }
        game.ensure_datapacks();
        let Some((fluids,_)) = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref().and_then(|v| v.as_ref().ok()) else { return true; };
        let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v| v.as_ref().ok());
        let mut slots = [Some(held),None]; let before = state.tank.amount;
        if !super::tconstruct_source_fluid_container::transfer_single_with_color(&mut state.tank,&mut slots,
            super::tconstruct_source_fluid_container::Direction::Auto,&|id| game.res.items().get(id).is_some(),
            &|id| fluids.contains(id),tags,game.lan.is_none() && game.lan_acting_uuid.is_none()) { return true; }
        let drained = state.tank.amount < before;
        if !save_cannon(game,pos,&state) { return false; }
        if game.creative { if drained { if let Some(item) = slots[1].take() { game.give_or_drop(item); } } }
        else if let Some(rest) = slots[0].take() { *game.held_mut() = Some(rest); if let Some(item) = slots[1].take() { game.give_or_drop(item); } }
        else { *game.held_mut() = slots[1].take(); }
        return true;
    }
    let held = game.held().cloned();
    match (state.item.take(), held) {
        (None, None) => true,
        (None, Some(item)) if item.count > 0 && item.wire_patch().is_none() => {
            state.item = Some(item); if !save_cannon(game,pos,&state) { return false; }
            *game.held_mut() = None; true
        }
        (Some(item), None) => {
            if !save_cannon(game,pos,&state) { return false; }
            *game.held_mut() = Some(item); true
        }
        (Some(mut item), Some(_)) => {
            let mut inventory = game.inventory.clone(); let mut offhand = game.offhand.clone();
            let max = game.res.items().get(&item.item).map_or(64, |info| info.max_stack);
            let left = super::inventory_add(&mut inventory,&mut offhand,game.selected,max,&mut item);
            state.item = left.then_some(item);
            if !save_cannon(game,pos,&state) { return false; }
            game.inventory = inventory; game.offhand = offhand; true
        }
        _ => true,
    }
}

/// A formed drain exposes its controller's tank on every side in TConstruct.
pub(super) fn interact_drain(game: &mut super::Game<'_>, pos: Pos) -> bool {
    game.ensure_datapacks();
    let Some(controller) = game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners().get(&pos).copied() else { return false; };
    let block = game.res.facts().get(game.world.block(controller.0, controller.1, controller.2)).name.clone();
    if !matches!(&*block, "tconstruct:smeltery_controller" | "tconstruct:foundry_controller") { return false; }
    let Some((min, max, _)) = super::tconstruct_source_smeltery_world::detect(game, controller) else { return false; };
    let (x, y, z) = pos;
    let wall = y > min[1] && y <= max[1] && ((x == min[0] || x == max[0]) && z > min[2] && z < max[2]
        || (z == min[2] || z == max[2]) && x > min[0] && x < max[0]);
    let floor = y == min[1] && x > min[0] && x < max[0] && z > min[2] && z < max[2];
    let foundry_shell = &*block == "tconstruct:foundry_controller" && x >= min[0] && x <= max[0]
        && z >= min[2] && z <= max[2] && y >= min[1] && y <= max[1]
        && (y == min[1] || x == min[0] || x == max[0] || z == min[2] || z == max[2]);
    if !wall && !floor && !foundry_shell { return false; }
    let Some(mut state) = store::read(&game.world, controller, "tconstruct:smeltery").and_then(|tag| Smeltery::from_nbt(&tag)) else { return false; };
    let Some(held) = game.held().cloned() else { return false; };
    let fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
        .and_then(|value| value.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|value| value.as_ref().ok())
        .map(|all| all.iter().filter(|(id, _)| matches!(id.as_str(), "forge:bottles/splash" | "forge:bottles/lingering"))
            .map(|(id, items)| (id.clone(), items.clone())).collect());
    let before = state.tank.total();
    let mut slots = [Some(held), None];
    if !super::tconstruct_source_fluid_container::transfer_selected_with_color(&mut state.tank, &mut slots,
        super::tconstruct_source_fluid_container::Direction::Auto, None,
        &|item| game.res.items().get(item).is_some(), &|fluid| fluids.contains(fluid), tags.as_ref(), game.lan.is_none() && game.lan_acting_uuid.is_none()) { return false; }
    if !store::write(&mut game.world, controller, "tconstruct:smeltery", state.to_nbt()) { return false; }
    if game.creative {
        if state.tank.total() < before {
            if let Some(result) = slots[1].take() { game.give_or_drop(result); }
        }
    } else if let Some(rest) = slots[0].take() {
        *game.held_mut() = Some(rest);
        if let Some(result) = slots[1].take() { game.give_or_drop(result); }
    } else { *game.held_mut() = slots[1].take(); }
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
    let slots = match kind { "tconstruct:casting_tank" => 2, "tconstruct:fluid_cannon" => 1, _ => 27 };
    game.world.block_entities.get_or_insert_with(pos, || BlockEntity::Chest(vec![None; slots]));
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
    let below_name = game.res.facts().get(heater_state).name.clone();
    let has_heater = &*below_name == "tconstruct:seared_heater";
    let liquid_kind = super::tconstruct_source_smeltery_fuel::capacity(&below_name)
        .zip(kind(&below_name));
    // Recipe matches are only used on the fuel and heat phases of the
    // source's four-tick cycle. Avoid scanning the source recipe snapshot on
    // the intervening ticks.
    let recipes: Vec<_> = if melter.phase == 0 || melter.phase == 2 {
        melter.slots.iter().map(|slot| slot.as_ref().and_then(|stack|
            super::tconstruct_source_melter_world::melting_stack(&game.world, stack, no_crafting_remainder(game, stack))
                .or_else(|| damaged_recipe(game, stack).map(|(id, recipe, fluid, _)| (id, recipe.melting, fluid)))
                .or_else(|| super::tconstruct_source_melter_world::material_melting(&game.world, stack).map(|(id, recipe, fluid, _)| (id, recipe.melting, fluid))))).collect()
    } else { vec![None; melter.slots.len()] };
    let mut heater_update = None;
    let mut remainder_drop = None;
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
                    if duration > 0 && fuel.count > 0 {
                        let mut left = fuel.clone();
                        left.count -= 1;
                        let mut next = (left.count > 0).then_some(left);
                        if let Some(id) = remainder {
                            let remainder = horizon_items::items::ItemStack::new(&id, 1);
                            // HeaterItemHandler accepts only items whose solid
                            // burn time exceeds three ticks, even as remainders.
                            if game.res.providers().burn_time(game.res.items(), &remainder, false) <= 3 {
                                remainder_drop = Some(remainder);
                            } else {
                                match &mut next {
                                    None => next = Some(remainder),
                                    Some(stack) if stack.same_item_same_components(&remainder)
                                        && stack.count < game.res.items().get(&stack.item).map_or(64, |item| item.max_stack).min(64) => stack.count += 1,
                                    _ => remainder_drop = Some(remainder),
                                }
                            }
                        }
                        heater_update = Some(next);
                        melter.fuel = duration; melter.fuel_quality = duration; melter.temperature = temperature; melter.rate = rate;
                    }
                }
            }
        }
    }
    let mut fluid_update = None;
    if melter.phase == 0 && melter.fuel == 0 {
        if let Some((capacity, kind)) = liquid_kind {
            if let Some(mut tank) = store::read(&game.world, below, kind).and_then(|tag| Tank::from_nbt(&tag, capacity)) {
                if let Some(fluid) = tank.fluid.as_deref() {
                    if let Some(fuel) = super::tconstruct_source_melter_world::fluid_fuel(&game.world, fluid, tank.amount) {
                        let can_heat = recipes.iter().enumerate().any(|(slot, recipe)| recipe.as_ref().is_some_and(|(_, recipe, _)|
                            melter.can_heat_slot(slot, fuel.temperature, recipe.temperature)));
                        if can_heat && tank.take(fuel.fluid.amount).is_some() {
                            melter.fuel = fuel.duration; melter.fuel_quality = fuel.duration;
                            melter.temperature = fuel.temperature; melter.rate = fuel.rate;
                            fluid_update = Some((below, kind, tank.to_nbt()));
                        }
                    }
                }
            }
        }
    }
    let mut lit_update = None;
    if melter.phase == 2 {
        let formed = has_heater || liquid_kind.is_some();
        let burning = formed && melter.fuel > 0;
        // An unformed Melter consumes already-burning fuel without heating or
        // cooling. It only cools once that stored fuel has run out.
        if formed || melter.fuel == 0 {
            for (index, recipe) in recipes.iter().enumerate() {
                if let Some((_, recipe, fluid)) = recipe {
                    if burning || melter.progress[index] == -1 {
                        // MeltingModule stores recipe time in tenths of a heat step.
                        melter.heat_slot(index, recipe.temperature, recipe.time * 10, fluid, recipe.output.amount);
                    } else { melter.cool_slot(index); }
                } else { melter.progress[index] = 0; }
            }
        }
        if melter.fuel > 0 { melter.fuel -= 1; }
        if formed { lit_update = Some(burning); }
    }
    melter.phase = (melter.phase + 1) % 4;
    let heat_codes = if melter.phase == 1 || melter.phase == 3 {
        Some(Tag::IntArray(melter.slots.iter().enumerate().map(|(index, slot)| {
            if slot.is_none() { return -1; }
            let recipe = recipes.get(index).and_then(Option::as_ref).map(|(_, recipe, _)| (recipe.temperature, recipe.time));
            super::tconstruct_source_melter_render::encode_heat(recipe, melter.temperature, melter.progress[index])
        }).collect()))
    } else { saved.get("heat_codes").cloned() };
    let mut melter_tag = melter.to_nbt();
    let liquid_temperature = liquid_kind.and_then(|(capacity, kind)| store::read(&game.world, below, kind)
        .and_then(|tag| Tank::from_nbt(&tag, capacity)))
        .and_then(|tank| tank.fluid.and_then(|fluid| super::tconstruct_source_melter_world::fluid_fuel(&game.world, &fluid, u32::MAX)))
        .map_or(0, |fuel| fuel.temperature);
    if let Tag::Compound(fields) = &mut melter_tag {
        fields.insert("fuel_temperature".into(), Tag::Long(i64::from(liquid_temperature)));
        if let Some(codes) = heat_codes { fields.insert("heat_codes".into(), codes); }
    }
    let mut updates = vec![(pos, "tconstruct:melter", melter_tag)];
    if let Some(update) = fluid_update { updates.push(update); }
    // The live inventories are distinct from the saved machine metadata.
    // Validate both before persisting, then publish the already-staged edits.
    if !matches!(game.world.block_entities.get(&pos), Some(BlockEntity::Chest(live)) if live.len() >= 3) { return; }
    if heater_update.is_some() && !matches!(game.world.block_entities.get(&below), Some(BlockEntity::Chest(live)) if !live.is_empty()) { return; }
    if !store::write_many(&mut game.world, &updates) { return; }
    if let Some(BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&pos) {
        for (dst, src) in live.iter_mut().take(3).zip(&melter.slots) { *dst = src.clone(); }
    }
    if let Some(next) = heater_update {
        if let Some(BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&below) { live[0] = next; }
        game.world.modified.insert((below.0 >> 4, below.2 >> 4));
    }
    if let Some(item) = remainder_drop { super::spill::tconstruct_source_smeltery_spill::spill(game, below, vec![item]); }
    if let Some(burning) = lit_update {
        lit(game, pos, burning);
        if has_heater { lit(game, below, burning); }
    }
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

// Empty-cast items stand in for an absent recipe cast, but remain physically
// in the slot. Scorched tables require a physical cast even for those recipes.
fn casting_recipe_at(game: &super::Game<'_>, pos: Pos, kind: &str, casting: &Casting, tank: &Tank)
    -> Option<(String, super::tconstruct_source_melting_recipes::CastingRecipe, horizon_items::items::ItemStack)> {
    let block = &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name;
    if block == "tconstruct:scorched_table" && casting.input.is_none() { return None; }
    let mut input = casting.input.as_ref();
    if let Some(stack) = input {
        let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
        let tag = if kind == "tconstruct:basin" { "tconstruct:casts/empty/basin" } else { "tconstruct:casts/empty/table" };
        if let Some(values) = tags.get(tag) {
            if values.contains(&*stack.item) { input = None; }
        } else {
            // A missing required cast such as an unregistered gold platform
            // invalidates the resolved tag, but unrelated items can still be
            // proved absent using the source tag graph.
            let raw = game.world.chunks.__horizon_mixins_tconstruct_recipe_tag_snapshot().as_ref()?;
            if !super::tconstruct_source_tags::definitely_absent(tag, &stack.item, raw).ok()? { return None; }
        }
    }
    matched_casting(&game.world, kind == "tconstruct:basin", input, tank, game.lan.is_none() && game.lan_acting_uuid.is_none())
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
    // Molding uses the placed material and one held pattern, before the
    // ordinary pickup path. Preserve a reusable pattern in the output slot.
    if casting.output.is_none() {
        if let Some(material) = casting.input.as_ref() {
            let pattern = held.as_ref().map(|stack| stack.item.as_ref());
            if let Some((recipe_id, recipe)) = super::tconstruct_source_melter_world::molding(&game.world, kind == "tconstruct:basin", &material.item, pattern) {
                let result = horizon_items::items::ItemStack::new(&recipe.result, 1);
                if let Some(mut stack) = held.clone() {
                    if !recipe.pattern_consumed { let mut item = stack.clone(); item.count = 1; casting.output = Some(item); }
                    stack.count -= 1;
                    remaining = Some((stack.count > 0).then_some(stack));
                    casting.input = Some(result);
                } else { casting.input = None; taken = Some(result); }
                if !store::write(&mut game.world, pos, kind, casting.to_nbt()) { return false; }
                if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&pos) {
                    slots[0] = casting.input.clone(); slots[1] = casting.output.clone();
                }
                if let Some(remaining) = remaining { *game.held_mut() = remaining; }
                if let Some(stack) = taken { game.give_or_drop(stack); }
                let player = game.native_acting_id();
                let result = horizon_items::items::ItemStack::new(&recipe.result, 1);
                game.native_item_crafted(player, &recipe_id, &result);
                game.stats.add("crafted", &result.item, 1);
                return true;
            }
            // A mold-less recipe also applies when the player is holding an
            // unrelated item; leave that item untouched and give the result.
            if held.is_some() {
                if let Some((_, recipe)) = super::tconstruct_source_melter_world::molding(&game.world, kind == "tconstruct:basin", &material.item, None) {
                    casting.input = None;
                    if !store::write(&mut game.world, pos, kind, casting.to_nbt()) { return false; }
                    if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&pos) { slots[0] = None; }
                    let result = horizon_items::items::ItemStack::new(&recipe.result, 1);
                    // The source fallback gives the assembled item without onCraftedBy.
                    game.give_or_drop(result);
                    return true;
                }
            }
        }
    }
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
    Some(Tank { fluid: Some(fluid.id.clone()), tag: fluid.tag.clone(), amount: fluid.amount, capacity: state.tank.capacity.max(fluid.amount) })
}

fn apply_smeltery_take(state: &mut Smeltery, before: &Tank, after: &Tank) -> bool {
    let Some(amount) = before.amount.checked_sub(after.amount) else { return false; };
    if amount == 0 { return true; }
    before.fluid.as_deref().is_some_and(|id| state.tank.take_tagged(id, before.tag.as_ref(), amount))
}

fn matched_casting(world: &super::tconstruct_world::World, basin: bool, input: Option<&horizon_items::items::ItemStack>, tank: &Tank, allow_color: bool)
    -> Option<(String, super::tconstruct_source_melting_recipes::CastingRecipe, horizon_items::items::ItemStack)> {
    let fluid = tank.fluid.as_deref()?;
    match tank.tag.as_ref() {
        Some(tag) => super::tconstruct_source_melter_world::potion_casting_stack(world, basin, input, fluid, tag, allow_color),
        None => super::tconstruct_source_melter_world::casting_stack(world, basin, input, fluid),
    }
}

fn activate_smeltery_faucet(game: &mut super::Game<'_>, pos: Pos, source: Pos, target: Pos, mut faucet: Faucet) -> bool {
    let Some((controller, mut smeltery)) = smeltery_source(game, source) else { return true; };
    let Some(mut fluid) = selected_smeltery_fluid(&smeltery) else { return true; };
    let before = fluid.clone();
    let target_name = game.res.facts().get(game.world.block(target.0, target.1, target.2)).name.clone();
    if &*target_name == "tconstruct:seared_fluid_cannon" {
        if !ensure(game,target,&target_name) { return false; }
        let Some(state) = cannon_at(game,target) else { return false; };
        if !faucet.activate(&mut fluid,&state.tank) { return true; }
        if !apply_smeltery_take(&mut smeltery,&before,&fluid) { return false; }
        return store::write_many(&mut game.world,&[(controller,"tconstruct:smeltery",smeltery.to_nbt()),(pos,"tconstruct:faucet",faucet.to_nbt())]);
    }
    if &*target_name == "tconstruct:seared_casting_tank" {
        if !ensure(game, target, &target_name) { return false; }
        let Some(state) = casting_tank_at(game, target) else { return false; };
        if !faucet.activate(&mut fluid, &state.tank) { return true; }
        if !apply_smeltery_take(&mut smeltery, &before, &fluid) { return false; }
        return store::write_many(&mut game.world, &[(controller,"tconstruct:smeltery",smeltery.to_nbt()),(pos,"tconstruct:faucet",faucet.to_nbt())]);
    }
    if !matches!(&*target_name, "tconstruct:seared_table" | "tconstruct:scorched_table" | "tconstruct:seared_basin") { return true; }
    if !ensure(game, target, &target_name) { return false; }
    let Some((kind, mut casting)) = casting_at(game, target) else { return false; };
    if casting.recipe.is_none() {
        let Some((recipe_id, recipe, output)) = casting_recipe_at(game, target, kind, &casting, &fluid) else { return true; };
        if !casting.prepare(&recipe_id, recipe.fluid.amount, recipe.cooling_time, output, recipe.consumes_cast, recipe.switch_slots) { return true; }
    }
    if !faucet.activate(&mut fluid, &casting.tank) { return true; }
    if !apply_smeltery_take(&mut smeltery, &before, &fluid) { return false; }
    store::write_many(&mut game.world, &[(controller, "tconstruct:smeltery", smeltery.to_nbt()), (target, kind, casting.to_nbt()), (pos, "tconstruct:faucet", faucet.to_nbt())])
}

fn tick_smeltery_faucet(game: &mut super::Game<'_>, pos: Pos, source: Pos, target: Pos, mut faucet: Faucet) {
    let Some((controller, mut smeltery)) = smeltery_source(game, source) else { return; };
    let mut fluid = selected_smeltery_fluid(&smeltery).unwrap_or_else(|| Tank::new(0));
    let before = fluid.clone();
    if &*game.res.facts().get(game.world.block(target.0,target.1,target.2)).name == "tconstruct:seared_fluid_cannon" {
        let Some(mut state) = cannon_at(game,target) else { return; };
        let previous_strength = state.comparator();
        faucet.tick(&mut fluid,Some(&mut state.tank));
        if !apply_smeltery_take(&mut smeltery,&before,&fluid) || !state.lossless() { return; }
        if store::write_many(&mut game.world,&[(controller,"tconstruct:smeltery",smeltery.to_nbt()),(target,"tconstruct:fluid_cannon",state.to_nbt()),(pos,"tconstruct:faucet",faucet.to_nbt())]) {
            notify_casting_comparator(game, target, previous_strength, state.comparator());
        }
        return;
    }
    if &*game.res.facts().get(game.world.block(target.0,target.1,target.2)).name == "tconstruct:seared_casting_tank" {
        let Some(mut state) = casting_tank_at(game, target) else { return; };
        let previous_strength = state.comparator();
        let previous_tank = state.tank.to_nbt();
        faucet.tick(&mut fluid, Some(&mut state.tank));
        if !apply_smeltery_take(&mut smeltery, &before, &fluid) { return; }
        if state.tank.to_nbt() != previous_tank { let _ = process_casting_tank(game, &mut state); }
        if !state.lossless() { return; }
        if store::write_many(&mut game.world, &[(controller,"tconstruct:smeltery",smeltery.to_nbt()),(target,"tconstruct:casting_tank",state.to_nbt()),(pos,"tconstruct:faucet",faucet.to_nbt())]) {
            let strength = state.comparator();
            if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&target) { slots[0] = state.slots[0].clone(); slots[1] = state.slots[1].clone(); }
            notify_casting_comparator(game, target, previous_strength, strength);
        }
        return;
    }
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
    if &*target_name == "tconstruct:seared_fluid_cannon" {
        if !ensure(game,source,&source_name) || !ensure(game,target,&target_name) { return false; }
        let Some(mut melter) = store::read(&game.world,source,"tconstruct:melter").and_then(|tag| Melter::from_nbt(&tag)) else { return false; };
        let Some(state) = cannon_at(game,target) else { return false; };
        if !faucet.activate(&mut melter.tank,&state.tank) { return true; }
        return store::write_many(&mut game.world,&[(source,"tconstruct:melter",melter.to_nbt()),(pos,"tconstruct:faucet",faucet.to_nbt())]);
    }
    if &*target_name == "tconstruct:seared_casting_tank" {
        if !ensure(game, source, &source_name) || !ensure(game, target, &target_name) { return false; }
        let Some(mut melter) = store::read(&game.world, source, "tconstruct:melter").and_then(|tag| Melter::from_nbt(&tag)) else { return false; };
        let Some(state) = casting_tank_at(game, target) else { return false; };
        if !faucet.activate(&mut melter.tank, &state.tank) { return true; }
        return store::write_many(&mut game.world, &[(source,"tconstruct:melter",melter.to_nbt()),(pos,"tconstruct:faucet",faucet.to_nbt())]);
    }
    if !matches!(&*target_name, "tconstruct:seared_table" | "tconstruct:scorched_table" | "tconstruct:seared_basin") { return true; }
    if !ensure(game, source, &source_name) || !ensure(game, target, &target_name) { return false; }
    let Some(mut melter) = store::read(&game.world, source, "tconstruct:melter").and_then(|tag| Melter::from_nbt(&tag)) else { return false; };
    let Some((casting_kind, mut casting)) = casting_at(game, target) else { return false; };
    if melter.tank.fluid.is_none() { return true; }
    if casting.recipe.is_none() {
        let Some((id, recipe, output)) = casting_recipe_at(game, target, casting_kind, &casting, &melter.tank) else { return true; };
        if !casting.prepare(&id, recipe.fluid.amount, recipe.cooling_time, output, recipe.consumes_cast, recipe.switch_slots) { return true; }
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
    if &*game.res.facts().get(game.world.block(target.0,target.1,target.2)).name == "tconstruct:seared_fluid_cannon" {
        let Some(mut state) = cannon_at(game,target) else { return; };
        let previous_strength = state.comparator();
        faucet.tick(input,Some(&mut state.tank));
        if !state.lossless() { return; }
        let mut updates = vec![(target,"tconstruct:fluid_cannon",state.to_nbt()),(pos,"tconstruct:faucet",faucet.to_nbt())];
        if let Some(melter) = melter { updates.push((source,"tconstruct:melter",melter.to_nbt())); }
        if store::write_many(&mut game.world,&updates) {
            notify_casting_comparator(game, target, previous_strength, state.comparator());
        }
        return;
    }
    if &*game.res.facts().get(game.world.block(target.0,target.1,target.2)).name == "tconstruct:seared_casting_tank" {
        let Some(mut state) = casting_tank_at(game, target) else { return; };
        let previous_strength = state.comparator();
        let previous_tank = state.tank.to_nbt();
        faucet.tick(input, Some(&mut state.tank));
        if state.tank.to_nbt() != previous_tank { let _ = process_casting_tank(game, &mut state); }
        if !state.lossless() { return; }
        let mut updates = vec![(target,"tconstruct:casting_tank",state.to_nbt()),(pos,"tconstruct:faucet",faucet.to_nbt())];
        if let Some(melter) = melter { updates.push((source,"tconstruct:melter",melter.to_nbt())); }
        if store::write_many(&mut game.world, &updates) {
            let strength = state.comparator();
            if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&target) { slots[0] = state.slots[0].clone(); slots[1] = state.slots[1].clone(); }
            notify_casting_comparator(game, target, previous_strength, strength);
        }
        return;
    }
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
    if casting.tank.fluid.is_none() { return; }
    let valid = casting_recipe_at(game, pos, kind, &casting, &casting.tank)
        .is_some_and(|(id, recipe, result)| casting.recipe.as_deref() == Some(&id)
            && casting.tank.capacity == recipe.fluid.amount && casting.cooling_time == recipe.cooling_time
            && casting.consumes_cast == recipe.consumes_cast && casting.switch_slots == recipe.switch_slots
            && casting.result.as_ref().is_some_and(|stored| stored.same_item_same_components(&result) && stored.count == result.count));
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
