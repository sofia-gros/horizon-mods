// Tint the completed render state after baking (including CIT). Only its
// layers change; display, animation, bounds policy and glint/HDR fields survive.
fn pluto_item_color_layers(mut value: ItemRenderState, renderer: &ItemRenderer, stack: &StackView) -> ItemRenderState {
    value.layers = pluto_item_colors::tint(renderer, stack, value.layers);
    value
}
