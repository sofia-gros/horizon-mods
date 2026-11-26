fn pluto_stack_tooltip_return(value: Vec<(sunrise_client::text::Text, u32)>, game: &crate::game::Game<'_>, stack: &sunrise_items::items::ItemStack) -> Vec<(sunrise_client::text::Text, u32)> {
    let mut lines = value;
    crate::game::pluto_component_display::component_durability_lines(stack, &mut lines, game.settings.file.bool("advancedItemTooltips"));
    lines
}
