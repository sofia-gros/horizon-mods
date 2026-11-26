// Adapted from pinned Horizon mobs/living.rs (MIT; LICENSE.horizon).
// Keep native collision/fall ordering; only registered post-fall callbacks own Y.
use super::*;
use sunrise_world::level::pluto_block_access::{self as transport, AfterFall, FallOn, StateInfo};

fn state_info(w: &MobWorld<'_>, state: BlockStateId) -> Option<StateInfo> {
    let definition = w.registry.state(state)?;
    Some(StateInfo {
        block: definition.name.to_string(),
        properties: definition.properties.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    })
}

pub(super) fn fall(mob: &Mob, contact: Option<(&MobWorld<'_>, [i32; 3])>, distance: f64, damage: f32) -> Option<f32> {
    let (w, pos) = contact?;
    let sink = transport::landing_sink()?;
    let state = w.world.block(pos[0], pos[1], pos[2]);
    let info = state_info(w, state)?;
    if !(sink.has)(&info.block) { return None; }
    let damage = crate::block_contact::fall(crate::block_contact::Contact {
        entity: mob.id as i32, kind: &mob.kind.id, block: &info.block, state,
        position: BlockPos::new(pos[0], pos[1], pos[2]),
    }, distance, damage);
    let mut request = FallOn {
        state: info, pos, entity: mob.id as i32, kind: mob.kind.id.to_string(), distance, damage,
    };
    (sink.fall_on)(&mut request);
    Some(if request.damage.is_finite() { request.damage.max(0.0) } else { damage })
}

pub(super) fn restitute(mob: &mut Mob, w: &MobWorld<'_>, wanted: Vec3, moved: Vec3) {
    // Native do_move has already resolved entity/powder-snow collisions,
    // supporting blocks and species fall damage. Only restitution is extended.
    let differs = |a: f64, b: f64| (b - a).abs() >= f64::from(1.0e-5_f32);
    let x_hit = differs(wanted.x, moved.x);
    let z_hit = differs(wanted.z, moved.z);
    let moved_vertically = wanted.y.abs() > 0.0;
    let vertical = wanted.y != moved.y;
    if moved_vertically && vertical || x_hit || z_hit {
        let pos = [mob.position.x.floor() as i32, (mob.position.y - f64::from(0.2_f32)).floor() as i32, mob.position.z.floor() as i32];
        let state = w.world.block(pos[0], pos[1], pos[2]);
        // Capture before native restitution and release the registry lock.
        let callback = if mob.vertical_collision_below {
            transport::landing_sink().and_then(|sink| state_info(w, state).and_then(|info| (sink.after_fall)(&info)))
        } else { None };
        let input = RestitutionInput {
            block: state, collided_x: x_hit, collided_z: z_hit,
            vertical_collision: vertical && callback.is_none(), collision_below: mob.vertical_collision_below,
            living: true, resolved_movement: moved, fall_distance: mob.fall_distance,
        };
        let _ = restitute_movement(&mut MobRestitution { mob, world: w.world, movement: w.movement }, input);
        if let Some(callback) = callback {
            let velocity = mob.velocity;
            let mut request = AfterFall {
                pos, entity: mob.id as i32, kind: mob.kind.id.to_string(), living: true, item: false,
                suppressing_bounce: false, velocity: [velocity.x, velocity.y, velocity.z],
                fall_distance: mob.fall_distance, on_ground: mob.on_ground,
            };
            callback(&mut request);
            mob.velocity = Vec3::new(request.velocity[0], request.velocity[1], request.velocity[2]);
            mob.fall_distance = request.fall_distance;
            mob.on_ground = request.on_ground;
        }
    }
}
