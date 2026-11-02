fn pluto_tooltip_b2(stack: &ItemStack, lines: &mut Vec<(Text, u32)>) {
    if let Ok(data) = crate::game::pluto_item_adapter_b2::read(stack) {
        for line in pluto_item_api_b2::tooltip(&stack.item, &data) {
            lines.push((Text::literal(line), 0xffaaaaaa));
        }
    }
}
