fn pluto_tooltip(mut value: Vec<(sunrise_client::text::Text, u32)>, stack: &sunrise_client::items::ItemStack) -> Vec<(sunrise_client::text::Text, u32)> {
    if let Ok(data) = crate::game::pluto_item_adapter::read(stack) {
        for line in pluto_item_api::tooltip(&stack.item, &data) {
            value.push((sunrise_client::text::Text::literal(line), sunrise_client::hud::rgba(170,170,170,255)));
        }
    }
    value
}
