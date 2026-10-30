fn clear_part_selection_on_unload(chunks: &mut crate::chunk_map::ChunkMap, chunk: &(i32,i32)) {
    chunks.__horizon_mixins_tconstruct_material_wire_sent_mut().retain(|(_, _, pos), _| (pos[0] >> 4, pos[2] >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_source_selection_origin_mut().retain(|(x, _, z), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_part_selections_mut().retain(|(x, _, z), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_builder_material_cache_mut().retain(|(x, _, z), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_builder_recipe_context_mut().retain(|(x, _, z), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_builder_choices_mut().retain(|(x, _, z), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_builder_side_size_mut().retain(|(x, _, z), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_builder_double_partner_mut().retain(|(x, _, z), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_builder_preview_mut().retain(|(x, _, z), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_builder_scroll_mut().retain(|(x, _, z), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_builder_preview_sent_mut().retain(|(_, _, _, (x, _, z)), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_station_preview_sent_mut().retain(|(_, _, _, (x, _, z)), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_builder_cast_costs_mut().retain(|(x, _, z), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_builder_cast_wire_recv_mut().retain(|(x, _, z), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_builder_cast_wire_sent_mut().retain(|(_, _, _, (x, _, z)), _| (*x >> 4, *z >> 4) != *chunk);
    chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().retain(|(x, _, z), _| (*x >> 4, *z >> 4) != *chunk);
}
