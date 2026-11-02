fn pluto_tool_encode(
    mut value: Option<Stack>,
    source: &Option<ItemStack>,
    registry: &impl Fn(&str, &str) -> Option<i32>,
) -> Option<Stack> {
    use horizon_protocol::item::Component;
    const TOOL: i32 = 28;
    let (Some(out), Some(data)) = (value.as_mut(), source.as_ref().and_then(|s| s.data())) else {
        return value;
    };
    if out.wire.0.is_some()
        || out.added.iter().any(|(id, _)| *id == TOOL)
        || out.removed.contains(&TOOL)
    {
        return value;
    }
    if data.wire.patch().is_some()
        && source
            .as_ref()
            .and_then(|stack| stack.wire_patch())
            .is_none()
    {
        eprintln!("PlutoLib: modified cached item patch; unrecognised components may be lost");
    }
    let removed = data.other("!minecraft:tool").is_some();
    match (removed, data.other("minecraft:tool")) {
        (true, None) => out.removed.push(TOOL),
        (false, Some(tool)) => match crate::pluto_tool_codec::encode(tool, registry) {
            Some(bytes) => {
                if crate::pluto_tool_codec::canonicalize(tool, registry).as_ref() != Some(tool) {
                    eprintln!("PlutoLib: minecraft:tool is not canonical; stack identity may change after transfer");
                }
                out.added.push((TOOL, Component::Raw(bytes)));
            },
            None => eprintln!("PlutoLib: could not encode minecraft:tool; host may omit the component"),
        },
        (true, Some(_)) => eprintln!("PlutoLib: conflicting minecraft:tool addition and removal; host may omit the component"),
        _ => {}
    }
    value
}
