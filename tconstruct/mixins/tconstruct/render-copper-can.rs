fn copper_can_contents_gui(
    _value: (),
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    stack: &horizon_client::items::ItemStack,
    x: f32, y: f32,
) {
    let context = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
        .and_then(|value| value.as_ref().ok());
    let known = |id: &str| context.is_some_and(|(fluids, _)| fluids.contains(id));
    let (fluid, tag) = if let Some(fluid) = crate::game::tconstruct_source_fluid_container::can_display_fluid(stack, &known) {
        (fluid, None)
    } else if crate::game::tconstruct_source_fluid_container::can_display_empty_potion(stack, &known) {
        ("tconstruct:potion".to_owned(), None)
    } else if let Some(tag) = crate::game::tconstruct_source_fluid_container::can_display_potion_icon(stack, &known) {
        ("tconstruct:potion".to_owned(), Some(tag))
    } else { return; };
    let sprite_id = crate::game::tconstruct_source_melter_render::sprite(&fluid);
    let Some(sprite) = horizon_client::pluto_render::sprite(&game.gui.atlas, &sprite_id) else { return; };
    let color = if let Some(tag) = tag.as_ref() {
        let Some(color) = crate::game::tconstruct_source_melter_render::can_potion_color(tag) else { return; };
        color
    } else { crate::game::tconstruct_source_melter_render::color(&fluid) };
    // Opaque row spans of the MIT copper_can_fluid.png (16 x 16).
    // Crop the existing still sprite, preserving the base icon outside its mask.
    for (left, top, width) in [(6., 3., 4.), (5., 4., 6.), (5., 5., 6.), (6., 6., 4.)] {
        let _ = horizon_client::pluto_render::crop(batch, sprite,
            [left, top, width, 1.0], [16.0, 16.0], [x + left, y + top, width, 1.0], color);
    }
}
