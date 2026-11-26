// HEAD on entity_nbt::item_frame_from_nbt: a tag carrying a registered
// variant's `Variant` loads through the variant's setRotation instead of
// the vanilla % 8 the body applies (the original's FancyItemFrameEntity
// overrides setRotation, and the load goes through it). The body below is
// the host's own loader with that one line changed; tags without a known
// variant run the body untouched.
fn pluto_hanging_frame_from_nbt(tag: &Tag, id: u32) -> std::ops::ControlFlow<Option<ItemFrame>> {
    let variant = match tag.get("Variant") {
        Some(Tag::Int(value)) => *value,
        Some(Tag::Byte(value)) => i32::from(*value),
        _ => return std::ops::ControlFlow::Continue(()),
    };
    let Some(facts) = sunrise_client_lib::pluto_hanging_access::frame_for_variant(variant) else {
        return std::ops::ControlFlow::Continue(());
    };
    let Some(pos) = attached_block_pos(tag) else {
        return std::ops::ControlFlow::Break(None);
    };
    let saved = tag.get("ItemRotation").and_then(int_value).unwrap_or(0);
    let frame = ItemFrame {
        id,
        pos,
        facing: wrap_data_value(number(tag.get("Facing")).unwrap_or(0), 6),
        glow: kind(tag).as_deref() == Some("minecraft:glow_item_frame"),
        item: tag
            .get("Item")
            .and_then(item_from_nbt)
            .map(|s| sunrise_items::items::ItemStack { count: 1, ..s }),
        rotation: sunrise_client_lib::pluto_hanging_access::frame_rotation(&facts, saved) as i8,
        drop_chance: float(tag.get("ItemDropChance")).map_or(1.0, |v| v as f32),
        fixed: flag(tag, "Fixed"),
        invisible: flag(tag, "Invisible"),
        check_interval: 0,
        custom_name: tag.get("CustomName").cloned(),
        random: sunrise_client_render::random_source::entity(),
    };
    std::ops::ControlFlow::Break(Some(frame))
}
