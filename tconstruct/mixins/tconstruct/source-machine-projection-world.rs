// Record the known local UI projection after successful source/live updates.
// Serialized Items are refreshed only after current-source equality. No
// historical snapshot or marker can authorize nonempty mismatched live data.
pub(super) fn record(world: &mut super::tconstruct_world::World, pos: (i32,i32,i32), offset: usize) -> bool {
    let Some(raw) = super::tconstruct_source_melter_store::read_outer(world, pos) else { return false; };
    let containers = world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers().get(&pos)
        .map(|(slots, _)| slots.clone()).unwrap_or([None, None]);
    let Some(horizon_client::containers::BlockEntity::Chest(live)) = world.block_entities.get(&pos) else { return false; };
    let Some(updated) = super::tconstruct_source_machine_destroy_guard::snapshot_outer(&raw, live, offset, &containers) else { return false; };
    super::tconstruct_source_melter_store::replace_outer(world, pos, &raw, updated)
}

/// Validate before a runtime/UI writer changes either source or live data.
/// Nonempty views always require current authoritative equality, not a marker.
pub(super) fn known(world: &super::tconstruct_world::World, pos: (i32,i32,i32)) -> bool {
    let Some(raw) = super::tconstruct_source_melter_store::read_outer(world, pos) else { return false; };
    let Some(horizon_client::containers::BlockEntity::Chest(live)) = world.block_entities.get(&pos) else { return false; };
    let offset = world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages().get(&pos).map(|page| page.1);
    let containers = world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers().get(&pos)
        .map(|(slots, _)| slots.clone()).unwrap_or([None, None]);
    super::tconstruct_source_machine_destroy_guard::current_projection(&raw, live, offset, &containers)
}
