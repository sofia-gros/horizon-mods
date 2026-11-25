// Same-tick authoritative effects flush. guest_server_tick calls this phase
// with the guest's own level/player installed, after swept block contacts.
// The plan guard pins both the phase and its caller; do not move the flush to
// a client frame or infer a dimension from a global entity id.
use super::*;
use crate::game::entity::Entity as _;

pub(crate) fn begin_mobs(game: &mut Game<'_>) {
    if game.native_authoritative() {
        let tick = game.world.game_time;
        let _ = sunrise_world::level::pluto_contact_effects::begin_world(&mut game.world.attachments, tick);
    }
}

pub(crate) fn flush_world(game: &mut Game<'_>) {
    if !game.native_authoritative() { return; }
    let tick = game.world.game_time;
    let sounds = sunrise_world::level::pluto_contact_effects::drain_world(&game.world.attachments, tick);
    for (entity, request) in sounds {
        let sound = game.res.facts().get(request.state).sounds.step.clone();
        let at = [f64::from(request.pos[0])+0.5, f64::from(request.pos[1])+0.5, f64::from(request.pos[2])+0.5];
        let accepted = game.with_level(|_, level, _| level.native_play_sound(&sound, at, 4, request.volume, request.pitch));
        if std::env::var_os("PLUTO_TRACE_INSIDE").is_some() {
            eprintln!("PLUTO-WORLD-CONTACT-SOUND entity={entity} tick={tick} sound={sound} at={at:?} volume={} pitch={} accepted={accepted}", request.volume, request.pitch);
        }
    }
}

pub(crate) fn item_tick(game: &mut Game<'_>, item: &mut sunrise_client::entities::ItemEntity,
    escape_speed: Option<f32>, entity_boxes: &[sunrise_core::geometry::Aabb], client: bool,
    splash: &mut dyn FnMut(sunrise_client::entities::ItemSplash)) -> (bool, bool) {
    let server = !client && game.native_authoritative();
    let tick = game.world.game_time;
    let owns_frame = server && sunrise_world::level::pluto_contact_effects::begin_world(&mut game.world.attachments, tick);
    let mut forward_splash = |event| splash(event);
    let no_physics = escape_speed.is_some();
    // Retain the pinned native fluid, escape-speed, lava/fall and splash path.
    let step = game.item_tables().move_item(item, escape_speed, entity_boxes, &mut forward_splash);
    if let Some(pos) = step.vertical_contact {
        let state = game.world.block(pos.x, pos.y, pos.z);
        let position = item.position;
        let mut discarded = false;
        game.native_item_after_vertical_collision(game.dim.id(), client, item.id, state,
            [pos.x, pos.y, pos.z], [position.x, position.y, position.z], &mut item.stack, &mut discarded);
        if discarded || item.stack.count == 0 {
            if owns_frame { flush_world(game); }
            return (false, true);
        }
    }
    item.finish_move_block_effects(&game.world, game.res.facts(), no_physics, Some(&game.res.motion().blocks), step);
    item.push_by_bubble_column(&game.world, game.pack.assets.registry, game.res.facts(), game.collision);
    let alive = sunrise_client::entities::pluto_item_inside::settle(item, &game.world,
        game.res.facts(), no_physics, step, game.pack.assets.registry, server);
    let before = item.velocity;
    let (water, _) = item.push_by_fluid(&game.world, &game.res.motion().fluids);
    item.water_entry(water > 0.0, false, before, &mut forward_splash);
    if owns_frame { flush_world(game); }
    (alive, false)
}

pub(super) fn flush(game: &mut Game<'_>) {
    if !game.native_authoritative() { return; }
    let Some((entity, tick)) = game.player.as_ref().map(|p| (p.entity_id, p.health.tick_count)) else { return };
    let sounds = sunrise_world::level::pluto_contact_effects::drain_player(&game.world.attachments, entity, tick);
    for request in sounds {
        let sound = game.res.facts().get(request.state).sounds.step.clone();
        let at = [f64::from(request.pos[0])+0.5, f64::from(request.pos[1])+0.5, f64::from(request.pos[2])+0.5];
        let accepted = game.with_level(|_, level, _| level.native_play_sound(&sound, at, 4, request.volume, request.pitch));
        if std::env::var_os("PLUTO_TRACE_INSIDE").is_some() {
            eprintln!("PLUTO-CONTACT-SOUND entity={entity} sound={sound} at={at:?} volume={} pitch={} accepted={accepted}", request.volume, request.pitch);
        }
    }
}
