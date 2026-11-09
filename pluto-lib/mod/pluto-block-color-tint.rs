fn tint(
    mesher: &Mesher<'_>, pad: &Padded, tints: &mut Tints, out: &mut SectionMesh,
    pos: [i32; 3], origin: [f32; 3], face: &BakedFace, ao: bool,
    state: (u32, &Info), world: BlockPos, interior_leaf: bool,
    original: impl FnOnce(&Mesher<'_>, &Padded, &mut Tints, &mut SectionMesh, [i32; 3], [f32; 3], &BakedFace, bool, (u32, &Info), BlockPos, bool),
) {
    let color = pluto_block_colors::color_in(
        mesher.assets.registry, BlockStateId(state.0),
        Some([world.x, world.y, world.z]), face.quad.tint_index,
    );
    if let Some(color) = color {
        // A face-local copy preserves every lighting/culling fact and leaves the shared table intact.
        // The native renderer still applies model colors, AO, connected textures and vertex packing.
        let mut info = *state.1;
        info.tints = [Tint::Constant(color); 2];
        info.tint_count = u8::MAX;
        original(mesher, pad, tints, out, pos, origin, face, ao, (state.0, &info), world, interior_leaf);
    } else {
        original(mesher, pad, tints, out, pos, origin, face, ao, state, world, interior_leaf);
    }
}
