fn clear_part_selection_on_block_change(world: &mut crate::world::World, x: i32, y: i32, z: i32, state: horizon_world::registry::BlockStateId) {
    if world.block(x, y, z) != state && *world.chunks.__horizon_mixins_tconstruct_material_wire_client()
        && (crate::chunk::MIN_Y..crate::chunk::MIN_Y + crate::chunk::SECTIONS as i32 * 16).contains(&y) {
        // Only a remote client's raw entity is cleared here. The server's
        // gameplay removal must first capture the entity for loot.
        if let Some(chunk) = world.chunks.get_mut(&(x >> 4, z >> 4)) {
            let chunk = std::sync::Arc::make_mut(chunk);
            if let Some(saved) = chunk.saved.as_mut() {
                use horizon_world::nbt::Tag;
                std::sync::Arc::make_mut(saved).block_entities.retain(|tag| {
                    !(tag.get("id") == Some(&Tag::String("tconstruct:material_block".into()))
                        && tag.get("x") == Some(&Tag::Int(x)) && tag.get("y") == Some(&Tag::Int(y))
                        && tag.get("z") == Some(&Tag::Int(z)))
                });
            }
        }
    }
    if world.block(x, y, z) != state {
        world.chunks.__horizon_mixins_tconstruct_part_selections_mut().remove(&(x,y,z));
        world.chunks.__horizon_mixins_tconstruct_builder_choices_mut().remove(&(x,y,z));
        world.chunks.__horizon_mixins_tconstruct_builder_wire_recv_mut().remove(&(x,y,z));
    }
}
