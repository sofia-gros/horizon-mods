// The `face.layer as u16` expression of `Mesher::face`'s `own` tile: a
// retextured slot's face draws with its texture block's particle layer.
fn retexture_layer(value: u16, mesher: &Mesher<'_>, id: u32, world: BlockPos, face: &BakedFace) -> u16 {
    pluto_retexture::layer(value, mesher, id, world, face)
}
