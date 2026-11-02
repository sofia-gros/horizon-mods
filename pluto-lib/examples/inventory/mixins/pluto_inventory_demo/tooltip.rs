fn inventory_demo_tooltip(stack: &ItemStack, lines: &mut Vec<(Text, u32)>) {
    if stack.item.as_ref() == "minecraft:stick" && crate::game::pluto_inventory_demo::verified() {
        lines.push((Text::literal("Inventory planner: verified"), 0xff55ff55));
    }
}
