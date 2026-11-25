fn component_tool_drops(value: bool, items: &Items, held: Option<&ItemStack>, block: &str) -> bool {
    held.and_then(|stack| {
        pluto_tool_rules::evaluate_stack(stack, |spec| items.block_matches(spec, block))
    })
    .map_or(value, |result| result.correct_for_drops)
}
