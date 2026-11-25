fn pluto_tooltip_b2(stack: &ItemStack, lines: &mut Vec<(Text, u32)>) {
    if let Ok(data) = crate::game::pluto_item_adapter_b2::read(stack) {
        for line in pluto_api::item::tooltip(&stack.item, &data) {
            lines.push((Text::literal(line), 0xffaaaaaa));
        }
    }
}
