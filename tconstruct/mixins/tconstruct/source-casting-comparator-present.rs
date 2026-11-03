fn tconstruct_casting_comparator_present(level: &crate::game::GameLevel<'_>, state: horizon_world::registry::BlockStateId, output: &mut bool) {
    if matches!(&*level.facts.get(state).name, "tconstruct:seared_casting_tank" | "tconstruct:seared_fluid_cannon") { *output = true; }
}
