// Record the known local UI projection after successful source/live updates.
// A missed or failed recording never grants trust: the old snapshot must still
// equal the entire live view, and otherwise the destruction guard refuses it.
pub(super) fn record(world: &mut super::tconstruct_world::World, pos: (i32,i32,i32), offset: usize) -> bool {
    let Some(raw) = super::tconstruct_source_melter_store::read_outer(world, pos) else { return false; };
    let containers = world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers().get(&pos)
        .map(|(slots, _)| slots.clone()).unwrap_or([None, None]);
    let Some(horizon_client::containers::BlockEntity::Chest(live)) = world.block_entities.get(&pos) else { return false; };
    let Some(updated) = super::tconstruct_source_machine_destroy_guard::snapshot_outer(&raw, live, offset, &containers) else { return false; };
    super::tconstruct_source_melter_store::replace_outer(world, pos, &raw, updated)
}

/// Capture legacy save provenance before runtime resizing, melting or guest
/// changes alter authoritative inputs. A mismatch leaves all data untouched.
pub(super) fn capture_legacy(world: &mut super::tconstruct_world::World, pos: (i32,i32,i32)) -> bool {
    let Some(raw) = super::tconstruct_source_melter_store::read_outer(world, pos) else { return false; };
    let Some(horizon_client::containers::BlockEntity::Chest(live)) = world.block_entities.get(&pos) else { return false; };
    let Some(updated) = super::tconstruct_source_machine_destroy_guard::legacy_snapshot_outer(&raw, live) else { return false; };
    super::tconstruct_source_melter_store::replace_outer(world, pos, &raw, updated)
}
