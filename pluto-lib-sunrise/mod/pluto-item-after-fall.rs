fn pluto_item_after_fall(value: f64, item: &mut crate::entities::ItemEntity, world: &crate::world::World, collision: &sunrise_world::block_collision::BlockCollisionRegistry<'_>, contact: Option<horizon_core::position::BlockPos>) -> f64 {
    // Wrap only vertical restitution. The caller retains collision, native
    // contact/discard handlers, fluid/block effects, drag and tracker ordering.
    use sunrise_world::level::pluto_block_access::{self as transport, AfterFall, StateInfo};
    let callback = (|| {
        if !item.on_ground { return None; }
        let sink = transport::landing_sink()?;
        // The native support query covers the whole bounding box, including
        // edge landings whose center lies outside the supporting block.
        let contact = contact?;
        let pos = [contact.x, contact.y, contact.z];
        let state = world.block(pos[0], pos[1], pos[2]);
        let definition = collision.registry().state(state)?;
        let info = StateInfo {
            block: definition.name.to_string(),
            properties: definition.properties.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        };
        Some(((sink.after_fall)(&info)?, pos))
    })();
    let Some((callback, pos)) = callback else { return value };
    let velocity = item.velocity;
    let mut request = AfterFall {
        pos, entity: item.id as i32, kind: "minecraft:item".into(), living: false, item: true,
        suppressing_bounce: false, velocity: [velocity.x, velocity.y, velocity.z],
        // Native items do not accumulate a landing-damage fall distance.
        fall_distance: 0.0, on_ground: item.on_ground,
    };
    callback(&mut request);
    item.velocity.x = request.velocity[0];
    item.velocity.z = request.velocity[2];
    item.on_ground = request.on_ground;
    request.velocity[1]
}
