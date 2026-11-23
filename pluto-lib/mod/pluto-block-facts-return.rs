fn pluto_block_facts_return(value: std::result::Result<GameplayTables, String>) -> std::result::Result<GameplayTables, String> {
    if let Ok(tables) = &value {
        let states = tables.facts.states.iter().map(|state| (state.name.to_string(), crate::entry::pluto_block_facts::DigFacts {
            hardness: state.hardness, requires_tool: state.requires_tool, explosion_resistance: state.explosion_resistance, sturdy_full: state.sturdy_full,
        })).collect();
        crate::entry::pluto_block_facts::publish(states);
    }
    value
}
