fn tconstruct_machine_remove(level: &mut crate::game::GameLevel<'_>, pos: horizon_core::position::BlockPos, old: horizon_world::registry::BlockStateId, side_effects: bool) {
    let name = level.facts.get(old).name.clone();
    if let Some(capacity) = crate::game::tconstruct_source_smeltery_fuel::capacity(&name) {
        let kind = crate::game::tconstruct_source_machine_runtime::kind(&name);
        if side_effects {
            if let Some((_, tank)) = kind.and_then(|kind| crate::game::tconstruct_source_melter_store::read(level.world, (pos.x, pos.y, pos.z), kind)
                .and_then(|tag| crate::game::tconstruct_source_melter_state::Tank::from_nbt(&tag, capacity).map(|tank| (kind, tank)))) {
                if tank.amount > 0 {
                    let entry = level.pending.copied.entry([pos.x, pos.y, pos.z]).or_insert_with(|| (name.to_string(), horizon_world::nbt::Compound::new()));
                    entry.1.insert("tconstruct:internal_tank_copy".into(), tank.to_nbt());
                }
            }
        }
        crate::game::tconstruct_source_melter_store::clear(level.world, (pos.x, pos.y, pos.z));
    }
    if &*name == "tconstruct:smeltery_controller" {
        let controller = (pos.x, pos.y, pos.z);
        if let Some(state) = crate::game::tconstruct_source_melter_store::read(level.world, controller, "tconstruct:smeltery")
            .and_then(|tag| crate::game::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) {
            if side_effects {
                for stack in state.slots.into_iter().flatten() { level.drop_item_stack(pos, Some(stack)); }
            }
            // The chest is a projection, not a second inventory.
            if let Some(horizon_client::containers::BlockEntity::Chest(slots)) = level.world.block_entities.get_mut(&controller) { slots.clear(); }
            crate::game::tconstruct_source_melter_store::clear(level.world, controller);
        }
        level.world.chunks.__horizon_mixins_tconstruct_smeltery_owners_mut().retain(|_, owner| *owner != controller);
    }
    if matches!(&*name, "tconstruct:seared_melter" | "tconstruct:seared_heater" | "tconstruct:seared_faucet" | "tconstruct:seared_table" | "tconstruct:seared_basin") {
        crate::game::tconstruct_source_melter_store::clear(level.world, (pos.x, pos.y, pos.z));
    }
}
