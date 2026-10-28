fn clear_part_selections(chunks: &mut crate::chunk_map::ChunkMap) {
    chunks.__horizon_mixins_tconstruct_part_selections_mut().clear();
    *chunks.__horizon_mixins_tconstruct_recipe_snapshot_mut() = None;
    *chunks.__horizon_mixins_tconstruct_recipe_tag_snapshot_mut() = None;
    *chunks.__horizon_mixins_tconstruct_material_snapshot_mut() = None;
    *chunks.__horizon_mixins_tconstruct_material_tag_snapshot_mut() = None;
}
