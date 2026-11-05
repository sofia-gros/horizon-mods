use super::{View, Kind, dir_from_name, offset};
// Hopper entry points live in game::machines, where View and the host's
// container rules are available. Intercept only the explicitly supported machines.
fn tconstruct_chute_controller(game: &crate::game::Game<'_>, chute: (i32, i32, i32)) -> Option<((i32, i32, i32), usize)> {
    let name = game.res.facts().get(game.world.block(chute.0, chute.1, chute.2)).name.clone();
    if !matches!(&*name, "tconstruct:seared_chute" | "tconstruct:scorched_chute") { return None; }
    let controller = *game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners().get(&chute)?;
    let block = game.res.facts().get(game.world.block(controller.0, controller.1, controller.2)).name.clone();
    if !matches!(&*block, "tconstruct:smeltery_controller" | "tconstruct:foundry_controller") { return None; }
    let (min, max, _) = crate::game::tconstruct_source_smeltery_world::detect(game, controller)?;
    let (x,y,z) = chute;
    let wall = y > min[1] && y <= max[1] && ((x == min[0] || x == max[0]) && z > min[2] && z < max[2]
        || (z == min[2] || z == max[2]) && x > min[0] && x < max[0]);
    let floor = y == min[1] && x > min[0] && x < max[0] && z > min[2] && z < max[2];
    let foundry = &*block == "tconstruct:foundry_controller" && x >= min[0] && x <= max[0]
        && z >= min[2] && z <= max[2] && y >= min[1] && y <= max[1]
        && (y == min[1] || x == min[0] || x == max[0] || z == min[2] || z == max[2]);
    if !(wall || floor || foundry) { return None; }
    let dx = usize::try_from(max[0].checked_sub(min[0])?.checked_sub(1)?).ok()?;
    let dy = usize::try_from(max[1].checked_sub(min[1])?).ok()?;
    let dz = usize::try_from(max[2].checked_sub(min[2])?.checked_sub(1)?).ok()?;
    Some((controller, dx.checked_mul(dy)?.checked_mul(dz)?))
}

// Callbacks may mutate either inventory; never commit a stale plan.
fn chute_snapshots_match(
    before: &[Option<horizon_items::items::ItemStack>],
    current: &[Option<horizon_items::items::ItemStack>],
    saved: &horizon_world::nbt::Tag,
    current_saved: Option<&horizon_world::nbt::Tag>,
) -> bool {
    before == current && current_saved == Some(saved)
}

fn tconstruct_chute_transfer(game: &mut crate::game::Game<'_>, hopper: &View, chute: (i32, i32, i32), push: bool) -> bool {
    use horizon_client::containers::BlockEntity;
    use horizon_world::nbt::Tag;
    use horizon_client::pluto_inventory::{plan_between, StackSnapshot};
    if hopper.kind != Kind::Hopper || hopper.parts.len() != 1 || hopper.len() != 5 { return false; }
    let hopper_pos = hopper.parts[0].0;
    let Some((controller, volume)) = tconstruct_chute_controller(game, chute) else { return false; };
    if !matches!(game.world.block_entities.get(&controller), Some(BlockEntity::Chest(_))) { return false; }
    let Some(saved) = crate::game::tconstruct_source_melter_store::read(&game.world, controller, "tconstruct:smeltery") else { return false; };
    let Some(mut state) = crate::game::tconstruct_source_smeltery_state::Smeltery::from_nbt(&saved) else { return false; };
    if state.slots.len() != volume { return false; }
    let Some(BlockEntity::Chest(original_hopper)) = game.world.block_entities.get(&hopper_pos) else { return false; };
    if original_hopper.len() != 5 { return false; }
    let before_hopper = original_hopper.clone();
    let containers = game.containers();
    fn snapshot(stack: &Option<horizon_items::items::ItemStack>) -> Option<StackSnapshot<'_, horizon_items::items::ItemStack>> {
        stack.as_ref().filter(|s| s.count > 0).map(|item| StackSnapshot { item, count: item.count })
    }
    let mut selected = None;
    if push {
        for (h, source) in before_hopper.iter().enumerate() {
            let Some(item) = source.as_ref().filter(|s| s.count > 0 && s.wire_patch().is_none()) else { continue; };
            for (m, target) in state.slots.iter().enumerate() {
                if target.is_some() { continue; } // Source melting slots accept one item.
                if plan_between(snapshot(source), snapshot(target), 1, |_, _| 1, |_, _| true,
                    |a, b| a.same_item_same_components(b)).ok().flatten().is_some() {
                    selected = Some((h, m, item.clone())); break;
                }
            }
            if selected.is_some() { break; }
        }
    } else {
        for (m, source) in state.slots.iter().enumerate() {
            let Some(item) = source.as_ref().filter(|s| s.count > 0 && s.wire_patch().is_none()) else { continue; };
            for (h, target) in before_hopper.iter().enumerate() {
                if target.as_ref().is_some_and(|stack| stack.wire_patch().is_some()) { continue; }
                if !containers.can_place(hopper, h, item) { continue; }
                let max = containers.max_stack(item);
                if plan_between(snapshot(source), snapshot(target), 1, |_, _| max, |_, _| true,
                    |a, b| a.same_item_same_components(b)).ok().flatten().is_some() {
                    selected = Some((h, m, item.clone())); break;
                }
            }
            if selected.is_some() { break; }
        }
    }
    let Some((h, m, mut item)) = selected else { return false; };
    let (from, to) = if push { (hopper_pos, chute) } else { (chute, hopper_pos) };
    let mut cancel = false;
    game.native_inventory_move_item([from.0, from.1, from.2], [to.0, to.1, to.2], &item.item, &mut cancel);
    if cancel { return false; }
    // The native callback can change either inventory or the structure.
    if tconstruct_chute_controller(game, chute) != Some((controller, volume)) { return false; }
    let Some(BlockEntity::Chest(current_hopper)) = game.world.block_entities.get(&hopper_pos) else { return false; };
    let current_saved = crate::game::tconstruct_source_melter_store::read(&game.world, controller, "tconstruct:smeltery");
    if !chute_snapshots_match(&before_hopper, current_hopper, &saved, current_saved.as_ref()) { return false; }
    let mut next_hopper = before_hopper.clone();
    item.count = 1;
    let moving = item.clone();
    if !push {
        let Some(Tag::List(raw_slots)) = saved.get("slots") else { return false; };
        let Some(raw) = raw_slots.get(m) else { return false; };
        let Some(source) = state.slots[m].as_ref() else { return false; };
        if *raw != horizon_items::item_nbt::item_to_nbt(source) { return false; }
    }
    if push {
        let Some(source) = next_hopper[h].as_mut() else { return false; };
        source.count -= 1;
        if source.count == 0 { next_hopper[h] = None; }
        state.slots[m] = Some(item);
    } else {
        let Some(source) = state.slots[m].as_mut() else { return false; };
        source.count -= 1;
        if source.count == 0 { state.slots[m] = None; }
        match next_hopper[h].as_mut() {
            Some(target) if target.count > 0 => target.count += 1,
            _ => next_hopper[h] = Some(item),
        }
    }
    state.progress[m] = 0;
    // Preserve every unrelated raw slot and machine field. The transferred
    // item must survive the host's saved-item codec before either side moves.
    let encoded = horizon_items::item_nbt::item_to_nbt(&moving);
    let Some(decoded) = horizon_items::item_nbt::item_from_nbt(&encoded) else { return false; };
    if !moving.same_item_same_components(&decoded) || moving.count != decoded.count { return false; }
    let Tag::Compound(mut fields) = saved else { return false; };
    let Some(Tag::List(raw_slots)) = fields.get_mut(&"slots".into()) else { return false; };
    let Some(raw_slot) = raw_slots.get_mut(m) else { return false; };
    if push {
        if !matches!(raw_slot, Tag::Compound(map) if map.is_empty()) { return false; }
        *raw_slot = horizon_items::item_nbt::item_to_nbt(state.slots[m].as_ref().unwrap());
    } else if state.slots[m].is_none() {
        *raw_slot = Tag::compound(Vec::<(&str, Tag)>::new());
    } else {
        let Tag::Compound(map) = raw_slot else { return false; };
        map.insert("count".into(), Tag::Int(state.slots[m].as_ref().unwrap().count as i32));
    }
    let Some(Tag::IntArray(progress)) = fields.get_mut(&"progress".into()) else { return false; };
    let Some(progress) = progress.get_mut(m) else { return false; };
    *progress = 0;
    let Some(BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&hopper_pos) else { return false; };
    *live = next_hopper;
    if !crate::game::tconstruct_source_melter_store::write(&mut game.world, controller, "tconstruct:smeltery", Tag::Compound(fields)) {
        if let Some(BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&hopper_pos) { *live = before_hopper; }
        return false;
    }
    if game.container == Some(controller) && game.remote.is_none() {
        let offset = game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages().get(&controller).map_or(0, |page| page.1);
        let containers = game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers().get(&controller)
            .map(|(slots, _)| slots.clone()).unwrap_or([None, None]);
        if let Some(BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&controller) {
            *live = crate::game::tconstruct_source_smeltery_page::project_with_containers(&state.slots, offset, &containers);
            crate::game::tconstruct_source_machine_projection_world::record(&mut game.world, controller, offset);
        }
    }
    game.container_changed(controller);
    true
}

pub(super) fn eject(game: &mut crate::game::Game<'_>, pos: (i32, i32, i32), hopper: &View) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let state = game.world.block(pos.0, pos.1, pos.2);
    let Some(facing) = game.containers().prop(state, "facing").and_then(dir_from_name) else { return Continue(()); };
    let target = offset(pos, facing);
    let name = game.res.facts().get(game.world.block(target.0, target.1, target.2)).name.clone();
    if matches!(&*name, "tconstruct:seared_table" | "tconstruct:scorched_table" | "tconstruct:seared_basin" | "tconstruct:scorched_basin") {
        game.native_hopper_inventory_search([pos.0,pos.1,pos.2], Some([target.0,target.1,target.2]), None);
        return Break(casting_transfer(game, hopper, target, true));
    }
    if &*name == "tconstruct:seared_fluid_cannon" {
        game.native_hopper_inventory_search([pos.0,pos.1,pos.2], Some([target.0,target.1,target.2]), None);
        return Break(cannon_transfer(game, hopper, target, true));
    }
    if &*name == "tconstruct:seared_casting_tank" {
        game.native_hopper_inventory_search([pos.0,pos.1,pos.2], Some([target.0,target.1,target.2]), None);
        return Break(casting_tank_transfer(game, hopper, target, true));
    }
    if !matches!(&*name, "tconstruct:seared_chute" | "tconstruct:scorched_chute") { return Continue(()); }
    game.native_hopper_inventory_search([pos.0,pos.1,pos.2], Some([target.0,target.1,target.2]), None);
    Break(tconstruct_chute_transfer(game, hopper, target, true))
}

pub(super) fn suck(game: &mut crate::game::Game<'_>, pos: (i32, i32, i32), hopper: &View) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(y) = pos.1.checked_add(1) else { return Continue(()); };
    let target = (pos.0, y, pos.2);
    let name = game.res.facts().get(game.world.block(target.0, target.1, target.2)).name.clone();
    if matches!(&*name, "tconstruct:seared_table" | "tconstruct:scorched_table" | "tconstruct:seared_basin" | "tconstruct:scorched_basin") {
        game.native_hopper_inventory_search([pos.0,pos.1,pos.2], Some([target.0,target.1,target.2]), None);
        return Break(casting_transfer(game, hopper, target, false));
    }
    if &*name == "tconstruct:seared_fluid_cannon" {
        game.native_hopper_inventory_search([pos.0,pos.1,pos.2], Some([target.0,target.1,target.2]), None);
        return Break(cannon_transfer(game, hopper, target, false));
    }
    if &*name == "tconstruct:seared_casting_tank" {
        game.native_hopper_inventory_search([pos.0,pos.1,pos.2], Some([target.0,target.1,target.2]), None);
        return Break(casting_tank_transfer(game, hopper, target, false));
    }
    if !matches!(&*name, "tconstruct:seared_chute" | "tconstruct:scorched_chute") { return Continue(()); }
    game.native_hopper_inventory_search([pos.0,pos.1,pos.2], Some([target.0,target.1,target.2]), None);
    Break(tconstruct_chute_transfer(game, hopper, target, false))
}

// Match the source casting block's sided input/output and one-item limit.
// Work from both complete live inventories and raw metadata, and recheck all
// three after callbacks so no unrelated stack or saved field is overwritten.
fn casting_transfer(game: &mut crate::game::Game<'_>, hopper: &View, target: (i32,i32,i32), push: bool) -> bool {
    use horizon_client::{containers::BlockEntity, pluto_inventory::{plan_between, StackSnapshot}};
    use horizon_items::{item_nbt::{item_from_nbt, item_to_nbt}, items::ItemStack};
    if hopper.kind != Kind::Hopper || hopper.parts.len() != 1 || hopper.len() != 5 { return false; }
    let hopper_pos = hopper.parts[0].0;
    let hopper_block = game.world.block(hopper_pos.0, hopper_pos.1, hopper_pos.2);
    let target_block = game.world.block(target.0, target.1, target.2);
    let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get(&hopper_pos) else { return false; };
    if slots.len() != 5 { return false; }
    let before_hopper = slots.clone();
    let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get(&target) else { return false; };
    if slots.len() < 2 { return false; }
    let before_target = slots.clone();
    let Some((kind, state)) = crate::game::tconstruct_source_machine_runtime::casting_at(game, target) else { return false; };
    let Some(saved) = crate::game::tconstruct_source_melter_store::read(&game.world, target, kind) else { return false; };
    // A mismatch may mean a prior external edit or an unrepresentable saved
    // component. Do not overwrite or resurrect either slot in that case.
    for (key, live) in [("input", &state.input), ("output", &state.output)] {
        match live {
            None if saved.get(key).is_none() => {},
            Some(item) if item.wire_patch().is_none() && saved.get(key) == Some(&item_to_nbt(item)) => {},
            _ => return false,
        }
    }
    if if push { !state.hopper_push_allowed() } else { !state.hopper_pull_allowed() } { return false; }
    fn snapshot(value: &Option<ItemStack>) -> Option<StackSnapshot<'_, ItemStack>> {
        value.as_ref().filter(|s| s.count > 0).map(|item| StackSnapshot { item, count: item.count })
    }
    let (from, to) = if push { (hopper_pos, target) } else { (target, hopper_pos) };
    let candidates: Vec<(usize, ItemStack)> = if push {
        before_hopper.iter().enumerate().filter_map(|(index, source)| {
            let item = source.as_ref().filter(|s| s.count > 0 && s.wire_patch().is_none())?;
            plan_between(snapshot(source), None, 1, |_,_| 1, |_,_| true, |a,b| a.same_item_same_components(b)).ok().flatten()?;
            Some((index, item.clone()))
        }).collect()
    } else {
        let Some(item) = state.output.as_ref().filter(|s| s.count > 0 && s.wire_patch().is_none()) else { return false; };
        let containers = game.containers();
        before_hopper.iter().enumerate().filter_map(|(index, dest)| {
            if dest.as_ref().is_some_and(|s| s.wire_patch().is_some()) || !containers.can_place(hopper, index, item) { return None; }
            let max = containers.max_stack(item);
            plan_between(snapshot(&state.output), snapshot(dest), 1, |_,_| max, |_,_| true, |a,b| a.same_item_same_components(b)).ok().flatten()?;
            Some((index, item.clone()))
        }).collect()
    };
    // Insertion offers each source slot to the callback separately, as the
    // host does. Extraction has one source slot, so cancellation ends it.
    let candidates = if push { candidates } else { candidates.into_iter().take(1).collect() };
    let choice = crate::game::tconstruct_source_melter_state::select_casting_candidate(candidates, |_, item| {
        let mut cancel = false;
        game.native_inventory_move_item([from.0,from.1,from.2], [to.0,to.1,to.2], &item.item, &mut cancel);
        if game.world.block(hopper_pos.0,hopper_pos.1,hopper_pos.2) != hopper_block
            || game.world.block(target.0,target.1,target.2) != target_block
            || !matches!(game.world.block_entities.get(&hopper_pos), Some(BlockEntity::Chest(slots)) if *slots == before_hopper)
            || !matches!(game.world.block_entities.get(&target), Some(BlockEntity::Chest(slots)) if *slots == before_target)
            || crate::game::tconstruct_source_melter_store::read(&game.world, target, kind).as_ref() != Some(&saved) { return None; }
        Some(!cancel)
    });
    let Some((index, mut item)) = choice else { return false; };
    if !push {
        let containers = game.containers();
        if !containers.can_place(hopper, index, &item) { return false; }
        let max = containers.max_stack(&item);
        if plan_between(snapshot(&state.output), snapshot(&before_hopper[index]), 1, |_,_| max, |_,_| true, |a,b| a.same_item_same_components(b)).ok().flatten().is_none() { return false; }
    }
    item.count = 1;
    let encoded = item_to_nbt(&item);
    if item_from_nbt(&encoded).is_none_or(|decoded| decoded.count != 1 || !decoded.same_item_same_components(&item)) { return false; }
    let Some((after_hopper, next_state)) = state.hopper_transfer(&before_hopper, index, push) else { return false; };
    let mut after_target = before_target.clone();
    let (key, before_slot, after_slot) = if push { ("input", &state.input, &next_state.input) } else { ("output", &state.output, &next_state.output) };
    let Some(patched) = crate::game::tconstruct_source_melter_state::patch_casting_slot(&saved, key, before_slot, after_slot) else { return false; };
    after_target[0] = next_state.input;
    after_target[1] = next_state.output;
    if !crate::game::tconstruct_source_melter_store::write(&mut game.world, target, kind, patched) { return false; }
    if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&hopper_pos) { *slots = after_hopper; } else { return false; }
    if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&target) { *slots = after_target; } else { return false; }
    game.container_changed(target);
    true
}

fn cannon_transfer(game: &mut crate::game::Game<'_>, hopper: &View, target: (i32,i32,i32), push: bool) -> bool {
    use horizon_client::{containers::BlockEntity, pluto_inventory::{plan_between, StackSnapshot}};
    if hopper.kind != Kind::Hopper || hopper.parts.len() != 1 || hopper.len() != 5 { return false; }
    let hopper_pos = hopper.parts[0].0;
    let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get(&hopper_pos) else { return false; };
    if slots.len() != 5 { return false; }
    let before = slots.clone();
    let Some(mut cannon) = crate::game::tconstruct_source_machine_runtime::cannon_at(game,target) else { return false; };
    let machine_before = cannon.to_nbt();
    fn snapshot(value: &Option<horizon_items::items::ItemStack>) -> Option<StackSnapshot<'_,horizon_items::items::ItemStack>> {
        value.as_ref().filter(|s| s.count > 0).map(|item| StackSnapshot { item, count: item.count })
    }
    let mut choice = None;
    let containers = game.containers();
    if push {
        if cannon.item.as_ref().is_some_and(|s| s.wire_patch().is_some()) { return false; }
        for (index, source) in before.iter().enumerate() {
            let Some(item) = source.as_ref().filter(|s| s.count > 0 && s.wire_patch().is_none()) else { continue; };
            let max = containers.max_stack(item).min(64);
            if plan_between(snapshot(source),snapshot(&cannon.item),1,|_,_| max,|_,_| true,|a,b| a.same_item_same_components(b)).ok().flatten().is_some() {
                choice = Some((index,item.clone())); break;
            }
        }
    } else {
        let Some(item) = cannon.item.as_ref().filter(|s| s.count > 0 && s.wire_patch().is_none()) else { return false; };
        for (index,dest) in before.iter().enumerate() {
            if dest.as_ref().is_some_and(|s| s.wire_patch().is_some()) || !containers.can_place(hopper,index,item) { continue; }
            let max = containers.max_stack(item);
            if plan_between(snapshot(&cannon.item),snapshot(dest),1,|_,_| max,|_,_| true,|a,b| a.same_item_same_components(b)).ok().flatten().is_some() {
                choice = Some((index,item.clone())); break;
            }
        }
    }
    let Some((index,mut item)) = choice else { return false; };
    let (from,to) = if push { (hopper_pos,target) } else { (target,hopper_pos) };
    let mut cancel = false;
    game.native_inventory_move_item([from.0,from.1,from.2],[to.0,to.1,to.2],&item.item,&mut cancel);
    if cancel || !matches!(game.world.block_entities.get(&hopper_pos), Some(BlockEntity::Chest(slots)) if *slots == before)
        || crate::game::tconstruct_source_machine_runtime::cannon_at(game,target).is_none_or(|state| state.to_nbt() != machine_before) { return false; }
    let mut after = before.clone(); item.count = 1;
    if push {
        let Some(source) = after[index].as_mut() else { return false; };
        source.count -= 1; if source.count == 0 { after[index] = None; }
        if let Some(dest) = cannon.item.as_mut() { dest.count += 1; } else { cannon.item = Some(item); }
    } else {
        let Some(source) = cannon.item.as_mut() else { return false; };
        source.count -= 1; if source.count == 0 { cannon.item = None; }
        if let Some(dest) = after[index].as_mut() { dest.count += 1; } else { after[index] = Some(item); }
    }
    let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&hopper_pos) else { return false; };
    *slots = after;
    if !crate::game::tconstruct_source_machine_runtime::save_cannon(game,target,&cannon) {
        if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&hopper_pos) { *slots = before; }
        return false;
    }
    true
}

// The source sided inventory inserts only into INPUT and extracts only from
// OUTPUT. Stage both sides before saving either inventory.
fn casting_tank_transfer(game: &mut crate::game::Game<'_>, hopper: &View, target: (i32,i32,i32), push: bool) -> bool {
    use horizon_client::{containers::BlockEntity, pluto_inventory::{plan_between, StackSnapshot}};
    if hopper.kind != Kind::Hopper || hopper.parts.len() != 1 || hopper.len() != 5 { return false; }
    let hopper_pos = hopper.parts[0].0;
    let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get(&hopper_pos) else { return false; };
    if slots.len() != 5 { return false; }
    let before = slots.clone();
    let Some(mut state) = crate::game::tconstruct_source_machine_runtime::casting_tank_at(game, target) else { return false; };
    let before_machine = state.to_nbt();
    fn snapshot(value: &Option<horizon_items::items::ItemStack>) -> Option<StackSnapshot<'_, horizon_items::items::ItemStack>> {
        value.as_ref().filter(|s| s.count > 0).map(|item| StackSnapshot { item, count: item.count })
    }
    let mut choice = None;
    if push {
        if state.slots[0].is_some() || state.slots[1].is_some() { return false; }
        game.ensure_datapacks();
        let Some((fluids, _)) = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref().and_then(|v| v.as_ref().ok()) else { return false; };
        for (index, source) in before.iter().enumerate() {
            let Some(item) = source.as_ref().filter(|s| s.count > 0 && s.wire_patch().is_none()) else { continue; };
            if !crate::game::tconstruct_source_fluid_container::accepts_with_color(item, &|id| fluids.contains(id), game.lan.is_none() && game.lan_acting_uuid.is_none()) { continue; }
            if plan_between(snapshot(source), None, 1, |_, _| 1, |_, _| true, |a,b| a.same_item_same_components(b)).ok().flatten().is_some() {
                choice = Some((index, item.clone())); break;
            }
        }
    } else {
        let Some(item) = state.slots[1].as_ref().filter(|s| s.count > 0 && s.wire_patch().is_none()) else { return false; };
        let containers = game.containers();
        for (index, dest) in before.iter().enumerate() {
            if dest.as_ref().is_some_and(|s| s.wire_patch().is_some()) || !containers.can_place(hopper, index, item) { continue; }
            let max = containers.max_stack(item);
            if plan_between(snapshot(&state.slots[1]), snapshot(dest), 1, |_, _| max, |_, _| true, |a,b| a.same_item_same_components(b)).ok().flatten().is_some() {
                choice = Some((index, item.clone())); break;
            }
        }
    }
    let Some((index, mut item)) = choice else { return false; };
    let (from,to) = if push { (hopper_pos,target) } else { (target,hopper_pos) };
    let mut cancel = false;
    game.native_inventory_move_item([from.0,from.1,from.2], [to.0,to.1,to.2], &item.item, &mut cancel);
    if cancel { return false; }
    if !matches!(game.world.block_entities.get(&hopper_pos), Some(BlockEntity::Chest(slots)) if *slots == before)
        || crate::game::tconstruct_source_machine_runtime::casting_tank_at(game, target).is_none_or(|state| state.to_nbt() != before_machine) { return false; }
    let mut after = before.clone(); item.count = 1;
    if push {
        let Some(stack) = after[index].as_mut() else { return false; };
        stack.count -= 1; if stack.count == 0 { after[index] = None; }
        state.slots[0] = Some(item);
        let _ = crate::game::tconstruct_source_machine_runtime::process_casting_tank(game, &mut state);
    } else {
        let Some(stack) = state.slots[1].as_mut() else { return false; };
        stack.count -= 1; if stack.count == 0 { state.slots[1] = None; }
        if let Some(stack) = after[index].as_mut() { stack.count += 1; } else { after[index] = Some(item); }
    }
    let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&hopper_pos) else { return false; };
    *slots = after;
    if !crate::game::tconstruct_source_machine_runtime::save_casting_tank(game, target, &state) {
        if let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&hopper_pos) { *slots = before; }
        return false;
    }
    true
}
