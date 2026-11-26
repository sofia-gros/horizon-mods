// Adapted from pinned Horizon entities.rs::ItemEntity::tick_settle (MIT).
// The owner retains fluid refresh, contact consumption and tracker order.
use sunrise_client_lib::*;
use crate::pluto_block_access::{self as transport, InsideMotion, StateInfo};

fn dispatch(item: &mut ItemEntity, world: &World, registry: &sunrise_world::registry::BlockRegistry,
    pos: sunrise_core::position::BlockPos, state: sunrise_world::registry::BlockStateId, server: bool) {
    if !server { return; }
    let Some(sink) = transport::landing_sink() else { return };
    let Some(definition) = registry.state(state) else { return };
    let info = StateInfo { block: definition.name.to_string(), properties: definition.properties.iter()
        .map(|(key, value)| (key.to_string(), value.to_string())).collect() };
    if let Some(dimension) = sunrise_client::block_contact::current_level() {
        let bounds = item.bounds();
        (sink.entity_inside)(&transport::EntityInside {
            dimension: dimension.to_string(), state: info.clone(), pos: [pos.x, pos.y, pos.z], entity: item.id as i32, kind: "minecraft:item".into(),
            bounds: [bounds.min_position().x, bounds.min_position().y, bounds.min_position().z,
                bounds.max_position().x, bounds.max_position().y, bounds.max_position().z],
            fall_distance: item.fall_distance,
        });
    }
    let Some(callback) = (sink.inside_motion)(&info) else { return };
    let mut request = InsideMotion {
        pos: [pos.x, pos.y, pos.z], entity: item.id as i32, kind: "minecraft:item".into(),
        server, suppressing_bounce: false,
        position: [item.position.x, item.position.y, item.position.z],
        previous_position: [item.previous.x, item.previous.y, item.previous.z],
        velocity: [item.velocity.x, item.velocity.y, item.velocity.z], sync_velocity: false, step_sound: None,
    };
    callback(&mut request);
    if std::env::var_os("PLUTO_TRACE_INSIDE").is_some() {
        eprintln!("PLUTO-NONPLAYER-INSIDE entity={} kind=minecraft:item block={} pos={:?} after={:?} marked={} sound={}",
            item.id, info.block, request.pos, request.velocity, request.sync_velocity, request.step_sound.is_some());
    }
    item.velocity = Vec3::new(request.velocity[0], request.velocity[1], request.velocity[2]);
    if request.sync_velocity { item.needs_sync = true; }
    if let Some([volume, pitch]) = request.step_sound {
        let _ = crate::pluto_contact_effects::queue_world(&world.attachments,
            item.id as i32, world.game_time,
            crate::pluto_contact_effects::ContactSound { state, pos: request.pos, volume, pitch });
    }
}

pub fn settle(item: &mut ItemEntity, world: &World, facts: &BlockFacts, no_physics: bool,
    step: ItemStep, registry: &sunrise_world::registry::BlockRegistry, server: bool) -> bool {
    if step.did_move {
        if !no_physics { item.slime_step_on(world, facts); }
        let mut friction = 0.98_f32;
        if item.on_ground {
            let below = world.block(item.position.x.floor() as i32,
                (item.position.y - 0.999_999).floor() as i32, item.position.z.floor() as i32);
            friction *= facts.get(below).friction;
        }
        let friction = f64::from(friction);
        let air = f64::from(0.98_f32);
        item.velocity = Vec3::new(item.velocity.x * friction, item.velocity.y * air, item.velocity.z * friction);
        if item.on_ground && item.velocity.y < 0.0 { item.velocity.y *= -0.5; }
    }
    if !no_physics {
        if item.on_ground {
            let pos = BlockPos::containing(item.position.add_vector(Vec3::new(0.0, -f64::from(0.2_f32), 0.0)));
            let state = world.block(pos.x, pos.y, pos.z);
            sunrise_client::block_contact::emit(sunrise_client::block_contact::Event::StepOn(sunrise_client::block_contact::Contact {
                entity: item.id as i32, kind: "minecraft:item", block: &facts.get(state).name, state, position: pos,
            }));
        }
    }
    if item.velocity.subtract(step.old_velocity).length_squared() > 0.01 { item.needs_sync = true; }
    if item.age != i16::MIN as i32 { item.age += 1; }
    item.age < 6000 && item.position.y >= f64::from(world.shape.min_y - 64)
}
