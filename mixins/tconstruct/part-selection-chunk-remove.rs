fn clear_part_selection_on_unload(chunks: &mut crate::chunk_map::ChunkMap, chunk: &(i32,i32)) {
    chunks.__horizon_mixins_tconstruct_part_selections_mut().retain(|(x, _, z), _| (*x >> 4, *z >> 4) != *chunk);
}
