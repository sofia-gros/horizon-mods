// Concrete player landing response. Preserve native horizontal restitution,
// then let a registered block own vertical response without newer-version
// gravity/drag compensation. Unregistered blocks retain the complete native path.
use sunrise_client_lib::*;
use crate::pluto_block_access::{self as pluto_block, AfterFall, StateInfo};

pub(super) fn dispatch(world: &mut MotionWorld<'_, '_, '_>, state: BlockStateId, x: bool, z: bool, resolved: Vec3) -> Option<Result<()>> {
    if !world.player.collision.vertical || !world.player.collision.below { return None; }
    let definition = world.assets.registry.state(state)?;
    let info = StateInfo { block: definition.name.to_string(), properties: definition.properties.iter().map(|(k,v)|(k.to_string(),v.to_string())).collect() };
    // Clone a handler before native effects; registration cannot replace the
    // chosen callback between discovery and invocation, and no lock is held.
    let callback = (pluto_block::landing_sink()?.after_fall)(&info)?;
    Some((|| -> Result<()> {
        let input = RestitutionInput {
            block: state, collided_x: x, collided_z: z, vertical_collision: false,
            collision_below: world.player.collision.below, living: true,
            resolved_movement: resolved, fall_distance: world.player.fall_distance,
        };
        movement_restitution::restitute_movement(world, input)?;
        let position = world.support(sunrise_world::movement_support::MOVEMENT_SUPPORT_OFFSET)?;
        let velocity = world.player.velocity;
        let mut request = AfterFall {
            pos: [position.x,position.y,position.z], entity: world.player.entity_id,
            kind: "minecraft:player".into(), living: true, item: false,
            suppressing_bounce: world.player.shift_key_down,
            velocity: [velocity.x,velocity.y,velocity.z], fall_distance: world.player.fall_distance,
            on_ground: world.player.ground.on_ground,
        };
        callback(&mut request);
        world.player.velocity = Vec3::new(request.velocity[0],request.velocity[1],request.velocity[2]);
        world.player.fall_distance = request.fall_distance;
        world.player.ground.on_ground = request.on_ground;
        Ok(())
    })())
}
