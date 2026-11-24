// The `own` tile of `Mesher::face`, at its store: a retextured slot's
// face draws with its texture block's particle layer.
fn retexture_layer(value: TileLayer, mesher: &Mesher<'_>, id: u32, world: BlockPos, face: &BakedFace) -> TileLayer {
    match value {
        TileLayer::Layer { layer, translucent, opaque } => TileLayer::Layer {
            layer: pluto_retexture::layer(layer, mesher, id, world, face),
            translucent,
            opaque,
        },
        other => other,
    }
}
