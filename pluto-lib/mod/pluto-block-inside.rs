// Called directly at the pinned native entityInside seam, after ServerStuck's
// early exit. Horizon retains the complete sweep, pass modes and vanilla effects.
use super::*;
use horizon_world::level::pluto_block_access::{self as transport, StateInfo, InsideMotion};

pub(super) fn dispatch(world: &mut MotionWorld<'_, '_, '_>, state: BlockStateId, pos: BlockPos) {
    let Some(sink) = transport::landing_sink() else { return };
    let Some(definition) = world.assets.registry.state(state) else { return };
    let info = StateInfo { block: definition.name.to_string(), properties: definition.properties.iter().map(|(k,v)|(k.to_string(),v.to_string())).collect() };
    if !world.player.client_side {
        if let Some(dimension) = crate::block_contact::current_level() {
            let bounds = world.player.bounds();
            (sink.entity_inside)(&transport::EntityInside {
                dimension: dimension.to_string(), state: info.clone(), pos: [pos.x, pos.y, pos.z], entity: world.player.entity_id, kind: "minecraft:player".into(),
                bounds: [bounds.min_position().x, bounds.min_position().y, bounds.min_position().z,
                    bounds.max_position().x, bounds.max_position().y, bounds.max_position().z],
                fall_distance: world.player.fall_distance,
            });
        }
    }
    let Some(callback) = (sink.inside_motion)(&info) else { return };
    let player = &mut world.player;
    let mut request = InsideMotion {
        pos: [pos.x,pos.y,pos.z], entity: player.entity_id, kind: "minecraft:player".into(),
        server: !player.client_side, suppressing_bounce: player.shift_key_down,
        position: [player.position.x,player.position.y,player.position.z],
        previous_position: [player.old_position.x,player.old_position.y,player.old_position.z],
        velocity: [player.velocity.x,player.velocity.y,player.velocity.z], sync_velocity: false, step_sound: None,
    };
    let before = request.velocity;
    callback(&mut request);
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *TRACE.get_or_init(|| std::env::var_os("PLUTO_TRACE_INSIDE").is_some()) {
        eprintln!("PLUTO-INSIDE block={} server={} suppress={} position={:?} previous={:?} before={:?} after={:?}",
            info.block, request.server, request.suppressing_bounce, request.position, request.previous_position, before, request.velocity);
    }
    player.velocity = Vec3::new(request.velocity[0],request.velocity[1],request.velocity[2]);
    if !player.client_side && request.sync_velocity { player.health.sync_velocity = true; }
    if !player.client_side {
        if let Some([volume, pitch]) = request.step_sound {
            let _ = horizon_world::level::pluto_contact_effects::queue_player(&mut world.scene.attachments, player.entity_id, player.health.tick_count,
                horizon_world::level::pluto_contact_effects::ContactSound { state, pos: request.pos, volume, pitch });
        }
    }
}
