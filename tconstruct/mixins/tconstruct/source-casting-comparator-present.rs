fn tconstruct_casting_comparator_present(level: &crate::game::GameLevel<'_>, state: horizon_world::registry::BlockStateId, output: &mut bool) {
    if &*level.facts.get(state).name == "tconstruct:seared_casting_tank" { *output = true; }
}
