// Registered fallOn damage through the concrete resident-player boundary.
// Preserve native block-contact listeners exactly once, before PlutoLib.
use super::*;
use sunrise_world::level::pluto_block_access::{self as pluto_block, FallOn, StateInfo};

pub(super) fn dispatch(world: &MotionWorld<'_, '_, '_>, state: BlockStateId, position: BlockPos, distance: f64, damage: f32) -> Option<f32> {
    let sink = pluto_block::landing_sink()?;
    let definition = world.assets.registry.state(state)?;
    if !(sink.has)(&definition.name) { return None; }
    let damage = crate::block_contact::fall(crate::block_contact::Contact {
        entity: world.player.entity_id, kind: "minecraft:player", block: definition.name.as_str(), state, position,
    }, distance, damage);
    let mut request = FallOn {
        state: StateInfo { block: definition.name.to_string(), properties: definition.properties.iter().map(|(k,v)|(k.to_string(),v.to_string())).collect() },
        pos: [position.x,position.y,position.z], entity: world.player.entity_id,
        kind: "minecraft:player".into(), distance, damage,
    };
    (sink.fall_on)(&mut request);
    Some(if request.damage.is_finite() { request.damage.max(0.0) } else { damage })
}
