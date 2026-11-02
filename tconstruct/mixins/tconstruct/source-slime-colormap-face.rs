fn tconstruct_slime_colormap_face(value: Option<u32>, mesher: &Mesher<'_>, tints: &Tints<'_>, pos: [i32; 3], id: &u32, face: &BakedFace) -> Option<u32> {
    if face.quad.tint_index < 0 { return value; }
    let Some(state) = mesher.assets.registry.state(horizon_world::registry::BlockStateId(*id)) else { return value; };
    let [cx, cz, _] = tints.origin;
    let Some(x) = cx.checked_mul(16).and_then(|x| x.checked_add(pos[0])) else { return value; };
    let Some(z) = cz.checked_mul(16).and_then(|z| z.checked_add(pos[2])) else { return value; };
    crate::mesher::tconstruct_slime_colormap::color(&state.name, x, z).or(value)
}
