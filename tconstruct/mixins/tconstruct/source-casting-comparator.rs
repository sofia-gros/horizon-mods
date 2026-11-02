fn tconstruct_casting_comparator(level: &crate::game::GameLevel<'_>, pos: horizon_core::position::BlockPos,
    state: horizon_world::registry::BlockStateId, output: &mut i32) {
    if &*level.facts.get(state).name != "tconstruct:seared_casting_tank" { return; }
    *output = crate::game::tconstruct_source_melter_store::read(level.world,(pos.x,pos.y,pos.z),"tconstruct:casting_tank")
        .and_then(|tag| crate::game::tconstruct_source_casting_tank::CastingTank::from_nbt(&tag)).map_or(0, |state| state.comparator());
}
