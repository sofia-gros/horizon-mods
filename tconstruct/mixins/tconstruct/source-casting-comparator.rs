fn tconstruct_casting_comparator(level: &crate::game::GameLevel<'_>, pos: horizon_core::position::BlockPos,
    state: horizon_world::registry::BlockStateId, output: &mut i32) {
    let name = &*level.facts.get(state).name;
    let key = (pos.x,pos.y,pos.z);
    match name {
        "tconstruct:seared_melter" => {
            *output = crate::game::tconstruct_source_melter_store::read(level.world,key,"tconstruct:melter")
                .and_then(|tag| crate::game::tconstruct_source_melter_state::Melter::from_nbt(&tag)).map_or(0, |state| state.comparator());
        }
        "tconstruct:seared_casting_tank" => {
            *output = crate::game::tconstruct_source_melter_store::read(level.world,key,"tconstruct:casting_tank")
                .and_then(|tag| crate::game::tconstruct_source_casting_tank::CastingTank::from_nbt(&tag)).map_or(0, |state| state.comparator());
        }
        "tconstruct:seared_fluid_cannon" => {
            *output = crate::game::tconstruct_source_melter_store::read(level.world,key,"tconstruct:fluid_cannon")
                .and_then(|tag| crate::game::tconstruct_source_fluid_cannon::Cannon::from_nbt(&tag)).map_or(0, |state| state.comparator());
        }
        _ => {
            if let Some(capacity) = crate::game::tconstruct_source_smeltery_fuel::capacity(name) {
                let key_name = if capacity == 4000 { "tconstruct:fuel_tank" } else { "tconstruct:ingot_tank" };
                *output = crate::game::tconstruct_source_melter_store::read(level.world,key,key_name)
                    .and_then(|raw| crate::game::tconstruct_source_melter_state::saved_comparator(key_name, &raw)).unwrap_or(0);
            }
        }
    }
}
