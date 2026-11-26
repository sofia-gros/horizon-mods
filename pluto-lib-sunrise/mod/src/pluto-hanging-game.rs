// Fancy hanging entities (PlutoLib): the game side of the hanging variant
// surface. The host keeps frames and stands in vanilla lists under the
// vanilla type ids, so a variant lives in the entity's kept raw NBT (the
// `Variant` int the original saves, plus `RotationTimer` for the rotating
// frames and `LeftHanded` for stands) and its behaviour is the registered
// variant's, read through the entry transport. Bodies below adapt the
// pinned host's own placement/interaction code (MIT, see
// pluto-lib/LICENSE.horizon) with the variant rules applied; anything the
// provider does not know keeps the host's vanilla path untouched.
use super::*;
use crate::pluto_hanging_access as access;
use sunrise_client::entities::{ArmorStand, ItemFrame};

use sunrise_client::items::ItemStack;
use sunrise_core::geometry::Vec3;

use sunrise_core::position::BlockPos;

use sunrise_core::random::RandomSource;
use sunrise_core::nbt::Tag;
use std::cell::RefCell;

use std::collections::HashMap;

/// The click sound a diamond frame's `setRotation` plays (`tconstruct:item_frame_click`).
const CLICK_SOUND: &str = "tconstruct:item_frame_click";

fn facing_of(face: [i32; 3]) -> u8 {
    match face {
        [0, -1, 0] => 0,
        [0, 1, 0] => 1,
        [0, 0, -1] => 2,
        [0, 0, 1] => 3,
        [-1, 0, 0] => 4,
        _ => 5,
    }
}

fn center(f: &ItemFrame) -> [f64; 3] {
    let (lo, hi) = f.bounds();
    std::array::from_fn(|i| (lo[i] + hi[i]) / 2.0)
}

fn sound(f: &ItemFrame, what: &str) -> String {
    let prefix = if f.glow { "glow_item_frame" } else { "item_frame" };
    format!("minecraft:entity.{prefix}.{what}")
}

/// An int in an entity's kept raw NBT (a byte reads as its value too).
fn raw_int(game: &Game<'_>, id: u32, key: &str) -> Option<i32> {
    let Some(Tag::Compound(map)) = game.persist.raw.get(&id) else {
        return None;
    };
    match map.get(&sunrise_core::nbt::JavaString::from(key)) {
        Some(Tag::Int(value)) => Some(*value),
        Some(Tag::Byte(value)) => Some(i32::from(*value)),
        _ => None,
    }
}

fn raw_set_int(game: &mut Game<'_>, id: u32, key: &str, value: i32) {
    if let Some(Tag::Compound(map)) = game.persist.raw.get_mut(&id) {
        map.insert(key.into(), Tag::Int(value));
    }
}

/// The frame's registered variant, from the `Variant` its raw NBT saves.
pub(super) fn frame_facts(game: &Game<'_>, f: &ItemFrame) -> Option<access::FrameFacts> {
    access::frame_for_variant(raw_int(game, f.id, "Variant")?)
}

/// The stand's registered variant, from the `Variant` its raw NBT saves.
pub(super) fn stand_facts(game: &Game<'_>, a: &ArmorStand) -> Option<access::StandFacts> {
    access::stand_for_variant(raw_int(game, a.id, "Variant")?)
}

// The comparator path (`GameLevel::item_frame_analog_output`) reads slices
// of the frame list with no access to the kept raw NBT, so the game side
// keeps the variant of every live fancy frame here, keyed by entity id.
// Placement and load note it; the frame tick rebuilds it from the raw NBT
// every tick, so a stale row survives less than a tick and only until the
// first tick after a world (re)load.
thread_local! {
    static FRAME_FACTS: RefCell<HashMap<u32, access::FrameFacts>> = RefCell::new(HashMap::new());
}

fn note_frame(id: u32, facts: &access::FrameFacts) {
    FRAME_FACTS.with(|facts_by_id| {
        facts_by_id.borrow_mut().insert(id, facts.clone());
    });
}

pub(super) fn known_frame(id: u32) -> Option<access::FrameFacts> {
    FRAME_FACTS.with(|facts_by_id| facts_by_id.borrow().get(&id).cloned())
}

fn refresh_frame_facts(game: &Game<'_>) {
    FRAME_FACTS.with(|facts_by_id| {
        let mut facts_by_id = facts_by_id.borrow_mut();
        facts_by_id.clear();
        for f in &game.item_frames {
            if let Some(facts) = frame_facts(game, f) {
                facts_by_id.insert(f.id, facts);
            }
        }
    });
}

/// `ItemFrame.survives`, adapted from the pinned host's private check (the
/// placement below needs it before the frame exists in the list).
fn frame_survives(game: &Game<'_>, f: &ItemFrame) -> bool {
    if f.fixed {
        return true;
    }
    let (lo, hi) = f.bounds();
    let bounds = sunrise_core::geometry::Aabb::new(
        Vec3::new(lo[0], lo[1], lo[2]),
        Vec3::new(hi[0], hi[1], hi[2]),
    );
    let mut scene = sunrise_client_render::collisions::SceneCollisions {
        world: &game.world,
        binding: game.collision,
        subject: None,
    };
    if !scene.unobstructed(bounds).unwrap_or(false) {
        return false;
    }
    let s = f.step();
    let behind = game.res.facts().get(game.world.block(
        f.pos[0] - s[0],
        f.pos[1] - s[1],
        f.pos[2] - s[2],
    ));
    let supported = behind.legacy_solid || (s[1] == 0 && behind.has_class("DiodeBlock"));
    supported && super::paintings::frame_coexists(f, &game.item_frames, &game.paintings)
}

/// `ItemFrame.setItem` / `setRotation`'s comparator update.
fn frame_output_changed(game: &mut Game<'_>, pos: [i32; 3]) {
    let at = BlockPos::new(pos[0], pos[1], pos[2]);
    game.with_level(|b, level, ticks| b.update_neighbour_for_output_signal(level, ticks, at));
}

/// A rotating variant's `setItem` side effect: the rotation and its timer
/// reset (the original's `setRotation(0, false)`, so no comparator update
/// beyond the one the caller is already making and no click sound).
fn reset_rotation(game: &mut Game<'_>, i: usize, facts: &access::FrameFacts) {
    if facts.rotation_step != 0 || facts.winds_down {
        game.item_frames[i].rotation = 0;
        let id = game.item_frames[i].id;
        raw_set_int(game, id, "RotationTimer", 0);
    }
}

/// `FancyItemFrameItem.useOn`: the host's `place_item_frame` body with the
/// variant taken from the item (the stack's own entity data never names a
/// variant; the configured rotation still goes through the variant's
/// `setRotation`, as the original's load does).
pub(super) fn place_fancy_frame(
    game: &mut Game<'_>,
    hit: &Hit,
    stack: &ItemStack,
    facts: &access::FrameFacts,
) -> bool {
    let pos: [i32; 3] = std::array::from_fn(|i| hit.pos[i] + hit.face[i]);
    if !(sunrise_client_render::chunk::MIN_Y..sunrise_client_render::chunk::MIN_Y + sunrise_client_render::chunk::HEIGHT)
        .contains(&pos[1])
    {
        game.use_failed = true;
        return true;
    }
    let mut frame = ItemFrame {
        id: game.next_entity + 1,
        pos,
        facing: facing_of(hit.face),
        glow: false,
        item: None,
        rotation: 0,
        drop_chance: 1.0,
        fixed: false,
        invisible: false,
        check_interval: 0,
        custom_name: stack.data().and_then(|d| d.custom_name.clone()),
        random: sunrise_client_render::random_source::entity(),
    };
    let own = sunrise_client_lib::entity_nbt::item_frame_to_nbt(&frame, [0; 4], None);
    let mut configured =
        sunrise_client_lib::block_entity_item::stack_configured_entity(stack, "minecraft:item_frame", &own);
    if let Tag::Compound(map) = &mut configured {
        map.insert("Variant".into(), Tag::Int(facts.variant));
    }
    // The woven loader applies the variant's setRotation to the configured
    // rotation (pluto-hanging-frame-from-nbt), as the original's load does.
    if let Some(mut loaded) = sunrise_client_lib::entity_nbt::item_frame_from_nbt(&configured, frame.id) {
        loaded.glow = false;
        frame = loaded;
    }
    if !frame_survives(game, &frame) {
        game.award_stat("used", &stack.item, 1);
        return true;
    }
    if game.hanging_place_cancelled(&facts.entity_type, pos) {
        return true;
    }
    game.next_entity += 1;
    game.audio.play(&sound(&frame, "place"), Some(center(&frame)), 1.0, 1.0);
    let ctx = game.player_context(None);
    game.post_game_event("minecraft:entity_place", center(&frame), ctx);
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
    let mut raw = configured;
    if let Tag::Compound(map) = &mut raw {
        map.remove(&"UUID".into());
    }
    game.persist.raw.insert(frame.id, raw);
    note_frame(frame.id, facts);
    game.item_frames.push(frame);
    if !game.creative {
        game.consume_held();
    }
    game.award_stat("used", &stack.item, 1);
    true
}

/// The frame an interact names (or the one under the crosshair), its
/// variant and the crosshair distance the host's own resolution found
/// (0 for a named frame), when it is a fancy frame at all.
pub(super) fn fancy_frame_target(
    game: &Game<'_>,
    target: &Option<InteractTarget>,
) -> Option<(usize, access::FrameFacts, f64)> {
    let (i, d) = match target {
        Some(t) => (Game::named_entity(&game.item_frames, t.id, |f| f.id)?, 0.0),
        None => game.frame_hit()?,
    };
    let facts = frame_facts(game, &game.item_frames[i])?;
    Some((i, facts, d))
}

/// The acting player is sneaking, for the clear frame's click-through: the
/// game's own player reads its keys; a LAN guest's sneak state is not
/// tracked server-side here, so a guest never clicks through (their click
/// falls back to the frame's own interaction).
fn acting_sneaking(game: &Game<'_>) -> bool {
    if game.lan_acting_uuid.is_some() {
        return true;
    }
    game.keys.held(Mapping::Sneak)
}

/// The clear frame's `interact`: use the block behind the frame through the
/// level's generic use branch (the special-cased blocks the host's own use
/// flow checks first — jukeboxes, composters, lecterns and the like — are
/// not reached this way and fall back to the frame's own interaction).
/// True when the block's use consumed the action.
fn click_through(game: &mut Game<'_>, i: usize) -> bool {
    let f = &game.item_frames[i];
    let step = f.step();
    let behind = [f.pos[0] - step[0], f.pos[1] - step[1], f.pos[2] - step[2]];
    let face = sunrise_core::geometry::Direction::ALL[(f.facing as usize).min(5)];
    let at = Vec3::new(
        f64::from(behind[0]) + 0.5 + f64::from(step[0]) * 0.5,
        f64::from(behind[1]) + 0.5 + f64::from(step[1]) * 0.5,
        f64::from(behind[2]) + 0.5 + f64::from(step[2]) * 0.5,
    );
    let player = game.use_context();
    let item = game.held().map(|s| s.item.to_string());
    let (result, _branch) = game.with_level(|b, level, ticks| {
        b.use_item_on_branch(
            level,
            ticks,
            BlockPos::new(behind[0], behind[1], behind[2]),
            face,
            at,
            player,
            item.as_deref(),
        )
    });
    let _ = game.process_level_events();
    if result.consumes_action() {
        if result.should_swing() {
            game.swing_arm();
        }
        return true;
    }
    false
}

/// `FancyItemFrameEntity.interact` over the host's `interact_frame` body:
/// the clear frame's click-through first, then put/rotate with the
/// variant's rotation rules (a rotating variant's `setItem` resets its
/// rotation; rotation goes through the variant's `setRotation`, timer
/// reset and diamond click included).
pub(super) fn interact_fancy_frame(
    game: &mut Game<'_>,
    named: bool,
    i: usize,
    facts: &access::FrameFacts,
    distance: f64,
) -> bool {
    if (!named && !game.before_block(distance)) || game.item_frames[i].fixed {
        return false;
    }
    if facts.click_through
        && game.item_frames[i].item.is_some()
        && !acting_sneaking(game)
        && click_through(game, i)
    {
        return true;
    }
    let at = center(&game.item_frames[i]);
    let entity_id = game.item_frames[i].id;
    let action;
    if game.item_frames[i].item.is_none() {
        let Some(held) = game.held().cloned() else {
            return false;
        };
        if let Some(id) = super::maps::map_id(&held)
            && game.map(id).is_some_and(|m| m.tracked_over_limit(256))
        {
            return true;
        }
        game.item_frames[i].item = Some(ItemStack { count: 1, ..held });
        reset_rotation(game, i, facts);
        action = "put";
        let name = sound(&game.item_frames[i], "add_item");
        game.audio.play(&name, Some(at), 1.0, 1.0);
    } else {
        let s = sound(&game.item_frames[i], "rotate_item");
        game.audio.play(&s, Some(at), 1.0, 1.0);
        let rotation = access::frame_rotation(facts, i32::from(game.item_frames[i].rotation) + 1);
        game.item_frames[i].rotation = rotation as i8;
        // setRotation resets the timer; diamond's also clicks.
        raw_set_int(game, entity_id, "RotationTimer", 0);
        if facts.winds_down {
            game.audio.play(CLICK_SOUND, Some(at), 1.0, 1.0);
        }
        action = "rotate";
    }
    if facts.hidden_when_filled {
        game.item_frames[i].invisible = game.item_frames[i].item.is_some();
    }
    let p = game.item_frames[i].pos;
    game.world.modified.insert((p[0] >> 4, p[2] >> 4));
    frame_output_changed(game, p);
    let ctx = game.who_context(Some(sunrise_client_lib::mobs::Who::Player), None);
    game.post_game_event("minecraft:block_change", at, ctx);
    if action == "put" && !game.creative {
        game.consume_held();
    }
    if let (Some(player), Some(name)) = (game.lan_acting_uuid, game.lan_acting_name.clone()) {
        let frame = game.item_frames.iter().find(|f| f.id == entity_id);
        let item = frame
            .and_then(|f| f.item.as_ref())
            .map_or_else(String::new, |s| s.item.to_string());
        let rotation = frame.map_or(0, |f| f.rotation);
        game.native_player_item_frame_change(
            player,
            &name,
            entity_id,
            action,
            &item,
            rotation.rem_euclid(8) as u8,
        );
    }
    game.swing_arm();
    true
}

/// At the head of `tick_item_frames`: the variant registry refresh, the
/// clear frames' visibility, and the rotation timers (server-side, in
/// entity ticking chunks, as the original's entity tick). The head is
/// the woven seam (a RETURN cannot be woven into the host body), so the
/// host's survival pass runs right after this: a frame it discards this
/// tick may have advanced its timer once, which its drops never read.
pub(super) fn tick_fancy_frames(game: &mut Game<'_>) {
    refresh_frame_facts(game);
    let ticking = game.entity_ticking_area();
    for i in 0..game.item_frames.len() {
        let Some(facts) = frame_facts(game, &game.item_frames[i]) else {
            continue;
        };
        if facts.hidden_when_filled {
            game.item_frames[i].invisible = game.item_frames[i].item.is_some();
        }
        if facts.rotation_step == 0 && !facts.winds_down {
            continue;
        }
        let at = center(&game.item_frames[i]);
        if !ticking(at[0].floor() as i32, at[2].floor() as i32) {
            continue;
        }
        let id = game.item_frames[i].id;
        let timer = raw_int(game, id, "RotationTimer").unwrap_or(0);
        let (timer, rotation, click) =
            access::frame_tick(&facts, timer, i32::from(game.item_frames[i].rotation));
        raw_set_int(game, id, "RotationTimer", timer);
        if let Some(rotation) = rotation {
            game.item_frames[i].rotation = rotation as i8;
            if click {
                game.audio.play(CLICK_SOUND, Some(at), 1.0, 1.0);
            }
            let pos = game.item_frames[i].pos;
            frame_output_changed(game, pos);
        }
    }
}

/// Before `explosion_breaks_frames`: fire-immune frames (netherite) step
/// out of the list for the blast (`ignoreExplosion`), and step back in
/// afterwards in their original order.
thread_local! {
    static BLAST_STASH: RefCell<Vec<(usize, ItemFrame)>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn stash_immune_frames(game: &mut Game<'_>) {
    let mut i = game.item_frames.len();
    while i > 0 {
        i -= 1;
        let immune = frame_facts(game, &game.item_frames[i]).is_some_and(|facts| facts.fire_immune);
        if immune {
            let f = game.item_frames.remove(i);
            BLAST_STASH.with(|stash| stash.borrow_mut().push((i, f)));
        }
    }
}

pub(super) fn restore_immune_frames(game: &mut Game<'_>) {
    BLAST_STASH.with(|stash| {
        let mut stash = stash.borrow_mut();
        // The stash was filled from the back, so it pops in ascending order.
        while let Some((index, f)) = stash.pop() {
            let at = index.min(game.item_frames.len());
            game.item_frames.insert(at, f);
        }
    });
}

/// `ItemFrame.dropItem` for a fancy frame: the host's `drop_frame_items`
/// body with the variant's item (through `getFrameItemStackWithData`'s
/// shape: the frame item with the entity's data) in place of the vanilla
/// one. True when the frame was fancy and the drops are done.
pub(super) fn fancy_frame_drops(
    game: &mut Game<'_>,
    f: &mut ItemFrame,
    with_frame: bool,
    by_player: bool,
    creative: bool,
) -> bool {
    let Some(facts) = frame_facts(game, f) else {
        return false;
    };
    if game.ui.rules.bool("entity_drops") || !by_player {
        game.remove_framed_map(f);
    }
    if !game.ui.rules.bool("entity_drops") || (by_player && creative) {
        return true;
    }
    let s = f.step().map(f64::from);
    let c = center(f);
    let at = [c[0] + s[0] * 0.15, c[1], c[2] + s[2] * 0.15];
    let mut drops = Vec::new();
    if with_frame {
        let mut stack = f.item_stack_with_data();
        stack.item = facts.item.clone().into();
        drops.push(stack);
    }
    if let Some(item) = f.item.clone()
        && f.random.next_float() < f.drop_chance
    {
        drops.push(item);
    }
    if by_player && !drops.is_empty() {
        let tool = game.held().cloned();
        game.native_use_drops("", c, None, Some((f.id, f.kind())), tool.as_ref(), &mut drops);
    }
    for stack in drops {
        game.spawn_item(stack, at, [0.0; 3], 10);
    }
    true
}

/// After `hit_frame`: a filled fancy frame whose item was taken out went
/// through the original's `setItem(EMPTY)`, which resets a rotating
/// variant's rotation; a frame that broke is gone and needs nothing.
pub(super) fn fancy_frame_hit(game: &mut Game<'_>, i: usize) {
    let Some(f) = game.item_frames.get(i) else {
        return;
    };
    if f.item.is_some() {
        return;
    }
    let Some(facts) = frame_facts(game, f) else {
        return;
    };
    reset_rotation(game, i, &facts);
    if facts.hidden_when_filled {
        game.item_frames[i].invisible = false;
    }
}

/// Before `set_frame_item` (the command path's `setItem(stack, true)`): a
/// rotating variant's rotation resets, as on every `setItem`.
pub(super) fn fancy_frame_set_item(game: &mut Game<'_>, i: usize) {
    let Some(f) = game.item_frames.get(i) else {
        return;
    };
    let Some(facts) = frame_facts(game, f) else {
        return;
    };
    reset_rotation(game, i, &facts);
}

/// `StandType.onPlace` plus the saved variant: the properties go on the
/// stand itself (the host's save writes them from there) and the variant
/// and left-handed flag into the kept raw NBT, the two fields the host
/// does not model.
fn apply_on_place(game: &mut Game<'_>, id: u32, facts: &access::StandFacts, left_handed: Option<bool>) {
    if let Some(i) = game.armor_stands.iter().position(|a| a.id == id) {
        game.armor_stands[i].small = facts.small;
        if facts.arms {
            game.armor_stands[i].show_arms = true;
        }
    }
    // A dispensed stand whose stack carried no config has no raw entry
    // yet; start one from the stand itself so the variant has somewhere
    // to live (the save merges the kept raw under the modelled fields).
    if !game.persist.raw.contains_key(&id) {
        if let Some(a) = game.armor_stands.iter().find(|a| a.id == id) {
            let own = sunrise_client_lib::entity_nbt::armor_stand_to_nbt(a, [0; 4], None);
            game.persist.raw.insert(id, own);
        }
    }
    if let Some(Tag::Compound(map)) = game.persist.raw.get_mut(&id) {
        map.insert("Variant".into(), Tag::Int(facts.variant));
        if let Some(left) = left_handed {
            map.insert("LeftHanded".into(), Tag::Byte(i8::from(left)));
        }
    }
}

/// `FancyArmorStandItem.useOn`: the host's `place_armor_stand` body with
/// the variant's `onPlace` applied after the stack config (the original's
/// order: create with the config, then `type.onPlace`), and the
/// left-handed flag from the placing hand and the player's main arm
/// (`(hand == MAIN_HAND) == (mainArm == LEFT)`; for a LAN guest the host
/// player's main-arm setting stands in, as the host tracks no other).
pub(super) fn place_fancy_stand(
    game: &mut Game<'_>,
    hit: &Hit,
    stack: &ItemStack,
    facts: &access::StandFacts,
) -> bool {
    let Some((pos, mut stand)) = game.armor_stand_spawn(hit, stack) else {
        return false;
    };
    let drop = game.drop_offset_from(0.5, 1.975, (pos[0], pos[1], pos[2]), true);
    stand.position.y = f64::from(pos[1]) + drop;
    stand.previous = stand.position;
    let own = sunrise_client_lib::entity_nbt::armor_stand_to_nbt(&stand, [0; 4], None);
    let mut raw = sunrise_client_lib::block_entity_item::stack_configured_entity(stack, &facts.entity_type, &own);
    if let Some(mut loaded) = sunrise_client_lib::entity_nbt::armor_stand_from_nbt(&raw, stand.id) {
        loaded.first_tick = stand.first_tick;
        loaded.yaw = stand.yaw;
        loaded.previous_yaw = stand.previous_yaw;
        loaded.pitch = 0.0;
        loaded.previous = loaded.position;
        stand = loaded;
    }
    stand.small = facts.small;
    if facts.arms {
        stand.show_arms = true;
    }
    if let Tag::Compound(map) = &mut raw {
        map.remove(&"UUID".into());
        map.insert("Variant".into(), Tag::Int(facts.variant));
        let left = (game.hand == sunrise_client_lib::first_person::Hand::Main) == game.settings.main_hand_left;
        map.insert("LeftHanded".into(), Tag::Byte(i8::from(left)));
    }
    game.persist.raw.insert(stand.id, raw);
    let foot = stand.position;
    game.world.random.next_float();
    game.next_entity += 1;
    let (_, uuid) = sunrise_client_lib::mobs::living_random();
    game.persist.uuids.insert(stand.id, uuid);
    game.audio
        .play("minecraft:entity.armor_stand.place", Some([foot.x, foot.y, foot.z]), 0.75, 0.8);
    let ctx = game.player_context(None);
    game.post_game_event("minecraft:entity_place", [foot.x, foot.y, foot.z], ctx);
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
    game.armor_stands.push(stand);
    if !game.creative {
        game.consume_held();
    }
    game.award_stat("used", &stack.item, 1);
    game.swing_arm();
    true
}

fn stand_box(a: &ArmorStand) -> (f32, f32) {
    match (a.marker, a.small) {
        (true, _) => (0.0, 0.0),
        (false, true) => (0.25, 0.9875),
        (false, false) => (0.5, 1.975),
    }
}

/// `ArmorStand.showBreakingParticles`, adapted from the pinned host.
fn show_breaking_particles(game: &mut Game<'_>, at: [f64; 3], width: f64, height: f64) {
    let Some(state) = game.pack.assets.registry.default_state("minecraft:oak_planks") else {
        return;
    };
    let block = [at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32];
    let Some((sprite, tint)) = game.particle_sprite(state, block) else {
        return;
    };
    let options = sunrise_client_render::particles::ParticleOptions::block(
        sunrise_client_render::particles::ParticleType::Block,
        sprite,
        tint,
    );
    let centre = [at[0], at[1] + height * (2.0 / 3.0), at[2]];
    let spread = [width / 4.0, height / 4.0, width / 4.0];
    game.particles.send_particles(&options, centre, spread, 0.05, 10);
    game.lan_level_particles(None, &options, centre, spread, 0.05, 10, Some(state));
}

/// The host's `break_armor_stand_by_hit` body with one change:
/// `brokenByPlayer` pops the variant's item (keeping the custom name the
/// host already copies), not the vanilla stand item.
pub(super) fn break_fancy_stand(
    game: &mut Game<'_>,
    i: usize,
    creative: bool,
    always_kills: bool,
    by: Option<super::commands::Target>,
    facts: &access::StandFacts,
) -> bool {
    let time = game.world.game_time;
    let at = game.armor_stands[i].position;
    let at = [at.x, at.y, at.z];
    if creative {
        let (width, height) = stand_box(&game.armor_stands[i]);
        game.armor_stands.remove(i);
        game.audio.play("minecraft:entity.armor_stand.break", Some(at), 1.0, 1.0);
        show_breaking_particles(game, at, f64::from(width), f64::from(height));
        game.entity_event_by("minecraft:entity_die", at, by);
        return true;
    }
    if time - game.armor_stands[i].last_hit > 5 && !always_kills {
        game.audio
            .play("minecraft:entity.armor_stand.hit", Some(at), 0.3, 1.0);
        game.entity_event_by("minecraft:entity_damage", at, by);
        game.armor_stands[i].last_hit = time;
        return true;
    }
    let (width, height) = stand_box(&game.armor_stands[i]);
    let p = game.armor_stands[i].position;
    let pos = [p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32];
    let mut stand = ItemStack::new(facts.item.as_str(), 1);
    if let Some(name) = &game.armor_stands[i].custom_name {
        stand.data_mut().custom_name =
            Some(sunrise_client_core::scoreboard::component_tag(&name.component()));
    }
    game.pop_resource(pos, stand);
    game.break_armor_stand_with_source(i, by);
    show_breaking_particles(game, at, f64::from(width), f64::from(height));
    true
}

/// Around `tick_armor_stands`: every fancy stand is fire immune at the
/// class level, so its fire ticks never stand (cleared before the tick,
/// whose environment step would light it, and again in the after-pass —
/// which rides the head of `tick_cushions`, the call the pinned tick
/// sequence makes immediately after the stand tick, a RETURN on the
/// stand tick being unweavable — so it never renders as burning either);
/// the clear stand's invisibility follows its equipment
/// (`isClearHidden` — the host has the one flag, so an NBT-invisible
/// clear stand with empty slots reads visible again).
pub(super) fn tick_fancy_stands(game: &mut Game<'_>, after: bool) {
    for i in 0..game.armor_stands.len() {
        let Some(facts) = stand_facts(game, &game.armor_stands[i]) else {
            continue;
        };
        game.armor_stands[i].fire_ticks = 0;
        if after {
            let equipped = game.armor_stands[i].equipment.iter().any(|slot| slot.is_some());
            let hidden = access::stand_hidden(&facts, equipped);
            game.armor_stands[i].invisible = hidden;
        }
    }
}

/// `DispenseFancyArmorStand`: the host's `dispense_armor_stand` body with
/// the variant's `onPlace` applied after the stack config and no
/// left-handed flag (the item's placement sets it, a dispenser does not).
/// The answer is the host's CLICK event (1000).
pub(super) fn dispense_fancy_stand(
    game: &mut Game<'_>,
    pos: (i32, i32, i32),
    facing: sunrise_core::geometry::Direction,
    stack: &ItemStack,
    facts: &access::StandFacts,
) -> sunrise_client::Result<(Option<ItemStack>, i32)> {
    use sunrise_core::geometry::Direction;
    let step = match facing {
        Direction::Down => [0, -1, 0],
        Direction::Up => [0, 1, 0],
        Direction::North => [0, 0, -1],
        Direction::South => [0, 0, 1],
        Direction::West => [-1, 0, 0],
        Direction::East => [1, 0, 0],
    };
    let front = (pos.0 + step[0], pos.1 + step[1], pos.2 + step[2]);
    let foot = Vec3::new(f64::from(front.0) + 0.5, f64::from(front.1), f64::from(front.2) + 0.5);
    let _ = game.world.random.next_float();
    game.next_entity += 1;
    let (_, uuid) = sunrise_client_lib::mobs::living_random();
    game.persist.uuids.insert(game.next_entity, uuid);
    let yaw = match facing {
        Direction::South => 0.0,
        Direction::West => 90.0,
        Direction::North => 180.0,
        Direction::East => 270.0,
        _ => 270.0,
    };
    let stand = sunrise_client::entities::ArmorStand::new(game.next_entity, foot, yaw);
    let id = stand.id;
    game.world.modified.insert((front.0 >> 4, front.2 >> 4));
    game.armor_stands.push(stand);
    game.apply_item_stack_config(super::commands::Target::Stand(id), "minecraft:armor_stand", stack, false);
    apply_on_place(game, id, facts, None);
    let mut left = stack.clone();
    left.count -= 1;
    Ok(((left.count > 0).then_some(left), 1000))
}

/// A `/damage` refusal for a fancy stand: the class is fire immune, so a
/// fire damage type never lights or burns it (the host's command path
/// returns false for the fire sources but applies those effects first).
pub(super) fn fancy_stand_refuses(game: &Game<'_>, id: u32, damage_type: &str) -> bool {
    let Some(a) = game.armor_stands.iter().find(|a| a.id == id) else {
        return false;
    };
    if stand_facts(game, a).is_none() {
        return false;
    }
    let registry = &game.motion.health.damage;
    let Some(kind) = registry.id(damage_type) else {
        return false;
    };
    registry.in_tag(kind, "minecraft:is_fire")
}
