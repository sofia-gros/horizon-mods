fn tconstruct_warping_clear(chunks: &mut crate::chunk_map::ChunkMap) {
    chunks
        .__horizon_mixins_tconstruct_warping_draw_mut()
        .clear();
    chunks
        .__horizon_mixins_tconstruct_warping_teleport_mut()
        .clear();
}
