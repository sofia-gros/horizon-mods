fn clear_part_selections(chunks: &mut crate::chunk_map::ChunkMap) {
    chunks.__horizon_mixins_tconstruct_part_selections_mut().clear();
    *chunks.__horizon_mixins_tconstruct_recipe_snapshot_mut() = None;
    *chunks.__horizon_mixins_tconstruct_recipe_tag_snapshot_mut() = None;
    *chunks.__horizon_mixins_tconstruct_material_snapshot_mut() = None;
    *chunks.__horizon_mixins_tconstruct_material_tag_snapshot_mut() = None;
    *chunks.__horizon_mixins_tconstruct_resolved_item_tags_mut() = None;
    *chunks.__horizon_mixins_tconstruct_fluid_tag_snapshot_mut() = None;
    *chunks.__horizon_mixins_tconstruct_resolved_fluid_tags_mut() = None;
    *chunks.__horizon_mixins_tconstruct_resolved_material_definitions_mut() = None;
    *chunks.__horizon_mixins_tconstruct_source_config_snapshot_mut() = None;
}
