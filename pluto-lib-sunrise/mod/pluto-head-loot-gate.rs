// Gate only the native skull table. Never falsify the killer's powered data in
// the ordinary death loot context, nor consume another world's/random stream.
fn pluto_head_loot_gate(table:&str)->std::ops::ControlFlow<Vec<LootStack>> {
    if table=="minecraft:charged_creeper/root"
        && crate::mobs::nav::pluto_entity_spawn_state::head_drop_eligibility()==Some(false) {
        std::ops::ControlFlow::Break(Vec::new())
    } else {std::ops::ControlFlow::Continue(())}
}
