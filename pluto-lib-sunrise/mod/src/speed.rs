fn component_tool_speed(value: f32, items: &Items, held: Option<&ItemStack>, block: &str) -> f32 {
    held.and_then(|stack| {
        pluto_tool_rules::evaluate_stack(stack, |spec| items.block_matches(spec, block))
    })
    .map_or(value, |result| result.speed)
}
