fn guide_lifetime_clear(chunks: &mut ChunkMap) {
    crate::chunk_map::pluto_guide_lifetime_state::evict_session(
        std::ptr::from_ref(chunks) as usize,
    );
}
