fn copper_can_contents_gui(
    _value: (),
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    stack: &horizon_client::items::ItemStack,
    x: f32, y: f32,
) {
    let context = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
        .and_then(|value| value.as_ref().ok());
    let Some(fluid) = crate::game::tconstruct_source_fluid_container::can_display_fluid(stack,
        &|id| context.is_some_and(|(fluids, _)| fluids.contains(id))) else { return; };
    let sprite_id = crate::game::tconstruct_source_melter_render::sprite(&fluid);
    let Some(sprite) = game.gui.atlas.sprites.get(&sprite_id).copied() else { return; };
    // Horizon default water RGB 0x3f76e4, packed as HUD ABGR.
    let color = if crate::game::tconstruct_source_fluid_display::display_id(&fluid) == "minecraft:water" { 0xffe4763f }
        else { crate::game::tconstruct_source_melter_render::color(&fluid) };
    // Opaque row spans of the MIT copper_can_fluid.png (16 x 16).
    // Crop the existing still sprite, preserving the base icon outside its mask.
    for (left, top, width) in [(6., 3., 4.), (5., 4., 6.), (5., 5., 6.), (6., 6., 4.)] {
        let mut clipped = sprite;
        let du = (sprite.u1 - sprite.u0) / 16.0;
        let dv = (sprite.v1 - sprite.v0) / 16.0;
        clipped.u0 = sprite.u0 + left * du;
        clipped.u1 = clipped.u0 + width * du;
        clipped.v0 = sprite.v0 + top * dv;
        clipped.v1 = clipped.v0 + dv;
        batch.rect_uv(x + left, y + top, width, 1.0, clipped, color);
    }
}
