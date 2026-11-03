// Preserve a single supported custom potion color when the host emits the
// decoded PotionContents component. Opaque patches and other potion metadata
// are deliberately left to the host.
fn tconstruct_potion_color_encode(mut value: Option<Stack>, source: &Option<ItemStack>) -> Option<Stack> {
    use horizon_protocol::item::{component, Component};
    use horizon_world::nbt::Tag;
    let (Some(out), Some(input)) = (value.as_mut(), source.as_ref()) else { return value; };
    if out.wire.0.is_some() || input.wire_patch().is_some()
        || out.removed.contains(&component::POTION_CONTENTS)
        || !matches!(&*input.item, "minecraft:potion" | "minecraft:splash_potion" | "minecraft:lingering_potion") { return value; }
    let Some(data) = input.data() else { return value; };
    let [(key, Tag::Int(color))] = data.potion_extra.as_slice() else { return value; };
    if key != "custom_color" || data.potion.is_none() { return value; }
    let mut found = out.added.iter_mut().filter(|(id, _)| *id == component::POTION_CONTENTS);
    let Some((_, Component::Potion { color: target, rest, .. })) = found.next() else { return value; };
    if found.next().is_some() || target.is_some() || !rest.is_empty() { return value; }
    *target = Some(*color);
    value
}
