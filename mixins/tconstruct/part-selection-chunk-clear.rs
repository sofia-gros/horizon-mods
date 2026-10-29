fn clear_part_selections(chunks: &mut crate::chunk_map::ChunkMap) {
    chunks.__horizon_mixins_tconstruct_part_selections_mut().clear();
    chunks.__horizon_mixins_tconstruct_source_selection_origin_mut().clear();
    *chunks.__horizon_mixins_tconstruct_source_recipe_generation_mut() = 0;
    chunks.__horizon_mixins_tconstruct_builder_choices_mut().clear();
    chunks.__horizon_mixins_tconstruct_builder_preview_mut().clear();
    chunks.__horizon_mixins_tconstruct_builder_scroll_mut().clear();
    chunks.__horizon_mixins_tconstruct_builder_preview_sent_mut().clear();
    chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().clear();
    chunks.__horizon_mixins_tconstruct_material_wire_sent_mut().clear();
    *chunks.__horizon_mixins_tconstruct_material_wire_ids_mut() = None;
    chunks.__horizon_mixins_tconstruct_material_wire_before_mut().clear();
    *chunks.__horizon_mixins_tconstruct_material_wire_client_mut() = false;
    *chunks.__horizon_mixins_tconstruct_recipe_snapshot_mut() = None;
    *chunks.__horizon_mixins_tconstruct_recipe_tag_snapshot_mut() = None;
    *chunks.__horizon_mixins_tconstruct_material_snapshot_mut() = None;
    *chunks.__horizon_mixins_tconstruct_material_tag_snapshot_mut() = None;
    *chunks.__horizon_mixins_tconstruct_resolved_item_tags_mut() = None;
    *chunks.__horizon_mixins_tconstruct_fluid_tag_snapshot_mut() = None;
    *chunks.__horizon_mixins_tconstruct_resolved_fluid_tags_mut() = None;
    *chunks.__horizon_mixins_tconstruct_resolved_material_definitions_mut() = None;
    *chunks.__horizon_mixins_tconstruct_resolved_material_stats_mut() = None;
    chunks.__horizon_mixins_tconstruct_render_assets_mut().take();
    if let Ok(mut frames) = chunks.__horizon_mixins_tconstruct_render_sprite_frames().lock() { frames.clear(); }
    *chunks.__horizon_mixins_tconstruct_source_config_snapshot_mut() = None;
}
