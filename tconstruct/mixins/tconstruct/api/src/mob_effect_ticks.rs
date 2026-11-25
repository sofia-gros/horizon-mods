//! The per-tick behaviour of the Tinkers mob effects: magnetic, repulsive, returning, antigravity,
//! conductive and venom (the original's `MagneticEffect`, `RepulsiveEffect`, `ReturningEffect`,
//! `AntigravityEffect` and the conductive and venom arms of `ToolEvents.livingHurt`).
//! Enderference is a marker only: it has no per-tick behaviour, and its projectile-hit gate lives
//! in the enderference lane, not here. `teleport_cooldown` is declaration-only by design, as in
//! the original, and has no tick behaviour either.
//!
//! A mob's tick is heard on `ENTITY_TICK_PRE`, before the mob moves, so the durations read are the
//! original's: the host counts an effect down inside the tick, and removes a spent one before the
//! tick after it. A player's tick is heard at its end, after their effects counted down, so the
//! durations read are one less than the original's; the tick adds one back (an infinite one, -1,
//! stays -1), and the returning trigger reads one higher for a player, so the teleport lands
//! whether the countdown ran before or after the hook.
//!
//! The effects a bearer carries are remembered on every tick, for the damage event to read when a
//! hit carries no state of its own (a mob hit by anything but a player's weapon, or a player
//! wearing no armor); the level remembered is the original's, the amplifier plus one.
//!
//! What the reach cannot do is approximated, and noted where it happens: the magnetic pull adds
//! 0.04 up to every target, as the original does only to one that falls (the reach names no
//! no-gravity flag); the ceiling's friction is the vanilla default 0.6 (the reach names no
//! friction); the ceiling bounce does not fire the ceiling block's `fallOn` or
//! `updateEntityAfterFallOn` (no consumer verb fires them); the returning teleport has no portal
//! particles or enderporting sound (the teleport dodge skips them too); the jump reversal and the
//! ladder handling of antigravity are not wired (no consumer verb reads jumping or climbing); and
//! the velocities read are the ones the last tick ended with, so the ceiling record runs one tick
//! behind the original's.
use pluto_lib::{EntityInfo, EntityReach, PlayerTick, ServerEntityEvents, ServerLivingEntityEvents, World};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// The magnetic effect: it pulls item entities in (the original's `MagneticEffect`).
pub(crate) const MAGNETIC: &str = "tconstruct:magnetic";
/// The repulsive effect: it pushes living entities away (the original's `RepulsiveEffect`).
pub(crate) const REPULSIVE: &str = "tconstruct:repulsive";
/// The returning effect: it teleports the bearer home on the last tick (the original's `ReturningEffect`).
pub(crate) const RETURNING: &str = "tconstruct:returning";
/// The antigravity effect: the bearer falls up (the original's `AntigravityEffect`).
pub(crate) const ANTIGRAVITY: &str = "tconstruct:antigravity";
/// The conductive effect: fire damage on the bearer is multiplied (the original's `ToolEvents.livingHurt`).
pub(crate) const CONDUCTIVE: &str = "tconstruct:conductive";
/// The venom effect: magic damage on the bearer is multiplied (the original's `ToolEvents.livingHurt`).
pub(crate) const VENOM: &str = "tconstruct:venom";

/// The gravity a mob falls with when the host names no other (`MobType.gravity`, the only value the host writes).
const GRAVITY: f64 = 0.08;
/// The height of a player: their bounding box is 1.8 tall.
const PLAYER_HEIGHT: f64 = 1.8;
/// The friction of a ceiling when the reach names none (vanilla 0.6; ice is 0.98, slime 0.8).
const CEILING_FRICTION: f64 = 0.6;
/// How far above the build height the original hurts a bearer that fell out of the world.
const OUT_OF_WORLD_ABOVE: i32 = 5780;
/// The damage of falling out of the world (`DamageSources.fellOutOfWorld`, 4.0).
const OUT_OF_WORLD_DAMAGE: f32 = 4.0;
/// The damage type of falling out of the world.
const OUT_OF_WORLD: &str = "minecraft:out_of_world";

/// The effects each entity carried on its last tick (effect id, level), for the damage event to read
/// when the hit carries no state of its own.
static REMEMBERED: LazyLock<Mutex<HashMap<i32, Vec<(String, i32)>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
/// Where a returning bearer stood when the effect was first seen (block position, dimension), the
/// original's persistent `tconstruct:returning` data.
static RETURNING_FROM: LazyLock<Mutex<HashMap<i32, ([i32; 3], String)>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
/// The last two velocities of an antigravity bearer, for the ceiling bounce (`LastVelocity` in the original).
static LAST_VELOCITY: LazyLock<Mutex<HashMap<i32, (f64, f64)>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
/// Where each entity stood on its last tick, for the ceiling check (the host names no ceiling collision).
static PREVIOUS_POSITION: LazyLock<Mutex<HashMap<i32, [f64; 3]>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Registers the tick callbacks. Called once from the product's hooks.
pub(crate) fn register() {
    ServerEntityEvents::ENTITY_TICK_PRE.register(|info| {
        tick_entity(info);
        true
    });
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| tick_player(tick));
    ServerEntityEvents::ENTITY_UNLOAD.register(|info| prune(info.id as i32));
}

/// The level of the effect `effect` the entity `entity` carried on its last tick (the amplifier plus
/// one), or 0 when it carried none. The damage event reads this when the hit carries no state.
pub(crate) fn remembered_level(entity: i32, effect: &str) -> i32 {
    REMEMBERED
        .lock()
        .expect("remembered effects poisoned")
        .get(&entity)
        .map_or(0, |effects| effects.iter().find(|(id, _)| id == effect).map_or(0, |(_, level)| *level))
}

/// The damage multiplier for a conductive or venom level (`Math.pow(2, level)` in the original, 1
/// when the effect is not carried). The level is the original's, the amplifier plus one.
pub(crate) fn conductive_multiplier(level: i32) -> f32 {
    if level > 0 {
        2f32.powi(level)
    } else {
        1.0
    }
}

/// Forgets everything remembered about the entity `entity`: it unloaded, and a new entity may reuse its id.
fn prune(entity: i32) {
    REMEMBERED.lock().expect("remembered effects poisoned").remove(&entity);
    RETURNING_FROM.lock().expect("returning store poisoned").remove(&entity);
    LAST_VELOCITY.lock().expect("last velocity poisoned").remove(&entity);
    PREVIOUS_POSITION.lock().expect("previous position poisoned").remove(&entity);
}

/// Remembers the effects `effects` carries, for the damage event; an empty list clears what was remembered.
fn remember(entity: i32, effects: &[(String, i32, i32)]) {
    let levels: Vec<(String, i32)> =
        effects.iter().map(|(id, amplifier, _)| (id.clone(), amplifier.saturating_add(1))).collect();
    REMEMBERED.lock().expect("remembered effects poisoned").insert(entity, levels);
}

/// The per-tick behaviour of the effects a mob carries.
fn tick_entity(info: &EntityInfo) {
    let effects = EntityReach::effects_of(u64::from(info.id));
    remember(info.id as i32, &effects);
    if effects.is_empty() {
        return;
    }
    let Some(position) = EntityReach::position_of(u64::from(info.id)) else { return };
    tick_bearer(info.id as i32, &info.kind, &info.dimension, &effects, position, false);
}

/// The per-tick behaviour of the effects the acting player carries.
fn tick_player(tick: &PlayerTick) {
    remember(tick.entity, &tick.effects);
    if tick.effects.is_empty() {
        return;
    }
    tick_bearer(tick.entity, "minecraft:player", &tick.dimension, &tick.effects, tick.position, true);
}

/// Runs the per-tick behaviour of the effects `effects` carries, for the entity `id` of `kind` in
/// `dimension`, standing at `position`. `player` says the bearer is the acting player, whose
/// durations read one less than the original's. Conductive and venom have no tick arm: they run on
/// the hurt path (protection_hook). Enderference is a marker only.
fn tick_bearer(id: i32, kind: &str, dimension: &str, effects: &[(String, i32, i32)], position: [f64; 3], player: bool) {
    for (effect, amplifier, ticks_left) in effects {
        let duration = if *ticks_left < 0 { *ticks_left } else { ticks_left + i32::from(player) };
        match effect.as_str() {
            MAGNETIC => magnetic(*amplifier, duration, position),
            REPULSIVE => repulsive(*amplifier, duration, position),
            RETURNING => returning(id, dimension, duration, position, player),
            ANTIGRAVITY => antigravity(id, kind, dimension, position),
            _ => {}
        }
    }
    if let Ok(mut previous) = PREVIOUS_POSITION.lock() {
        previous.insert(id, position);
    }
}

/// Magnetic: every other tick, the items around the bearer are pulled in (the original's
/// `MagneticEffect`, range 3, speed 0.05, at most 100 pulled).
fn magnetic(amplifier: i32, duration: i32, position: [f64; 3]) {
    if (duration & 1) != 0 {
        return;
    }
    apply_velocity(position, amplifier, true, 3.0, 0.05, 100);
}

/// Repulsive: every other tick, the living entities around the bearer are pushed away (the
/// original's `RepulsiveEffect`, range 2, speed -0.1, at most 10 pushed).
fn repulsive(amplifier: i32, duration: i32, position: [f64; 3]) {
    if (duration & 1) != 0 {
        return;
    }
    apply_velocity(position, amplifier, false, 2.0, -0.1, 10);
}

/// The original's `MagneticEffect.applyVelocity`: the entities of `items` (item entities) or of living
/// kinds around `origin` are pushed toward it (`speed` positive) or away (`speed` negative), at most
/// `max_push` of them. An item's motion is set at once (the queued push cannot reach one); a living
/// entity is pushed, which the host applies at its next rider synchronisation, so the motion lands a
/// tick after the call.
fn apply_velocity(origin: [f64; 3], amplifier: i32, items: bool, min_range: f64, speed: f64, max_push: i32) {
    let range = min_range + f64::from(amplifier);
    let min = [origin[0] - range, origin[1] - range, origin[2] - range];
    let max = [origin[0] + range, origin[1] + range, origin[2] + range];
    let mut pulled = 0;
    for target in EntityReach::in_box(min, max) {
        if items {
            if target.item.is_none() {
                continue;
            }
        } else if target.living.is_none() {
            continue;
        }
        let Some(position) = EntityReach::position_of(target.id) else { continue };
        let Some(push) = pull_vector(origin, position, amplifier, speed) else { continue };
        if items {
            if let Some(velocity) = EntityReach::velocity_of(target.id) {
                EntityReach::set_velocity(target.id, [velocity[0] + push[0], velocity[1] + push[1], velocity[2] + push[2]]);
            }
        } else {
            EntityReach::push(target.id, push);
        }
        pulled += 1;
        if pulled > max_push {
            break;
        }
    }
}

/// The push the original gives one target: the direction from the target to the origin, scaled to
/// `speed * (amplifier + 1)`, with 0.04 up. None when the target stands on the origin (the
/// original's 0.25), which skips the bearer itself.
fn pull_vector(origin: [f64; 3], target: [f64; 3], amplifier: i32, speed: f64) -> Option<[f64; 3]> {
    let delta = [origin[0] - target[0], origin[1] - target[1], origin[2] - target[2]];
    let distance_squared = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
    if distance_squared < 0.25 {
        return None;
    }
    let factor = speed * f64::from(amplifier + 1) / distance_squared.sqrt();
    Some([delta[0] * factor, delta[1] * factor + 0.04, delta[2] * factor])
}

/// Whether the returning effect fires this tick: the original fires on the last tick (duration 1).
/// A player's tick is heard at its end, after their effects counted down, so the last one the tick
/// can still see is one higher.
fn returning_trigger(duration: i32, player: bool) -> bool {
    duration == if player { 2 } else { 1 }
}

/// Returning: on the first tick the effect is seen, where the bearer stands and which dimension they
/// are in is remembered (the original stores it when the effect is added, as the block position); on
/// the last tick they are teleported back, in the same dimension only.
fn returning(id: i32, dimension: &str, duration: i32, position: [f64; 3], player: bool) {
    let stored = {
        let mut from = RETURNING_FROM.lock().expect("returning store poisoned");
        from.entry(id)
            .or_insert_with(|| {
                ([position[0].floor() as i32, position[1].floor() as i32, position[2].floor() as i32], dimension.to_owned())
            })
            .clone()
    };
    if !returning_trigger(duration, player) {
        return;
    }
    if stored.1 != dimension {
        return;
    }
    RETURNING_FROM.lock().expect("returning store poisoned").remove(&id);
    // The original teleports to the exact block coordinates, with no centering; the reach's
    // teleport moves the bearer by the game's own server teleport, which leaves their vehicle.
    EntityReach::teleport(id as u64, [f64::from(stored.0[0]), f64::from(stored.0[1]), f64::from(stored.0[2])]);
}

/// Antigravity: the bearer falls up. The host applies no attribute modifier for the effect, so the
/// pull is undone here: each tick the velocity grows by twice the gravity the host's travel takes
/// off, which leaves it rising by the gravity each tick, as the original's reversed gravity does.
fn antigravity(id: i32, kind: &str, dimension: &str, position: [f64; 3]) {
    EntityReach::push(id as u64, [0.0, 2.0 * GRAVITY, 0.0]);
    out_of_world(id, dimension, position);
    if let Some(velocity) = EntityReach::velocity_of(id as u64) {
        ceiling_bounce(id, kind, position, velocity);
    }
}

/// The original hurts a bearer that rises above the build height by 5780 (`fellOutOfWorld`, 4.0).
fn out_of_world(id: i32, dimension: &str, position: [f64; 3]) {
    if position[1] > out_of_world_y(dimension) {
        EntityReach::hurt(id as u64, OUT_OF_WORLD_DAMAGE, OUT_OF_WORLD);
    }
}

/// The height the original builds to in a dimension (`Level.getMaxBuildHeight`, the vanilla shapes).
fn max_build_height(dimension: &str) -> i32 {
    match dimension {
        "minecraft:the_nether" | "minecraft:the_end" => 256,
        _ => 320,
    }
}

/// The height above which the original hurts a bearer that fell out of the world (`maxBuildHeight + 5780`).
fn out_of_world_y(dimension: &str) -> f64 {
    f64::from(max_build_height(dimension) + OUT_OF_WORLD_ABOVE)
}

/// The ceiling bounce: a bearer pressed to a ceiling while rising is sent back down with the velocity
/// it rose with two ticks ago, as the original's `LastVelocity` does. The push is the delta to that
/// velocity, scaled along the ceiling by its friction.
fn ceiling_bounce(id: i32, kind: &str, position: [f64; 3], velocity: [f64; 3]) {
    let (last_tick, two_ticks) = last_velocity(id);
    if !ceiling_blocked(id, kind, position, two_ticks) {
        update_last_velocity(id, velocity[1]);
        return;
    }
    let mut push = [velocity[0] * (CEILING_FRICTION - 1.0), 0.0, velocity[2] * (CEILING_FRICTION - 1.0)];
    if two_ticks > 0.1 {
        push[1] = two_ticks - velocity[1];
        update_last_velocity(id, 0.0);
    } else {
        update_last_velocity(id, velocity[1]);
    }
    EntityReach::push(id as u64, push);
}

/// The last two velocities of the bearer `id`, as the original's `LastVelocity` keeps them: the
/// velocity of the last tick and of the one before.
fn last_velocity(id: i32) -> (f64, f64) {
    LAST_VELOCITY.lock().expect("last velocity poisoned").get(&id).copied().unwrap_or((0.0, 0.0))
}

/// Shifts the bearer's velocity record by one tick, with `velocity_y` the latest (`LastVelocity.update`).
fn update_last_velocity(id: i32, velocity_y: f64) {
    let mut last = LAST_VELOCITY.lock().expect("last velocity poisoned");
    let entry = last.entry(id).or_insert((0.0, 0.0));
    entry.1 = entry.0;
    entry.0 = velocity_y;
}

/// Whether the bearer is pressed to a ceiling: it rose two ticks ago, rose less than half that last
/// tick, and the block above its head is not air. The host names no ceiling collision, so the rise
/// is read from the position of the last tick.
fn ceiling_blocked(id: i32, kind: &str, position: [f64; 3], two_ticks: f64) -> bool {
    if two_ticks <= 0.1 {
        return false;
    }
    let Some(previous) = PREVIOUS_POSITION.lock().expect("previous position poisoned").get(&id).copied() else {
        return false;
    };
    if position[1] - previous[1] >= two_ticks * 0.5 {
        return false;
    }
    let Some(head) = head_y(id, kind, position) else { return false };
    let above = [position[0].floor() as i32, (head + 0.1).floor() as i32, position[2].floor() as i32];
    let Some(state) = World::ticking().and_then(|world| world.get_block_state(above)) else { return false };
    !is_air(&state.block.to_string())
}

/// The top of the bearer's bounding box: the reach reads it for a mob; a player is 1.8 tall.
fn head_y(id: i32, kind: &str, position: [f64; 3]) -> Option<f64> {
    if kind == "minecraft:player" {
        return Some(position[1] + PLAYER_HEIGHT);
    }
    EntityReach::bounds_of(id as u64).map(|bounds| bounds[1][1])
}

/// Whether the block `block` is air, in any of its three forms.
fn is_air(block: &str) -> bool {
    matches!(block, "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{
        with_entities, with_ticking_dimension, with_world, Identifier, LivingFacts, MemoryEntityOps, MemoryWorldOps,
        WorldEntity,
    };
    use std::cell::RefCell;

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    fn living(kind: &str) -> WorldEntity {
        WorldEntity {
            id: 0,
            kind: id(kind),
            item: None,
            living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
        }
    }

    fn item(entity: u64, at: [f64; 3]) -> ([f64; 3], WorldEntity) {
        (
            at,
            WorldEntity {
                id: entity,
                kind: id("minecraft:item"),
                item: Some(pluto_lib::ItemSlot::new(id("minecraft:iron_ingot"), 1)),
                living: None,
            },
        )
    }

    fn info(id: u32, kind: &str) -> EntityInfo {
        EntityInfo { id, kind: kind.to_owned(), dimension: "minecraft:overworld".to_owned() }
    }

    fn player_tick(entity: i32, position: [f64; 3], effects: Vec<(String, i32, i32)>) -> PlayerTick {
        PlayerTick {
            player: 1,
            entity,
            dimension: "minecraft:overworld".into(),
            position,
            look: [0.0, 0.0, 1.0],
            creative: false,
            game_mode: 0,
            on_ground: true,
            sneaking: false,
            gliding: false,
            spectator: false,
            in_water: false,
            on_climbable: false,
            armor: Vec::new(),
            hands: Vec::new(),
            block_light: 0,
            sky_light: 15,
            sky_darken: 0,
            effects,
            remove_effects: RefCell::new(Vec::new()),
            attribute_additions: RefCell::new(Vec::new()),
            actions: RefCell::new(Vec::new()),
        }
    }

    #[test]
    fn the_pull_points_at_the_bearer_and_scales_with_the_amplifier() {
        assert_eq!(pull_vector([0.0, 64.0, 0.0], [2.0, 64.0, 0.0], 0, 0.05), Some([-0.05, 0.04, 0.0]));
        assert_eq!(pull_vector([0.0, 64.0, 0.0], [2.0, 64.0, 0.0], 1, 0.05), Some([-0.1, 0.04, 0.0]));
        assert_eq!(pull_vector([0.0, 64.0, 0.0], [2.0, 64.0, 0.0], 0, -0.1), Some([0.1, 0.04, 0.0]));
        assert_eq!(pull_vector([0.0, 64.0, 0.0], [0.4, 64.0, 0.0], 0, 0.05), None, "on top of the bearer, and the bearer itself");
        assert_eq!(pull_vector([0.0, 64.0, 0.0], [0.0, 64.0, 0.0], 0, 0.05), None);
    }

    #[test]
    fn returning_fires_on_the_last_tick_and_one_higher_for_a_player() {
        assert!(returning_trigger(1, false));
        assert!(!returning_trigger(2, false));
        assert!(returning_trigger(2, true));
        assert!(!returning_trigger(1, true));
    }

    #[test]
    fn the_conductive_multiplier_doubles_per_level() {
        assert_eq!(conductive_multiplier(0), 1.0);
        assert_eq!(conductive_multiplier(1), 2.0);
        assert_eq!(conductive_multiplier(2), 4.0);
        assert_eq!(conductive_multiplier(3), 8.0);
        assert_eq!(conductive_multiplier(-1), 1.0, "no effect, no multiplier");
    }

    #[test]
    fn out_of_world_height_is_the_build_height_plus_5780() {
        assert_eq!(out_of_world_y("minecraft:overworld"), 6100.0);
        assert_eq!(out_of_world_y("minecraft:the_nether"), 6036.0);
        assert_eq!(out_of_world_y("minecraft:the_end"), 6036.0);
        assert_eq!(max_build_height("minecraft:overworld"), 320);
    }

    #[test]
    fn magnetic_pulls_the_items_around_the_bearer_and_only_those() {
        let mut ops = MemoryEntityOps::default();
        let mut bearer = living("minecraft:zombie");
        bearer.id = 7101;
        ops.entities.push(([0.0, 64.0, 0.0], bearer));
        ops.entities.push(item(7102, [2.0, 64.0, 0.0]));
        ops.entities.push(item(7103, [100.0, 64.0, 0.0]));
        ops.effect_lists = vec![(7101, vec![("tconstruct:magnetic".to_owned(), 0, 120)])];
        ops.velocities = vec![(7102, [0.0, 0.0, 0.0]), (7103, [0.0, 0.0, 0.0])];
        with_entities(&mut ops, || tick_entity(&info(7101, "minecraft:zombie")));
        assert_eq!(ops.velocities.iter().find(|(id, _)| *id == 7102).map(|(_, velocity)| *velocity), Some([-0.05, 0.04, 0.0]));
        assert_eq!(ops.velocities.iter().find(|(id, _)| *id == 7103).map(|(_, velocity)| *velocity), Some([0.0, 0.0, 0.0]), "out of range, untouched");
        assert!(ops.pushed.is_empty(), "an item is set, not pushed");
        // An odd duration is not a tick: nothing moves again.
        ops.effect_lists = vec![(7101, vec![("tconstruct:magnetic".to_owned(), 0, 121)])];
        with_entities(&mut ops, || tick_entity(&info(7101, "minecraft:zombie")));
        assert_eq!(ops.velocities.iter().find(|(id, _)| *id == 7102).map(|(_, velocity)| *velocity), Some([-0.05, 0.04, 0.0]));
        // An infinite effect never ticks, as the original's bit test says.
        ops.effect_lists = vec![(7101, vec![("tconstruct:magnetic".to_owned(), 0, -1)])];
        with_entities(&mut ops, || tick_entity(&info(7101, "minecraft:zombie")));
        assert_eq!(ops.velocities.iter().find(|(id, _)| *id == 7102).map(|(_, velocity)| *velocity), Some([-0.05, 0.04, 0.0]));
    }

    #[test]
    fn repulsive_pushes_the_living_away_but_not_the_bearer_or_an_item() {
        let mut ops = MemoryEntityOps::default();
        let mut bearer = living("minecraft:zombie");
        bearer.id = 7111;
        ops.entities.push(([0.0, 64.0, 0.0], bearer));
        let mut other = living("minecraft:zombie");
        other.id = 7112;
        ops.entities.push(([2.0, 64.0, 0.0], other));
        ops.entities.push(item(7113, [1.0, 64.0, 0.0]));
        ops.effect_lists = vec![(7111, vec![("tconstruct:repulsive".to_owned(), 0, 120)])];
        with_entities(&mut ops, || tick_entity(&info(7111, "minecraft:zombie")));
        assert_eq!(ops.pushed, vec![(7112, [0.1, 0.04, 0.0])]);
    }

    #[test]
    fn returning_teleports_home_on_the_last_tick_and_not_across_dimensions() {
        let mut ops = MemoryEntityOps::default();
        let mut home = living("minecraft:zombie");
        home.id = 7121;
        ops.entities.push(([10.0, 64.0, 10.0], home));
        let mut away = living("minecraft:zombie");
        away.id = 7122;
        ops.entities.push(([10.0, 64.0, 10.0], away));
        ops.effect_lists = vec![
            (7121, vec![("tconstruct:returning".to_owned(), 0, 10)]),
            (7122, vec![("tconstruct:returning".to_owned(), 0, 10)]),
        ];
        with_entities(&mut ops, || {
            tick_entity(&info(7121, "minecraft:zombie"));
            tick_entity(&info(7122, "minecraft:zombie"));
        });
        // Both moved away before the last tick; only the one that stayed in the dimension goes home.
        for entity in ops.entities.iter_mut() {
            entity.0 = [20.0, 64.0, 20.0];
        }
        ops.effect_lists = vec![
            (7121, vec![("tconstruct:returning".to_owned(), 0, 1)]),
            (7122, vec![("tconstruct:returning".to_owned(), 0, 1)]),
        ];
        with_entities(&mut ops, || {
            tick_entity(&info(7121, "minecraft:zombie"));
            let mut nether = info(7122, "minecraft:zombie");
            nether.dimension = "minecraft:the_nether".to_owned();
            tick_entity(&nether);
        });
        assert_eq!(EntityReach::position_of(7121), None, "outside a scope nothing is read");
        with_entities(&mut ops, || {
            assert_eq!(EntityReach::position_of(7121), Some([10.0, 64.0, 10.0]), "home, to the exact block coordinates");
            assert_eq!(EntityReach::position_of(7122), Some([20.0, 64.0, 20.0]), "another dimension, no teleport");
        });
    }

    #[test]
    fn antigravity_pushes_up_twice_the_gravity_and_hurts_above_the_world() {
        let mut ops = MemoryEntityOps::default();
        let mut bearer = living("minecraft:zombie");
        bearer.id = 7131;
        ops.entities.push(([0.0, 6101.0, 0.0], bearer));
        ops.effect_lists = vec![(7131, vec![("tconstruct:antigravity".to_owned(), 0, 100)])];
        ops.velocities = vec![(7131, [0.0, 0.3, 0.0])];
        with_entities(&mut ops, || tick_entity(&info(7131, "minecraft:zombie")));
        assert_eq!(ops.pushed, vec![(7131, [0.0, 0.16, 0.0])], "twice the gravity, so the rise is one gravity a tick");
        assert_eq!(ops.hurt, vec![(7131, 4.0, "minecraft:out_of_world".to_owned())]);
    }

    #[test]
    fn the_ceiling_bounce_restores_the_rise_from_two_ticks_ago() {
        let mut ops = MemoryEntityOps::default();
        let mut bearer = living("minecraft:zombie");
        bearer.id = 7141;
        ops.entities.push(([0.0, 69.76, 0.0], bearer));
        ops.effect_lists = vec![(7141, vec![("tconstruct:antigravity".to_owned(), 0, 100)])];
        ops.velocities = vec![(7141, [0.0, 0.24, 0.0])];
        ops.bounding_boxes = vec![(7141, [[-0.3, 70.0, -0.3], [0.3, 71.8, 0.3]])];
        let mut world_ops = MemoryWorldOps::default();
        world_ops.blocks.insert(("minecraft:overworld".to_owned(), [0, 71, 0]), "minecraft:stone".to_owned());
        with_ticking_dimension("minecraft:overworld", || {
            with_world(&mut world_ops, || {
                with_entities(&mut ops, || tick_entity(&info(7141, "minecraft:zombie")));
                // Rising: the bearer gains a quarter block, and the record fills.
                ops.entities.iter_mut().find(|(_, entity)| entity.id == 7141).unwrap().0 = [0.0, 70.0, 0.0];
                with_entities(&mut ops, || tick_entity(&info(7141, "minecraft:zombie")));
                // Pressed to the ceiling: no rise, and the host zeroed the upward motion.
                ops.velocities.iter_mut().find(|(id, _)| *id == 7141).unwrap().1 = [0.0, 0.0, 0.0];
                with_entities(&mut ops, || tick_entity(&info(7141, "minecraft:zombie")));
            })
        });
        assert_eq!(ops.pushed.len(), 4, "three compensations and the bounce");
        assert_eq!(ops.pushed[3], (7141, [0.0, 0.24, 0.0]), "the rise from two ticks ago, restored");
    }

    #[test]
    fn the_player_returns_on_the_tick_their_report_can_still_see() {
        let mut ops = MemoryEntityOps::default();
        let mut player = living("minecraft:player");
        player.id = 7151;
        ops.entities.push(([10.0, 64.0, 10.0], player));
        with_entities(&mut ops, || {
            tick_player(&player_tick(7151, [10.0, 64.0, 10.0], vec![("tconstruct:returning".to_owned(), 0, 10)]));
        });
        // The player walked away before the last tick.
        ops.entities.iter_mut().find(|(_, entity)| entity.id == 7151).unwrap().0 = [20.0, 64.0, 20.0];
        with_entities(&mut ops, || {
            tick_player(&player_tick(7151, [20.0, 64.0, 20.0], vec![("tconstruct:returning".to_owned(), 0, 1)]));
            assert_eq!(EntityReach::position_of(7151), Some([10.0, 64.0, 10.0]), "home: the report said 1, the tick read 2");
        });
    }

    #[test]
    fn remembered_levels_follow_the_effects_and_clear_when_they_end() {
        let mut ops = MemoryEntityOps::default();
        let mut bearer = living("minecraft:zombie");
        bearer.id = 7161;
        ops.entities.push(([0.0, 64.0, 0.0], bearer));
        ops.effect_lists = vec![(7161, vec![("tconstruct:magnetic".to_owned(), 1, 120)])];
        with_entities(&mut ops, || tick_entity(&info(7161, "minecraft:zombie")));
        assert_eq!(remembered_level(7161, "tconstruct:magnetic"), 2, "the amplifier plus one, as the original's level");
        assert_eq!(remembered_level(7161, "tconstruct:repulsive"), 0, "not carried");
        ops.effect_lists = Vec::new();
        with_entities(&mut ops, || tick_entity(&info(7161, "minecraft:zombie")));
        assert_eq!(remembered_level(7161, "tconstruct:magnetic"), 0, "the effect ended, the level does not linger");
    }
}
