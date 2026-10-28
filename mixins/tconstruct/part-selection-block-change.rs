fn clear_part_selection_on_block_change(world: &mut crate::world::World, x: i32, y: i32, z: i32, state: horizon_world::registry::BlockStateId) {
    if world.block(x, y, z) != state {
        world.chunks.__horizon_mixins_tconstruct_part_selections_mut().remove(&(x,y,z));
    }
}
