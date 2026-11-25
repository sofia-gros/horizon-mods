fn pluto_clear_advancement_tags(map: &mut crate::chunk_map::ChunkMap) {
    // Drop only this world's overlays and custom cache; native cache is untouched.
    map.__horizon_mixins_pluto_native_creative_advancement_tags_mut().clear();
}
