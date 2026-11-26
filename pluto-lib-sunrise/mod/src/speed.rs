fn component_tool_speed(value: f32, items: &Items, held: Option<&ItemStack>, block: &str) -> f32 {
    held.and_then(|stack| {
        let tool = stack.data()?.other("minecraft:tool")?;
        sunrise_world::pluto_tool_rules::evaluate_tool(tool, |spec| items.block_matches(spec, block))
    })
    .map_or(value, |result| result.speed)
}
