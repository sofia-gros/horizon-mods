fn tconstruct_machine_decode(tag: &horizon_world::nbt::Tag) -> std::ops::ControlFlow<Option<((i32, i32, i32), crate::containers::BlockEntity)>> {
    use horizon_world::nbt::Tag;
    let Some(Tag::String(id)) = tag.get("id") else { return std::ops::ControlFlow::Continue(()); };
    if !matches!(id.to_string_lossy().as_str(), "tconstruct:melter" | "tconstruct:heater" | "tconstruct:faucet" | "tconstruct:table" | "tconstruct:basin" | "tconstruct:smeltery" | "tconstruct:fuel_tank" | "tconstruct:ingot_tank") {
        return std::ops::ControlFlow::Continue(());
    }
    let coord = |key| match tag.get(key) { Some(Tag::Int(n)) => Some(*n), _ => None };
    let (Some(x), Some(y), Some(z)) = (coord("x"), coord("y"), coord("z")) else { return std::ops::ControlFlow::Break(None); };
    std::ops::ControlFlow::Break(Some(((x, y, z), crate::containers::BlockEntity::Chest(decode_items(tag.get("Items"), 27)))))
}
