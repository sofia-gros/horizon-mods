fn pluto_tool_decode(
    mut value: ItemStack,
    source: &Stack,
    builtin: &Builtin,
    synced: &Dynamic,
) -> ItemStack {
    use horizon_protocol::item::Component;
    const TOOL: i32 = 28;
    if source.removed.contains(&TOOL)
        || source.added.iter().filter(|(id, _)| *id == TOOL).count() != 1
    {
        return value;
    }
    let Some(Component::Raw(bytes)) = source.component(TOOL) else {
        return value;
    };
    let name = |id| {
        builtin
            .get("minecraft:block")
            .and_then(|ids| ids.name(id))
            .or_else(|| synced.name("minecraft:block", id))
            .map(str::to_owned)
    };
    let Some(tool) = crate::pluto_tool_codec::decode(bytes, &name) else {
        eprintln!("PlutoLib: could not decode minecraft:tool; the original wire patch is retained");
        return value;
    };
    value.data_mut().set_other("minecraft:tool", Some(tool));
    // Refresh the decoded snapshot while retaining the original patch bytes,
    // including unrelated components and removal markers.
    if let Some(patch) = &source.wire.0 {
        let snapshot = value.clone();
        value.data_mut().wire = crate::item_data::Wire::new(snapshot, patch.clone());
    }
    value
}
