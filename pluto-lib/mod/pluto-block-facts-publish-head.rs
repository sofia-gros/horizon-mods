// The game's resource tables are loaded by the time the world rest loads:
// publish the per-state gameplay facts from the game's own block table then,
// so the dig report can read them by block name during play.
fn pluto_block_facts_publish_head(resources: &Resources) {
    let states = resources.facts().states.iter().map(|state| (state.name.to_string(), crate::entry::pluto_block_facts::DigFacts {
        hardness: state.hardness, requires_tool: state.requires_tool, explosion_resistance: state.explosion_resistance, sturdy_full: state.sturdy_full,
    })).collect();
    crate::entry::pluto_block_facts::publish(states);
}
