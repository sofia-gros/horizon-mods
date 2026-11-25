//! The slime bounce handler and the drill spin attack's landing, from the original's
//! `SlimeBounceHandler` and `ModifierEvents.bounceOnFall` (SlimeKnights, MIT; see LICENSE).
//!
//! A fall the bouncy attribute bends is decided here, on PlutoLib's living-fall event: a
//! crouching landing halves the damage, any other landing with bouncy at 1 or more is
//! cancelled and the entity bounces with the original's velocity, and the bounce handler
//! then runs on the player's ticks, preserving the horizontal momentum through the air
//! (the 0.975 boost, the angle-change sound, the stall timer) until the entity lands and
//! five ticks pass. The handler is the original's per-entity tick handler, kept by entity
//! id; the sling leap adds the momentum-only handler after its launch.
//!
//! Two shapes of the original are deferred by the host's fall event, which carries no
//! velocity and no host handle: the bounce velocity is worked out from the motion the
//! entity's last tick kept, and it is applied on the next tick (for a player, as a push of
//! the difference between the set velocity and that kept motion: the host's
//! `entity_set_velocity` refuses players, and the player push is the queued reach; for a
//! mob, as the set itself). The bounce therefore lands one tick late by construction,
//! exactly as the original's own non-player branch already accepts.
//!
//! Mobs are covered by PlutoLib's mob tick (`ServerLivingEntityEvents::MOB_TICK`): it feeds
//! the kept facts with the mob's own reported velocity (a mob never sneaks, so it never
//! suppresses its bounce), feeds the mob's bouncy value through the custom attributes, and
//! runs this same tick machine; the climbable fact comes from the transport's read of the
//! game's own check. A mob's kept facts and handler are forgotten when it unloads (its
//! entity id is not reused by the same mob); a player's live for the session.

use crate::custom_attributes::{self, BOUNCY};
use pluto_lib::{EntityInfo, EntityReach, LivingFall, LivingFallEvents, MobTick, PlayerTick, ServerEntityEvents, ServerLivingEntityEvents, World};
use std::collections::BTreeMap;
use std::sync::Mutex;

/// The step height the original reads (`ForgeMod.STEP_HEIGHT_ADDITION`, the vanilla default 0.6).
const STEP_HEIGHT: f64 = 0.6;
/// The entity gravity the original reads (`ENTITY_GRAVITY`, the vanilla default 0.08).
const ENTITY_GRAVITY: f64 = 0.08;
/// The bounce sound, played on the bounce and when the heading turns sharply.
const BOUNCE_SOUND: &str = "tconstruct:slimy_bounce";

/// What the fall handler decides for one landing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FallDecision {
    /// Crouching: the damage is halved, and nothing else happens.
    Suppress {
        /// The damage multiplier to write back (0.5).
        multiplier: f32,
    },
    /// The fall is cancelled and the entity bounces with this velocity; `sound` plays the bounce sound with it.
    Bounce {
        /// The velocity to set on the bounce tick.
        velocity: [f64; 3],
        /// Whether the bounce sound plays.
        sound: bool,
    },
}

/// The original's `bounceOnFall` (`ModifierEvents.java`): the gate, the crouch suppression,
/// and the bounce. `velocity` is the entity's motion at the fall; `step_height` is the step
/// height and `gravity` the entity gravity. `None` when the fall stands as-is. The player
/// branch rises with the velocity a fall of this distance earns under gravity; the
/// non-player branch flips the downward motion instead (the factor is -0.7 under 2 blocks,
/// -0.9 at or over, reachable through a fast descent on a short fall). A mob's bouncy value
/// is fed by its tick report through the custom attributes, so a bouncing mob reaches here
/// and takes the non-player formula.
pub fn decide_fall(fall_distance: f64, velocity: [f64; 3], bouncy: f64, suppressing: bool, player: bool, step_height: f64, gravity: f64) -> Option<FallDecision> {
    if fall_distance < 3.0 && velocity[1] > -0.3 {
        return None;
    }
    if fall_distance <= 0.5 + step_height {
        return None;
    }
    if bouncy < 1.0 {
        return None;
    }
    if suppressing {
        return Some(FallDecision::Suppress { multiplier: 0.5 });
    }
    if player {
        let rise = (fall_distance * gravity).sqrt();
        Some(FallDecision::Bounce { velocity: [velocity[0] / 0.975, rise, velocity[2] / 0.975], sound: true })
    } else {
        let factor = if fall_distance < 2.0 { -0.7 } else { -0.9 };
        Some(FallDecision::Bounce { velocity: [velocity[0] / 0.975, velocity[1] * factor, velocity[2] / 0.975], sound: true })
    }
}

/// What one tick of the bounce machine asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
enum TickStep {
    /// Set the entity's velocity to this; `sound` plays the bounce sound with it (the angle-change sound).
    Set {
        /// The velocity to set.
        velocity: [f64; 3],
        /// Whether the bounce sound plays with the set.
        sound: bool,
    },
    /// Nothing to do this tick; the handler stays.
    Keep,
    /// The handler ends; forget the entity.
    End,
}

/// The per-entity state of the original's `SlimeBounceHandler.BounceInfo`, kept while a bounce is live.
#[derive(Clone, Copy, Debug)]
pub struct BounceInfo {
    /// The velocity to set on the bounce tick; `None` once it has been set (the momentum-only handler).
    bounce: Option<[f64; 3]>,
    /// The tick the bounce lands on; 0 for the momentum-only handler.
    bounce_tick: i64,
    /// The tick the handler stops preserving the momentum, once the motion has stalled; 0 while it moves.
    stop_mag_tick: i64,
    /// The squared horizontal speed the handler preserves.
    last_mag_sq: f64,
    /// Whether the entity was airborne on its last tick.
    was_in_air: bool,
    /// The tick the handler ends, once the entity has landed; 0 while airborne.
    end_handler: i64,
    /// The last horizontal heading, in radians, for the angle-change sound.
    last_angle: f64,
}

impl BounceInfo {
    /// A handler for a bounce: `bounce` is the velocity to set on the next tick, or `None`
    /// for the momentum-only handler the sling leap adds. `now` is the entity's local tick
    /// count and `motion` its motion now, from which the preserved speed is taken. The
    /// heading starts at 0, as the original's constructor leaves it.
    pub fn new(now: i64, bounce: Option<[f64; 3]>, motion: [f64; 3]) -> Self {
        Self { bounce, bounce_tick: if bounce.is_some() { now + 1 } else { 0 }, stop_mag_tick: 0, last_mag_sq: motion[0] * motion[0] + motion[2] * motion[2], was_in_air: false, end_handler: 0, last_angle: 0.0 }
    }

    /// The original's update path: an existing handler takes a new bounce, to land on the
    /// next tick, and its preserved speed and heading are refreshed from the motion now.
    fn retarget(&mut self, now: i64, bounce: [f64; 3], motion: [f64; 3]) {
        self.bounce = Some(bounce);
        self.bounce_tick = now + 1;
        self.last_mag_sq = motion[0] * motion[0] + motion[2] * motion[2];
        self.last_angle = motion[0].atan2(motion[2]);
    }

    /// One tick of the original's `onLivingTick` for this handler. `velocity` is the
    /// entity's motion this tick; `on_ground`, `in_water`, `on_climbable`, `gliding`
    /// (`isFallFlying`) and `spectator` are the tick's facts.
    fn tick(&mut self, now: i64, velocity: [f64; 3], on_ground: bool, in_water: bool, on_climbable: bool, gliding: bool, spectator: bool) -> TickStep {
        if gliding || spectator {
            return TickStep::End;
        }
        let mut motion = velocity;
        let mut set: Option<[f64; 3]> = None;
        let mut sound = false;
        if let Some(bounce) = self.bounce {
            if now == self.bounce_tick {
                self.bounce = None;
                self.bounce_tick = 0;
                motion = bounce;
                set = Some(bounce);
            }
        }
        let in_air = !on_ground && !in_water && !on_climbable;
        if in_air && self.last_mag_sq > 0.0 {
            let motion_sq = motion[0] * motion[0] + motion[2] * motion[2];
            if motion_sq == 0.0 {
                if self.stop_mag_tick == 0 {
                    self.stop_mag_tick = now + 5;
                } else if now >= self.stop_mag_tick {
                    self.last_mag_sq = 0.0;
                }
            } else {
                self.stop_mag_tick = 0;
                let angle = motion[0].atan2(motion[2]);
                let mut boosted = false;
                if motion_sq < self.last_mag_sq {
                    let boost = (self.last_mag_sq / motion_sq).sqrt() * 0.975;
                    if boost > 1.0 {
                        sound = (angle - self.last_angle).abs() > 1.0;
                        self.last_angle = angle;
                        self.last_mag_sq *= 0.975 * 0.975;
                        let boosted_velocity = [motion[0] * boost, motion[1], motion[2] * boost];
                        set = Some(boosted_velocity);
                        boosted = true;
                    }
                }
                if !boosted {
                    self.last_mag_sq = motion_sq;
                    self.last_angle = angle;
                }
            }
        }
        if self.was_in_air && !in_air {
            if self.end_handler == 0 {
                self.end_handler = now + 5;
            } else if now >= self.end_handler {
                return TickStep::End;
            }
        } else {
            self.end_handler = 0;
            self.was_in_air = in_air;
        }
        match set {
            Some(velocity) => TickStep::Set { velocity, sound },
            None => TickStep::Keep,
        }
    }
}

/// What the bounce handler keeps per entity, between the entity's ticks.
#[derive(Clone, Debug)]
struct Kept {
    /// The entity's last tick position, so a player's motion is the change since then.
    position: [f64; 3],
    /// The entity's motion on its last tick: from successive positions for a player, the
    /// velocity its tick report carried for a mob.
    velocity: [f64; 3],
    /// The entity's local tick count: one per heard tick, so a bounce set on one tick lands on the next.
    tick: i64,
    /// Whether the entity sneaked on its last tick.
    sneaking: bool,
    /// The dimension the entity last ticked in, for the bounce sound (empty before the first tick).
    dimension: String,
}

impl Default for Kept {
    fn default() -> Self {
        Self { position: [0.0; 3], velocity: [0.0; 3], tick: 0, sneaking: false, dimension: String::new() }
    }
}

static KEPT: Mutex<BTreeMap<i32, Kept>> = Mutex::new(BTreeMap::new());
static BOUNCES: Mutex<BTreeMap<i32, BounceInfo>> = Mutex::new(BTreeMap::new());

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Listens to the living fall and to the entity ticks: the fall handler bounces (or halves)
/// a landing, and the tick handlers run the bounce machine for the entities that carry one.
pub(crate) fn register() {
    LivingFallEvents::FALL.register(|fall| on_fall(fall));
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| on_player_tick(tick));
    ServerLivingEntityEvents::MOB_TICK.register(|tick| on_mob_tick(tick));
    ServerEntityEvents::ENTITY_UNLOAD.register(|info| on_unload(info));
}

/// The original's `SlimeBounceHandler.addBounceHandler(entity)` (the null-bounce call): the
/// momentum-only handler the sling leap adds, so the holder keeps their speed through the air
/// after the launch. The local tick count and the motion come from the facts the player's last
/// tick kept; a player who has never ticked starts at tick 0 with no motion. An existing
/// handler is left alone: it already preserves the momentum, its pending bounce (if any)
/// stands, and the tick machine refreshes the preserved speed from the motion next tick.
/// Fake players are excluded in the original; the port has no fake players (every tick
/// carries a real player).
pub(crate) fn add_bounce_handler(entity: i32) {
    let (now, motion) = {
        let kept = KEPT.lock().expect("the bounce facts are never poisoned");
        match kept.get(&entity) {
            Some(kept) => (kept.tick, kept.velocity),
            None => (0, [0.0; 3]),
        }
    };
    let mut bounces = BOUNCES.lock().expect("the bounce state is never poisoned");
    if !bounces.contains_key(&entity) {
        bounces.insert(entity, BounceInfo::new(now, None, motion));
    }
}

fn on_fall(fall: &LivingFall) -> bool {
    let entity = fall.entity as i32;
    let bouncy = custom_attributes::value(entity, BOUNCY);
    if bouncy < 1.0 {
        return true;
    }
    let kept = {
        let kept = KEPT.lock().expect("the bounce facts are never poisoned");
        kept.get(&entity).cloned().unwrap_or_default()
    };
    let Some(decision) = decide_fall(fall.distance.get(), kept.velocity, bouncy, kept.sneaking, fall.kind == "minecraft:player", STEP_HEIGHT, ENTITY_GRAVITY) else {
        return true;
    };
    match decision {
        FallDecision::Suppress { multiplier } => {
            fall.multiplier.set(multiplier);
            true
        }
        FallDecision::Bounce { velocity, sound } => {
            fall.distance.set(0.0);
            fall.multiplier.set(0.0);
            if sound && !kept.dimension.is_empty() {
                World::new(&kept.dimension).play_sound(BOUNCE_SOUND, kept.position, 1.0, 1.0);
            }
            let mut bounces = BOUNCES.lock().expect("the bounce state is never poisoned");
            match bounces.get_mut(&entity) {
                Some(info) => info.retarget(kept.tick, velocity, kept.velocity),
                None => {
                    bounces.insert(entity, BounceInfo::new(kept.tick, Some(velocity), kept.velocity));
                }
            }
            false
        }
    }
}

fn on_player_tick(tick: &PlayerTick) {
    let (velocity, now) = {
        let mut kept = KEPT.lock().expect("the bounce facts are never poisoned");
        let entry = kept.entry(tick.entity).or_insert_with(Kept::default);
        let velocity = sub(tick.position, entry.position);
        entry.velocity = velocity;
        entry.position = tick.position;
        entry.sneaking = tick.sneaking;
        entry.tick += 1;
        entry.dimension = tick.dimension.clone();
        (velocity, entry.tick)
    };
    let step = {
        let mut bounces = BOUNCES.lock().expect("the bounce state is never poisoned");
        let Some(info) = bounces.get_mut(&tick.entity) else { return };
        info.tick(now, velocity, tick.on_ground, tick.in_water, tick.on_climbable, tick.gliding, tick.spectator)
    };
    match step {
        TickStep::Set { velocity: set, sound } => {
            // The host refuses a player's set-velocity, so the set lands as a push of the
            // difference between the set velocity and the motion the tick measured.
            EntityReach::push(tick.entity as u64, sub(set, velocity));
            if sound {
                World::new(&tick.dimension).play_sound(BOUNCE_SOUND, tick.position, 1.0, 1.0);
            }
        }
        TickStep::Keep => {}
        TickStep::End => {
            BOUNCES.lock().expect("the bounce state is never poisoned").remove(&tick.entity);
        }
    }
}

/// A mob's tick: the kept facts take the motion the report itself carries (a mob's velocity
/// is read from the host, not from successive positions), and the bounce machine runs with
/// the report's facts. A mob never sneaks, so it never suppresses its bounce, and it never
/// spectates for the handler's end conditions; a mob that glides ends its bounce as a
/// gliding player does.
fn on_mob_tick(tick: &MobTick) {
    let (velocity, now) = {
        let mut kept = KEPT.lock().expect("the bounce facts are never poisoned");
        let entry = kept.entry(tick.entity).or_insert_with(Kept::default);
        entry.velocity = tick.velocity;
        entry.position = tick.position;
        entry.sneaking = false;
        entry.tick += 1;
        entry.dimension = tick.dimension.clone();
        (tick.velocity, entry.tick)
    };
    let step = {
        let mut bounces = BOUNCES.lock().expect("the bounce state is never poisoned");
        let Some(info) = bounces.get_mut(&tick.entity) else { return };
        info.tick(now, velocity, tick.on_ground, tick.in_water, tick.on_climbable, tick.gliding, false)
    };
    match step {
        TickStep::Set { velocity: set, sound } => {
            // A mob's motion is set outright; when the host cannot set it, the set lands as a
            // push of the difference between the set velocity and the reported motion, as a
            // player's does.
            if !EntityReach::set_velocity(tick.entity as u64, set) {
                EntityReach::push(tick.entity as u64, sub(set, velocity));
            }
            if sound {
                World::new(&tick.dimension).play_sound(BOUNCE_SOUND, tick.position, 1.0, 1.0);
            }
        }
        TickStep::Keep => {}
        TickStep::End => {
            BOUNCES.lock().expect("the bounce state is never poisoned").remove(&tick.entity);
        }
    }
}

/// An entity leaving the world: a mob's kept facts and bounce handler go with it (its entity
/// id is not reused by the same mob). A player's are kept: their id is stable for the session
/// and the fall reads them between ticks.
fn on_unload(info: &EntityInfo) {
    if info.kind == "minecraft:player" {
        return;
    }
    KEPT.lock().expect("the bounce facts are never poisoned").remove(&(info.id as i32));
    BOUNCES.lock().expect("the bounce state is never poisoned").remove(&(info.id as i32));
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{Identifier, LivingFacts, MemoryEntityOps, MemoryWorldOps, WorldEntity, with_entities, with_world};
    use std::cell::{Cell, RefCell};

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 1e-9)
    }

    #[test]
    fn a_short_or_slow_fall_stands_and_a_bouncy_one_bounces() {
        // Under 3 blocks and slow: the gate stands the fall.
        assert_eq!(decide_fall(2.0, [0.0, -0.2, 0.0], 1.0, false, true, STEP_HEIGHT, ENTITY_GRAVITY), None);
        // Under the step height, however fast: the gate stands the fall.
        assert_eq!(decide_fall(1.0, [0.0, -1.0, 0.0], 1.0, false, true, STEP_HEIGHT, ENTITY_GRAVITY), None);
        // No bouncy: the fall stands.
        assert_eq!(decide_fall(9.0, [0.0, -3.0, 0.0], 0.0, false, true, STEP_HEIGHT, ENTITY_GRAVITY), None);
        assert_eq!(decide_fall(9.0, [0.0, -3.0, 0.0], 0.5, false, true, STEP_HEIGHT, ENTITY_GRAVITY), None);
        // A 9-block fall, bouncy: the player rises with the velocity the fall earns under gravity.
        let rise = (9.0 * ENTITY_GRAVITY).sqrt();
        let decision = decide_fall(9.0, [2.0, -3.0, 1.0], 1.0, false, true, STEP_HEIGHT, ENTITY_GRAVITY);
        assert!(matches!(decision, Some(FallDecision::Bounce { sound: true, .. })));
        if let Some(FallDecision::Bounce { velocity, .. }) = decision {
            assert!(close(velocity, [2.0 / 0.975, rise, 1.0 / 0.975]), "the bounce velocity: {velocity:?}");
        }
    }

    #[test]
    fn a_fast_descent_bounces_on_a_short_fall_with_the_non_player_factor() {
        // A short fall at speed: the first gate lets it through, and the non-player branch flips the motion.
        let short = decide_fall(1.5, [0.0, -1.0, 0.0], 1.0, false, false, STEP_HEIGHT, ENTITY_GRAVITY);
        assert!(matches!(short, Some(FallDecision::Bounce { sound: true, .. })));
        if let Some(FallDecision::Bounce { velocity, .. }) = short {
            assert!(close(velocity, [0.0, 0.7, 0.0]), "the short fall flips with -0.7: {velocity:?}");
        }
        let long = decide_fall(9.0, [2.0, -3.0, 1.0], 1.0, false, false, STEP_HEIGHT, ENTITY_GRAVITY);
        assert!(matches!(long, Some(FallDecision::Bounce { sound: true, .. })));
        if let Some(FallDecision::Bounce { velocity, .. }) = long {
            assert!(close(velocity, [2.0 / 0.975, 2.7, 1.0 / 0.975]), "the long fall flips with -0.9: {velocity:?}");
        }
    }

    #[test]
    fn crouching_halves_the_damage_and_bounces_nothing() {
        assert_eq!(
            decide_fall(9.0, [0.0, -3.0, 0.0], 1.0, true, true, STEP_HEIGHT, ENTITY_GRAVITY),
            Some(FallDecision::Suppress { multiplier: 0.5 })
        );
    }

    #[test]
    fn a_bounce_lands_on_the_next_tick_and_not_before() {
        let mut info = BounceInfo::new(10, Some([1.0, 2.0, 0.0]), [1.0, 0.0, 0.0]);
        assert_eq!(info.tick(10, [1.0, -1.0, 0.0], false, false, false, false, false), TickStep::Keep);
        assert_eq!(
            info.tick(11, [1.0, -1.0, 0.0], false, false, false, false, false),
            TickStep::Set { velocity: [1.0, 2.0, 0.0], sound: false }
        );
        assert_eq!(info.tick(12, [1.0, 2.0, 0.0], false, false, false, false, false), TickStep::Keep, "the bounce is spent");
    }

    #[test]
    fn momentum_is_boosted_back_and_a_sharp_turn_sounds() {
        let mut info = BounceInfo::new(10, None, [2.0, 0.0, 0.0]);
        // The motion fell to half the preserved speed: boost sqrt(4/1) * 0.975 = 1.95.
        let step = info.tick(11, [1.0, -1.0, 0.0], false, false, false, false, false);
        assert_eq!(step, TickStep::Set { velocity: [1.95, -1.0, 0.0], sound: true }, "the heading turned from 0 to pi/2");
        // The motion holds at the preserved speed: nothing to do.
        assert_eq!(info.tick(12, [1.95, -1.0, 0.0], false, false, false, false, false), TickStep::Keep);
    }

    #[test]
    fn a_stalled_handler_keeps_its_speed_for_five_ticks_then_lets_go() {
        let mut info = BounceInfo::new(0, None, [2.0, 0.0, 0.0]);
        for now in 1..=5 {
            assert_eq!(info.tick(now, [0.0, 0.0, 0.0], false, false, false, false, false), TickStep::Keep);
        }
        assert_eq!(info.tick(6, [0.0, 0.0, 0.0], false, false, false, false, false), TickStep::Keep, "the stall timer runs out on the sixth tick");
        assert_eq!(info.tick(7, [1.0, 0.0, 0.0], false, false, false, false, false), TickStep::Keep, "with nothing preserved, no boost follows");
    }

    #[test]
    fn landing_ends_the_handler_five_ticks_later_and_flying_ends_it_at_once() {
        let mut info = BounceInfo::new(0, None, [2.0, 0.0, 0.0]);
        assert_eq!(info.tick(1, [2.0, 0.0, 0.0], false, false, false, false, false), TickStep::Keep);
        for now in 2..=6 {
            assert_eq!(info.tick(now, [2.0, 0.0, 0.0], true, false, false, false, false), TickStep::Keep);
        }
        assert_eq!(info.tick(7, [2.0, 0.0, 0.0], true, false, false, false, false), TickStep::End);
        let mut flying = BounceInfo::new(0, None, [2.0, 0.0, 0.0]);
        assert_eq!(flying.tick(1, [2.0, 0.0, 0.0], false, false, false, true, false), TickStep::End, "gliding ends it");
        assert_eq!(flying.tick(1, [2.0, 0.0, 0.0], false, false, false, false, true), TickStep::End, "spectating ends it");
    }

    #[test]
    fn on_a_climbable_block_the_handler_counts_as_landed() {
        let mut info = BounceInfo::new(0, None, [2.0, 0.0, 0.0]);
        assert_eq!(info.tick(1, [2.0, 0.0, 0.0], false, false, false, false, false), TickStep::Keep, "airborne first");
        for now in 2..=6 {
            assert_eq!(info.tick(now, [2.0, 0.0, 0.0], false, false, true, false, false), TickStep::Keep, "on the climbable block the landing timer runs, and no boost is worked out");
        }
        assert_eq!(info.tick(7, [2.0, 0.0, 0.0], false, false, true, false, false), TickStep::End, "the handler ends five ticks after it landed");
    }

    fn tick_for(entity: i32, position: [f64; 3], on_ground: bool, sneaking: bool, bouncy: bool) -> PlayerTick {
        PlayerTick {
            player: entity as u128,
            entity,
            dimension: "minecraft:overworld".to_owned(),
            position,
            look: [0.0, 0.0, 1.0],
            creative: false,
            game_mode: 0,
            on_ground,
            sneaking,
            gliding: false,
            spectator: false,
            in_water: false,
            on_climbable: false,
            armor: Vec::new(),
            hands: Vec::new(),
            block_light: 0,
            sky_light: 0,
            sky_darken: 0,
            effects: if bouncy { vec![("tconstruct:bouncy".to_owned(), 0, 100)] } else { Vec::new() },
            remove_effects: RefCell::new(Vec::new()),
            attribute_additions: RefCell::new(Vec::new()),
            actions: RefCell::new(Vec::new()),
        }
    }

    #[test]
    fn a_bouncy_fall_bounces_and_the_next_tick_sets_the_velocity() {
        register();
        custom_attributes::register();
        ServerLivingEntityEvents::PLAYER_TICK.invoke(&tick_for(710_001, [0.0, 64.0, 0.0], true, false, true));
        assert!((custom_attributes::value(710_001, BOUNCY) - 1.0).abs() < 1e-9, "the bouncy effect is read");
        let fall = LivingFall { entity: 710_001, kind: "minecraft:player".to_owned(), distance: Cell::new(9.0), multiplier: Cell::new(1.0) };
        let mut world = MemoryWorldOps::default();
        let cancelled = with_world(&mut world, || LivingFallEvents::FALL.invoke(&fall));
        assert!(!cancelled, "the fall is cancelled");
        assert_eq!(fall.distance.get(), 0.0);
        assert_eq!(fall.multiplier.get(), 0.0);
        assert_eq!(world.sounds, vec![("tconstruct:slimy_bounce".to_owned(), [0.0, 64.0, 0.0])], "the bounce sounds at the landing");
        let mut ops = MemoryEntityOps::default();
        ops.entities = vec![([0.5, 64.5, 0.0], WorldEntity { id: 710_001, kind: Identifier::parse("minecraft:player").unwrap(), item: None, living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false, spectator: false }) })];
        with_entities(&mut ops, || {
            ServerLivingEntityEvents::PLAYER_TICK.invoke(&tick_for(710_001, [0.5, 64.5, 0.0], false, false, true));
        });
        assert_eq!(ops.pushed.len(), 1, "the bounce lands as one push");
        assert_eq!(ops.pushed[0].0, 710_001);
        let rise = (9.0 * ENTITY_GRAVITY).sqrt();
        assert!(close(ops.pushed[0].1, [-0.5, rise - 0.5, 0.0]), "the push is the set velocity minus the measured motion: {:?}", ops.pushed[0].1);
    }

    #[test]
    fn a_fall_without_bouncy_stands_and_a_crouch_halves_the_damage() {
        register();
        custom_attributes::register();
        let plain = LivingFall { entity: 710_002, kind: "minecraft:player".to_owned(), distance: Cell::new(9.0), multiplier: Cell::new(1.0) };
        assert!(LivingFallEvents::FALL.invoke(&plain), "no bouncy, no bounce");
        assert_eq!(plain.distance.get(), 9.0);
        assert_eq!(plain.multiplier.get(), 1.0);
        ServerLivingEntityEvents::PLAYER_TICK.invoke(&tick_for(710_003, [0.0, 64.0, 0.0], true, true, true));
        let crouched = LivingFall { entity: 710_003, kind: "minecraft:player".to_owned(), distance: Cell::new(9.0), multiplier: Cell::new(1.0) };
        assert!(LivingFallEvents::FALL.invoke(&crouched), "crouching does not cancel the fall");
        assert_eq!(crouched.distance.get(), 9.0, "the distance stands");
        assert_eq!(crouched.multiplier.get(), 0.5, "the damage is halved");
    }

    #[test]
    fn the_sling_leap_adds_a_momentum_only_handler() {
        // The leap's two ticks, as the player's last tick kept them: the handler reads the
        // kept motion, so the facts are set directly — invoking the tick here would run the
        // tick handler once per register() call already made in this process, and every run
        // after the first would see no movement and zero the kept velocity.
        {
            let mut kept = KEPT.lock().expect("the bounce facts are never poisoned");
            kept.insert(710_004, Kept { position: [2.0, 64.0, 0.0], velocity: [2.0, 0.0, 0.0], tick: 2, sneaking: false, dimension: "minecraft:overworld".to_owned() });
        }
        add_bounce_handler(710_004);
        let bounces = BOUNCES.lock().expect("the bounce state is never poisoned");
        let info = bounces.get(&710_004).expect("the handler is kept");
        assert!(info.bounce.is_none(), "no velocity is set: momentum only");
        assert!((info.last_mag_sq - 4.0).abs() < 1e-9, "the preserved speed is the leap's motion");
    }

    fn mob_tick_for(entity: i32, position: [f64; 3], velocity: [f64; 3], bouncy: bool, gliding: bool) -> MobTick {
        MobTick {
            entity,
            kind: "minecraft:zombie".to_owned(),
            dimension: "minecraft:overworld".to_owned(),
            position,
            velocity,
            on_ground: false,
            in_water: false,
            on_climbable: false,
            climbing: false,
            gliding,
            armor: Vec::new(),
            hands: Vec::new(),
            effects: if bouncy { vec![("tconstruct:bouncy".to_owned(), 0, 100)] } else { Vec::new() },
        }
    }

    #[test]
    fn a_bouncy_mobs_fall_bounces_and_the_next_tick_sets_its_velocity() {
        register();
        custom_attributes::register();
        ServerLivingEntityEvents::MOB_TICK.invoke(&mob_tick_for(710_101, [0.0, 64.0, 0.0], [0.5, -3.0, 0.0], true, false));
        assert!((custom_attributes::value(710_101, BOUNCY) - 1.0).abs() < 1e-9, "the mob's bouncy effect is read from its tick report");
        let fall = LivingFall { entity: 710_101, kind: "minecraft:zombie".to_owned(), distance: Cell::new(9.0), multiplier: Cell::new(1.0) };
        let mut world = MemoryWorldOps::default();
        let cancelled = with_world(&mut world, || LivingFallEvents::FALL.invoke(&fall));
        assert!(!cancelled, "the mob's fall is cancelled");
        assert_eq!(fall.distance.get(), 0.0);
        assert_eq!(fall.multiplier.get(), 0.0);
        assert_eq!(world.sounds, vec![("tconstruct:slimy_bounce".to_owned(), [0.0, 64.0, 0.0])], "the bounce sounds at the landing");
        let mut ops = MemoryEntityOps::default();
        ops.entities = vec![([0.5, 64.5, 0.0], WorldEntity { id: 710_101, kind: Identifier::parse("minecraft:zombie").unwrap(), item: None, living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false, spectator: false }) })];
        with_entities(&mut ops, || {
            ServerLivingEntityEvents::MOB_TICK.invoke(&mob_tick_for(710_101, [0.5, 64.5, 0.0], [0.5, -3.0, 0.0], true, false));
        });
        assert_eq!(ops.velocities.len(), 1, "the bounce sets the mob's velocity outright");
        assert_eq!(ops.velocities[0].0, 710_101);
        assert!(close(ops.velocities[0].1, [0.5 / 0.975, 2.7, 0.0]), "the non-player flip of the reported motion: {:?}", ops.velocities[0].1);
        assert!(ops.pushed.is_empty(), "a set that lands needs no push");
    }

    #[test]
    fn a_mob_without_bouncy_falls_and_an_unloaded_mobs_handler_is_forgotten() {
        register();
        custom_attributes::register();
        ServerLivingEntityEvents::MOB_TICK.invoke(&mob_tick_for(710_102, [0.0, 64.0, 0.0], [0.0, -3.0, 0.0], false, false));
        let plain = LivingFall { entity: 710_102, kind: "minecraft:zombie".to_owned(), distance: Cell::new(9.0), multiplier: Cell::new(1.0) };
        assert!(LivingFallEvents::FALL.invoke(&plain), "no bouncy, no bounce");
        assert_eq!(plain.distance.get(), 9.0);
        assert_eq!(plain.multiplier.get(), 1.0);
        // A bouncy mob that unloads leaves neither facts nor a handler behind.
        ServerLivingEntityEvents::MOB_TICK.invoke(&mob_tick_for(710_103, [0.0, 64.0, 0.0], [0.0, -3.0, 0.0], true, false));
        let fall = LivingFall { entity: 710_103, kind: "minecraft:zombie".to_owned(), distance: Cell::new(9.0), multiplier: Cell::new(1.0) };
        assert!(!LivingFallEvents::FALL.invoke(&fall), "the bouncy mob's fall is cancelled");
        assert!(BOUNCES.lock().expect("the bounce state is never poisoned").contains_key(&710_103), "the handler is kept");
        ServerEntityEvents::ENTITY_UNLOAD.invoke(&EntityInfo { id: 710_103, kind: "minecraft:zombie".to_owned(), dimension: "minecraft:overworld".to_owned() });
        assert!(!BOUNCES.lock().expect("the bounce state is never poisoned").contains_key(&710_103), "the handler goes with the mob");
        assert!(!KEPT.lock().expect("the bounce facts are never poisoned").contains_key(&710_103), "the facts go with the mob");
    }

    #[test]
    fn a_gliding_mobs_bounce_ends_at_once() {
        register();
        add_bounce_handler(710_104);
        assert!(BOUNCES.lock().expect("the bounce state is never poisoned").contains_key(&710_104), "the handler is kept");
        ServerLivingEntityEvents::MOB_TICK.invoke(&mob_tick_for(710_104, [0.0, 64.0, 0.0], [0.0, -3.0, 0.0], true, true));
        assert!(!BOUNCES.lock().expect("the bounce state is never poisoned").contains_key(&710_104), "gliding ends the handler at once");
    }
}
