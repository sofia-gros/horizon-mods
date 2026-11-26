fn component_tool_drops(value: bool, items: &Items, held: Option<&ItemStack>, block: &str) -> bool {
    held.and_then(|stack| {
        let tool = stack.data()?.other("minecraft:tool")?;
        sunrise_world::pluto_tool_rules::evaluate_tool(tool, |spec| items.block_matches(spec, block))
    })
    .map_or(value, |result| result.correct_for_drops)
}
