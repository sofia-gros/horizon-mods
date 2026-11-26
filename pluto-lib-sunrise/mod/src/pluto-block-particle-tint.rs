// Override the common particle colour, not its sprite or constructor/RNG.
fn pluto_block_particle_tint(value:u32,game:&Game<'_>,state:BlockStateId,pos:[i32;3])->u32 {
    let registry=&game.pack.assets.registry;
    // Native TerrainParticle keeps ordinary grass-block dirt crumbs untinted.
    if registry.state(state).is_some_and(|s|s.name.as_bytes()==b"minecraft:grass_block") {return value;}
    #[cfg(test)]
    { sunrise_client::mesher::pluto_block_colors::color_in(registry,state,Some(pos),0).unwrap_or(value) }
    #[cfg(not(test))]
    { crate::pluto_block_colors::color_in(registry,state,Some(pos),0).unwrap_or(value) }
}
