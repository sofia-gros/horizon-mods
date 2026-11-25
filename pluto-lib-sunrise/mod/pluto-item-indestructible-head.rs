// Adapted from pinned MIT Horizon entities.rs::ItemEntity::environment_tick.
// License: pluto-lib/LICENSE.horizon. Unregistered queries run the original.
fn pluto_item_indestructible(entity: &mut ItemEntity) -> std::ops::ControlFlow<ItemEnvironmentTick> {
    // The stack's own custom data, as the item behaviour transport carries it (JSON).
    let data = match entity.stack.data().and_then(|d| d.other("minecraft:custom_data")) {
        Some(tag @ sunrise_world::nbt::Tag::Compound(_)) => sunrise_client_core::scoreboard::tag_to_json(tag),
        _ => serde_json::Value::Object(serde_json::Map::new()),
    };
    if sunrise_world::level::pluto_item_behavior_access::indestructible(&entity.stack.item, &data) != Some(true) {
        return std::ops::ControlFlow::Continue(());
    }
    // The original's indestructible item entity: an unbounded lifespan (the host's
    // unlimited-lifetime age, re-asserted every tick so every entry path — drop,
    // throw, load, network spawn — is covered) and invulnerable to everything the
    // environment tick applies (fire, lava, cactus), so the tick stills with the
    // entity alive and nothing spilled.
    entity.age = i16::MIN as i32;
    std::ops::ControlFlow::Break(ItemEnvironmentTick { alive: true, lava_hurt: false, spilled: Vec::new() })
}
