fn component_durability_tooltip(game: &crate::game::Game<'_>, stack: &ItemStack, lines: &mut Vec<(Text, u32)>) {
    crate::game::pluto_component_display::component_durability_lines(stack, lines, game.settings.file.bool("advancedItemTooltips"));
}
