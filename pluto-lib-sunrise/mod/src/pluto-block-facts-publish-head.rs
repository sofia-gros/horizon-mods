// The game's resource tables are loaded by the time the world rest loads:
// publish the per-state gameplay facts from the game's own block table then,
// so the dig report can read them by block name during play, and the
// placement facts (the class chain and the shape flags `canSurvive` and the
// obstruction check read) alongside them.
fn pluto_block_facts_publish_head(resources: &Resources) {
    let states = resources.facts().states.iter().map(|state| (state.name.to_string(), crate::pluto_block_facts::DigFacts {
        hardness: state.hardness, requires_tool: state.requires_tool, explosion_resistance: state.explosion_resistance, sturdy_full: state.sturdy_full,
    })).collect();
    crate::pluto_block_facts::publish(states);
    let placement = resources.facts().states.iter().map(|state| (state.name.to_string(), crate::pluto_block_facts::PlacementFacts {
        classes: std::sync::Arc::clone(&state.classes), full_collision: state.full_collision, solid_render: state.solid_render,
    })).collect();
    crate::pluto_block_facts::publish_placement(placement);
}
