fn tint(value: Option<u32>, mesher: &Mesher<'_>, id: u32, world: BlockPos, face: &BakedFace) -> Option<u32> {
    pluto_block_colors::color_in(
        mesher.assets.registry, BlockStateId(id),
        Some([world.x, world.y, world.z]), face.quad.tint_index,
    ).or(value)
}
