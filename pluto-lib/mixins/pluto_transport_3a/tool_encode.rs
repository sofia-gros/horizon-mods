fn pluto_tool_encode(mut value: Option<Stack>, source: &Option<ItemStack>, registry: &impl Fn(&str, &str) -> Option<i32>) -> Option<Stack> {
    use horizon_protocol::item::Component;
    const TOOL: i32 = 28;
    let (Some(out), Some(data)) = (value.as_mut(), source.as_ref().and_then(|s| s.data())) else { return value };
    if out.wire.0.is_some() || out.added.iter().any(|(id, _)| *id == TOOL) || out.removed.contains(&TOOL) { return value; }
    let removed = data.other("!minecraft:tool").is_some();
    match (removed, data.other("minecraft:tool")) {
        (true, None) => out.removed.push(TOOL),
        (false, Some(tool)) => if let Some(bytes) = crate::pluto_tool_codec::encode(tool, registry) { out.added.push((TOOL, Component::Raw(bytes))); },
        _ => {}
    }
    value
}
