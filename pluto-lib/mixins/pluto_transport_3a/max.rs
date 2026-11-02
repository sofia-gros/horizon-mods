// Extend the host encoder without changing an existing component or cached patch.
fn add_max_damage(mut value: Option<Stack>, source: &Option<ItemStack>) -> Option<Stack> {
    use horizon_protocol::item::{component, Component};
    let (Some(out), Some(data)) = (
        value.as_mut(),
        source.as_ref().and_then(|stack| stack.data()),
    ) else { return value; };
    if out.wire.0.is_some()
        || out.added.iter().any(|(id, _)| *id == component::MAX_DAMAGE)
        || out.removed.contains(&component::MAX_DAMAGE)
    { return value; }
    let removed = data.other("!minecraft:max_damage").is_some();
    let added = data.other("minecraft:max_damage");
    match (removed, added) {
        (true, None) => out.removed.push(component::MAX_DAMAGE),
        (false, Some(horizon_world::nbt::Tag::Int(n))) if *n > 0 => {
            out.added.push((component::MAX_DAMAGE, Component::Int(*n)));
        }
        _ => {} // Invalid or conflicting input is left to the host.
    }
    value
}
