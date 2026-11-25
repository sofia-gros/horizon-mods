fn guide_lifetime_clear(chunks: &mut ChunkMap) {
    chunks
        .__horizon_mixins_pluto_guide_lifetime_experiment_session_mut()
        .invalidate();
}
