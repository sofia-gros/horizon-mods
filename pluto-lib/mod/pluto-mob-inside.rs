// Adapted from pinned Horizon mobs/ai.rs (MIT; LICENSE.horizon).
// Keep native stepOn, swept visitation, species effects and random order.
use super::*;
use horizon_world::level::pluto_block_access::{self as transport, InsideMotion, StateInfo};

fn dispatch(m: &mut Mob, w: &MobWorld<'_>, pos: [i32; 3]) {
    let server = horizon_world::level::pluto_contact_effects::world_active(&w.world.attachments, w.game_time);
    if !server { return; }
    let Some(sink) = transport::landing_sink() else { return };
    let state = w.world.block(pos[0], pos[1], pos[2]);
    let Some(definition) = w.registry.state(state) else { return };
    let info = StateInfo { block: definition.name.to_string(), properties: definition.properties.iter()
        .map(|(key, value)| (key.to_string(), value.to_string())).collect() };
    if let Some(dimension) = crate::block_contact::current_level() {
        (sink.entity_inside)(&transport::EntityInside {
            dimension: dimension.to_string(), state: info.clone(), pos, entity: m.id as i32, kind: m.kind.id.clone(),
        });
    }
    let Some(callback) = (sink.inside_motion)(&info) else { return };
    let mut request = InsideMotion {
        pos, entity: m.id as i32, kind: m.kind.id.clone(), server,
        suppressing_bounce: false,
        position: [m.position.x, m.position.y, m.position.z],
        previous_position: [m.previous.x, m.previous.y, m.previous.z],
        velocity: [m.velocity.x, m.velocity.y, m.velocity.z],
        sync_velocity: false, step_sound: None,
    };
    callback(&mut request);
    if std::env::var_os("PLUTO_TRACE_INSIDE").is_some() {
        eprintln!("PLUTO-NONPLAYER-INSIDE entity={} kind={} block={} pos={:?} after={:?} marked={} sound={}",
            m.id, m.kind.id, info.block, pos, request.velocity, request.sync_velocity, request.step_sound.is_some());
    }
    m.velocity = Vec3::new(request.velocity[0], request.velocity[1], request.velocity[2]);
    if request.sync_velocity { m.needs_sync = true; }
    if let Some([volume, pitch]) = request.step_sound {
        let _ = horizon_world::level::pluto_contact_effects::queue_world(&w.world.attachments,
            m.id as i32, w.game_time,
            horizon_world::level::pluto_contact_effects::ContactSound { state, pos, volume, pitch });
    }
}

pub(super) fn effects(m: &mut Mob, w: &MobWorld<'_>, level_random: &mut LegacyRandom, events: &mut Vec<MobEvent>) {
    // Native hurt can teleport during stepOn or the scan. Keep this tick's
    // original swept path, while each hurt still takes effect immediately.
    let start = m.previous;
    let travel = m.position.subtract(start);
    if m.on_ground && m.alive() {
        let [x, y, z] = m.block_on_legacy(w);
        crate::block_contact::emit(crate::block_contact::Event::StepOn(crate::block_contact::Contact {
            entity: m.id as i32, kind: &m.kind.id, block: w.block_name(x, y, z),
            state: w.world.block(x, y, z), position: horizon_core::position::BlockPos::new(x, y, z),
        }));
        if w.block_name(x, y, z) == "minecraft:magma_block" {
            hurt(m, w, events,1.0, Damage::of(DamageKind::HotFloor));
        }
        if w.block_name(x, y, z) == "minecraft:turtle_egg"
            && !matches!(m.species(), Species::Turtle | Species::Bat)
            && w.mob_griefing && level_random.next_int_bound(100).unwrap_or(1) == 0 {
            events.push(MobEvent::CrackTurtleEgg { pos: [x, y, z] });
        }
    }
    let mut visited: Vec<[i32; 3]> = Vec::new();
    let longest = travel.x.abs().max(travel.y.abs()).max(travel.z.abs());
    let steps = ((longest / 0.2).ceil() as i32).clamp(1, 100);
    for step in 0..=steps {
        if !m.alive() { return; }
        let at = start.add_vector(travel.scale(f64::from(step) / f64::from(steps)));
        let b = m.bounds_at(at).deflate(Vec3::new(1e-5, 1e-5, 1e-5));
        for x in b.min_position().x.floor() as i32..=b.max_position().x.floor() as i32 {
            for y in b.min_position().y.floor() as i32..=b.max_position().y.floor() as i32 {
                for z in b.min_position().z.floor() as i32..=b.max_position().z.floor() as i32 {
                    let p = [x, y, z];
                    if visited.contains(&p) { continue; }
                    crate::block_contact::emit(crate::block_contact::Event::Inside(crate::block_contact::Contact {
                        entity: m.id as i32, kind: &m.kind.id, block: w.block_name(x, y, z),
                        state: w.world.block(x, y, z), position: horizon_core::position::BlockPos::new(x, y, z),
                    }));
                    dispatch(m, w, p);
                    visited.push(p);
                    if w.block_name(x, y, z) == "minecraft:powder_snow" {
                        if m.is_on_fire() && w.mob_griefing {
                            events.push(MobEvent::BreakBlock { pos: p, drop: false });
                        }
                        m.extinguish_fire();
                    }
                    match w.block_name(x, y, z) {
                        "minecraft:cactus" => { hurt(m, w, events,1.0, Damage::of(DamageKind::Cactus)); }
                        name @ ("minecraft:campfire" | "minecraft:soul_campfire")
                            if w.prop(x, y, z, "lit") == Some("true") => {
                            let damage = if name == "minecraft:campfire" { 1.0 } else { 2.0 };
                            hurt(m, w, events,damage, Damage::of(DamageKind::Campfire));
                        }
                        "minecraft:cobweb" if !matches!(m.species(), Species::Spider | Species::CaveSpider | Species::Wither) => {
                            m.fall_distance = 0.0;
                            m.stuck_multiplier = if m.has_effect("minecraft:weaving") {
                                Vec3::new(0.5, 0.25, 0.5)
                            } else { Vec3::new(0.25, f64::from(0.05_f32), 0.25) };
                        }
                        "minecraft:sweet_berry_bush" if !matches!(m.species(), Species::Fox | Species::Bee) => {
                            if m.species() != Species::Wither {
                                m.fall_distance = 0.0;
                                m.stuck_multiplier = Vec3::new(f64::from(0.8_f32), 0.75, f64::from(0.8_f32));
                            }
                            let moved = m.previous.subtract(m.position);
                            if w.prop(x, y, z, "age") != Some("0") && moved.horizontal_distance_squared() > 0.0
                                && (moved.x.abs() >= f64::from(0.003_f32) || moved.z.abs() >= f64::from(0.003_f32)) {
                                hurt(m, w, events,1.0, Damage::of(DamageKind::SweetBerryBush));
                            }
                        }
                        "minecraft:powder_snow" if w.block_name(m.position.x.floor() as i32,
                            m.position.y.floor() as i32, m.position.z.floor() as i32) == "minecraft:powder_snow" => {
                            if m.species() != Species::Wither {
                                m.fall_distance = 0.0;
                                m.stuck_multiplier = Vec3::new(f64::from(0.9_f32), 1.5, f64::from(0.9_f32));
                            }
                            let known_movement = w.controlling_player_movement.unwrap_or(m.velocity);
                            if let Some(velocity) = powder_snow_particle_velocity(known_movement, level_random) {
                                events.push(MobEvent::PowderSnowParticle {
                                    at: [m.position.x, f64::from(y + 1), m.position.z], velocity,
                                });
                            }
                        }
                        "minecraft:honey_block" => honey_slide(m, p, level_random, events),
                        _ => continue,
                    }
                }
            }
        }
    }
}
