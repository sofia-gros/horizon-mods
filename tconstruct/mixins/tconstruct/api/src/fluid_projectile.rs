//! The fluid effect projectile (SlimeKnights FluidEffectProjectile, MIT; see LICENSE): a spit-sized
//! projectile carrying a fluid stack, which applies the fluid's effects where it lands — to the
//! entity it hits (keeping what it did not spend, crossbow-piercing style, until the fluid runs
//! out), or to the block it hits and then its neighbours in the order nearest its flight.
//!
//! The type is declared derived from the game's llama spit, whose flight it shares (the original is
//! a llama spit in movement: drag 0.99 in air, gravity -0.06), with the original's own numbers:
//! 0.25 by 0.25, tracking range 4 chunks, no velocity updates. The declaration is published into
//! the game's entity tables by PlutoLib's entity type provider (its content-overlay export clones
//! the llama spit's catalog row under the new id) and the spawn goes through the game's summon,
//! which loads any published kind; a cannon whose shot cannot spawn (the kind unpublished, the
//! chunk unloaded) fails the shot without spending fluid (see `fluid_cannon`). The spawn carries
//! the original's saved data (`fluid`, `power`, `knockback`, `water_inertia`, `cannon`).
//!
//! In flight, a shot is followed in memory by the id PlutoLib's spawn returned: the impact report
//! carries no saved data to read the fluid back from, so the record is the port's stand-in for the
//! entity's own data, and a shot in flight across a restart is no longer followed (its impacts pass
//! as any other projectile's). The flight telling ends a shot that flies above the build range's
//! max (the original's above-max discard), so a shot that can never come back does not linger.
//! Two of the original's behaviours have no reach and are not faked:
//! the inherent knockback push on an entity hit (a push by an arbitrary motion vector is not a
//! PlutoLib surface — see `fluid_world`), and writing the context stack back into the cannon's item
//! slot when a hit spends it (a block entity's items are only reachable inside its own callbacks;
//! the stack the context runs with is the one the cannon held at the shot, as the original copies
//! it out of the cannon, only at hit time rather than launch time).
use crate::common;
use crate::fluid_effects::{self, EntityContext, Env, Stack};
use crate::fluid_ingredient::FluidTags;
use crate::fluid_world::{block_context, holder, LiveTarget, LiveWorld};
use pluto_lib::{
    BlockPos, Direction, EntityTypeRegistry, FluidStack, Identifier, Nbt, NbtCompound, ProjectileEvents, ProjectileFlight, ProjectileImpact,
    ProjectileSpawn, World, spawn_projectile,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock};

/// The projectile's entity kind (`TinkerModifiers` fluid_spit).
pub(crate) const FLUID_SPIT: &str = "tconstruct:fluid_spit";
/// The knockback the original's projectile carries from the cannon (its field's default).
const KNOCKBACK: f32 = 1.0;
/// The water inertia the original's synched data starts at.
const WATER_INERTIA: f32 = 0.6;

/// A shot in flight: the fluid it still carries, the power it was fired with, the cannon that fired
/// it, and the cannon's item stack as the shot was fired (the context's fallback stack).
#[derive(Clone, Debug)]
struct Shot {
    fluid: FluidStack,
    power: f32,
    cannon: Option<BlockPos>,
    stack: Stack,
}

/// The shots in flight, by dimension and the entity id the spawn returned.
static SHOTS: Mutex<BTreeMap<(String, u32), Shot>> = Mutex::new(BTreeMap::new());

fn shots() -> std::sync::MutexGuard<'static, BTreeMap<(String, u32), Shot>> {
    SHOTS.lock().expect("fluid shot registry poisoned")
}

/// `Projectile.shoot` as the cannon calls it: the facing step with the +0.1 rise, normalized, each
/// axis jittered by a gaussian times 0.0075 times the inaccuracy, the whole scaled by the velocity
/// (the arrow-dispenser numbers the original fires with).
pub(crate) fn shot_motion(step: [i32; 3], velocity: f32, inaccuracy: f32, gaussians: [f64; 3]) -> [f64; 3] {
    let direction = [step[0] as f64, step[1] as f64 + 0.1, step[2] as f64];
    let length = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
    let jitter = 0.0075 * inaccuracy as f64;
    [
        (direction[0] / length + gaussians[0] * jitter) * velocity as f64,
        (direction[1] / length + gaussians[1] * jitter) * velocity as f64,
        (direction[2] / length + gaussians[2] * jitter) * velocity as f64,
    ]
}

/// Three gaussians for a shot, Box-Muller over the world's uniform random (the original draws its
/// jitter from the level's gaussian random; the world reach hands out uniforms).
pub(crate) fn gaussians(world: &World) -> [f64; 3] {
    fn uniform(world: &World) -> f64 {
        world.random_float().map_or(0.5, |value| value as f64).clamp(1e-9, 1.0 - 1e-9)
    }
    fn pair(world: &World) -> (f64, f64) {
        let radius = (-2.0 * uniform(world).ln()).sqrt();
        let angle = 2.0 * std::f64::consts::PI * uniform(world);
        (radius * angle.cos(), radius * angle.sin())
    }
    let (first, second) = pair(world);
    let (third, _) = pair(world);
    [first, second, third]
}

/// The fluid a projectile carries, in the game's saved layout (`FluidStack.writeToNBT`: the name,
/// the amount, and the tag when it has one).
fn fluid_nbt(fluid: &FluidStack) -> NbtCompound {
    let mut tag = NbtCompound::new();
    tag.insert("FluidName".to_owned(), Nbt::String(fluid.fluid.to_string()));
    tag.insert("Amount".to_owned(), Nbt::Int(fluid.amount.min(i32::MAX as u32) as i32));
    if let Some(extra) = &fluid.tag {
        tag.insert("Tag".to_owned(), Nbt::Compound(extra.clone()));
    }
    tag
}

/// Fires a fluid projectile from a cannon (the original's cannon constructor): it starts at the
/// cannon's centre stepped 0.7 along the facing, carries `fluid` at `power`, and is followed under
/// the id the spawn returns. None when the world cannot summon the kind there (the kind
/// unpublished, the chunk unloaded), and the caller fails the shot without spending fluid.
pub(crate) fn launch(
    world: &World,
    cannon: BlockPos,
    facing: Direction,
    fluid: FluidStack,
    power: f32,
    velocity: f32,
    inaccuracy: f32,
    stack: Stack,
) -> Option<u32> {
    let step = facing.offset();
    let position = [
        cannon[0] as f64 + 0.5 + 0.7 * step[0] as f64,
        cannon[1] as f64 + 0.5 + 0.7 * step[1] as f64,
        cannon[2] as f64 + 0.5 + 0.7 * step[2] as f64,
    ];
    let mut data = NbtCompound::new();
    data.insert("fluid".to_owned(), Nbt::Compound(fluid_nbt(&fluid)));
    data.insert("power".to_owned(), Nbt::Float(power));
    data.insert("knockback".to_owned(), Nbt::Float(KNOCKBACK));
    data.insert("water_inertia".to_owned(), Nbt::Float(WATER_INERTIA));
    data.insert("cannon".to_owned(), Nbt::IntArray(cannon.to_vec()));
    let spawn = ProjectileSpawn::new(Identifier::parse(FLUID_SPIT).expect("valid spit id"), position, shot_motion(step, velocity, inaccuracy, gaussians(world)))
        .data(data);
    let id = spawn_projectile(world, &spawn)?;
    shots().insert((world.dimension.clone(), id), Shot { fluid, power, cannon: Some(cannon), stack });
    Some(id)
}

/// Fires a fluid projectile from a creature (the original's spit from a tool's tank): it starts at
/// `eye` already moving at `motion`, carries `fluid` at `power` with the tool's own water inertia,
/// and is followed under the id the spawn returns, ownerless in the follow table (the spawn names
/// the owner, the table only carries what an impact needs). None when the world cannot summon the
/// kind there, and the caller fails the shot without spending fluid.
pub(crate) fn launch_shot(
    world: &World,
    eye: [f64; 3],
    motion: [f64; 3],
    fluid: FluidStack,
    power: f32,
    water_inertia: f32,
    owner: Option<u128>,
    stack: Stack,
) -> Option<u32> {
    let mut data = NbtCompound::new();
    data.insert("fluid".to_owned(), Nbt::Compound(fluid_nbt(&fluid)));
    data.insert("power".to_owned(), Nbt::Float(power));
    data.insert("knockback".to_owned(), Nbt::Float(KNOCKBACK));
    data.insert("water_inertia".to_owned(), Nbt::Float(water_inertia));
    let mut spawn = ProjectileSpawn::new(Identifier::parse(FLUID_SPIT).expect("valid spit id"), eye, motion).data(data);
    if let Some(owner) = owner {
        spawn = spawn.owner(owner);
    }
    let id = spawn_projectile(world, &spawn)?;
    shots().insert((world.dimension.clone(), id), Shot { fluid, power, cannon: None, stack });
    Some(id)
}

/// The effects data and the fluid tags an application runs against, as the other consumers read them.
fn data() -> Option<(Arc<RwLock<fluid_effects::Data>>, FluidTags)> {
    let effects = common::fluid_effects()?.clone();
    let tags = common::smeltery_data()?.read().ok()?.tags.clone();
    Some((effects, tags))
}

/// How a hit leaves the shot: it flies on with what fluid remains (the impact is cancelled), or the
/// flight ends here (the hit acts, and the game's own logic discards the projectile, as the original
/// discards it).
enum Flight {
    FliesOn(u32),
    Ends,
}

/// The flight's direction as the original reads it (its delta movement): the motion the impact
/// reports the projectile arrived with. A host that could not reach the projectile reports zero,
/// and the way it came — against the hit face — stands in.
fn flight_direction(hit: &ProjectileImpact) -> [f64; 3] {
    if hit.motion.iter().any(|axis| *axis != 0.0) { hit.motion } else { [-hit.face[0] as f64, -hit.face[1] as f64, -hit.face[2] as f64] }
}

/// The entity half of a hit (FluidEffectProjectile.onHitEntity, the knockback aside — see the module
/// note): with entity effects, they run against the target at the shot's power; what they spend is
/// gone, and the shot flies on with the rest, through the entity and on to what is behind it.
fn on_entity_hit(hit: &ProjectileImpact, shot: &Shot, world: &mut LiveWorld, effects: &fluid_effects::Data, tags: &FluidTags) -> Flight {
    let described = effects.describe(&shot.fluid, tags);
    if !described.map_or(false, |(_, has_entity, _)| has_entity) {
        return Flight::FliesOn(shot.fluid.amount);
    }
    let delta = flight_direction(hit);
    let mut target = LiveTarget::victim(hit, &hit.actions);
    let mut context = EntityContext {
        env: Env { world, holder: holder(shot.stack.clone(), Some((hit.projectile as i32, delta)), hit.at) },
        target: &mut target,
        location: hit.at,
    };
    let consumed = effects.apply_to_entity(&shot.fluid, shot.power, &mut context, tags).max(0) as u32;
    let remaining = shot.fluid.amount.saturating_sub(consumed);
    if remaining == 0 { Flight::Ends } else { Flight::FliesOn(remaining) }
}

/// The block half of a hit (FluidEffectProjectile.onHitBlock): the block effects run against the hit
/// block; with fluid left and the block gone to air the shot flies on, and with fluid left against a
/// block that stands it spends the rest on the neighbours, nearest its flight first, before the
/// flight ends.
fn on_block_hit(hit: &ProjectileImpact, shot: &Shot, world: &mut LiveWorld, effects: &fluid_effects::Data, tags: &FluidTags) -> Flight {
    let described = effects.describe(&shot.fluid, tags);
    if !described.map_or(false, |(has_block, _, _)| has_block) {
        return Flight::Ends;
    }
    let face = Direction::ALL.into_iter().find(|direction| direction.offset() == hit.face).unwrap_or(Direction::Up);
    // The flight's direction, which orders the neighbours below as the original's does.
    let delta = flight_direction(hit);
    let shot_holder = holder(shot.stack.clone(), Some((hit.projectile as i32, delta)), hit.at);
    let mut remaining = shot.fluid.amount;
    let mut context = block_context(world, shot_holder, hit.block, face, hit.at);
    let consumed = effects.apply_to_block(&shot.fluid, shot.power, &mut context, tags).max(0) as u32;
    remaining = remaining.saturating_sub(consumed);
    if remaining > 0 {
        let standing = context.env.world.block_state(hit.block).map_or(false, |state| state.block.to_string() != "minecraft:air");
        if !standing {
            return Flight::FliesOn(remaining);
        }
        for direction in order_by_nearest(delta) {
            if remaining == 0 { break; }
            let neighbour = direction.opposite().relative(hit.block);
            let current = FluidStack { amount: remaining, ..shot.fluid.clone() };
            let mut neighbour_context = context.at(neighbour, face);
            let consumed = effects.apply_to_block(&current, shot.power, &mut neighbour_context, tags).max(0) as u32;
            remaining = remaining.saturating_sub(consumed);
        }
    }
    Flight::Ends
}

/// The original's orderByNearest: every direction, largest dot with the flight first (the sort is by
/// the negated dot, as the original writes it).
fn order_by_nearest(delta: [f64; 3]) -> [Direction; 6] {
    let mut directions = Direction::ALL;
    directions.sort_by(|a, b| {
        let dot = |direction: &Direction| {
            let step = direction.offset();
            step[0] as f64 * delta[0] + step[1] as f64 * delta[1] + step[2] as f64 * delta[2]
        };
        dot(b).partial_cmp(&dot(a)).unwrap_or(std::cmp::Ordering::Equal)
    });
    directions
}

/// A projectile impact: only the shots this module fired are followed; every other projectile's hit
/// acts as it would without us.
fn on_impact(hit: &ProjectileImpact) -> bool {
    let key = (hit.dimension.clone(), hit.projectile);
    let Some(shot) = shots().get(&key).cloned() else { return true };
    let Some((effects, tags)) = data() else {
        shots().remove(&key);
        return true;
    };
    let effects = effects.read().expect("fluid effects lock poisoned");
    let mut world = LiveWorld::new(hit.world());
    let flight = if hit.entity.is_some() {
        on_entity_hit(hit, &shot, &mut world, &effects, &tags)
    } else {
        on_block_hit(hit, &shot, &mut world, &effects, &tags)
    };
    match flight {
        Flight::FliesOn(remaining) => {
            if let Some(tracked) = shots().get_mut(&key) {
                tracked.fluid.amount = remaining;
            }
            false
        }
        Flight::Ends => {
            shots().remove(&key);
            true
        }
    }
}

/// Whether a shot at height `y` in `dimension` flies above the world (FluidEffectProjectile's
/// end-of-tick discard, `getY() > level.getMaxBuildHeight() + 64`): past 64 blocks above the
/// build limit the shot is discarded, as it can never come back into the world (in practice
/// only a lighter-than-air fluid's rising shot gets there). The range is read from the host's
/// publication, as the fireball's flight end reads it (the flight telling runs in no world
/// callback); when the host has not published the range, the flight is not ended on height.
fn above_max(dimension: &str, y: f64) -> bool {
    match World::new(dimension).build_height() {
        Some([_, max]) => y > f64::from(max) + 64.0,
        None => false,
    }
}

/// A projectile in flight: only the shots this module fired are followed. One that another
/// consumer already ended is simply forgotten; one above the build range's max is discarded
/// here (the original's above-max discard), and its record goes with it.
fn on_flight(flight: &ProjectileFlight) {
    if flight.kind != FLUID_SPIT {
        return;
    }
    let key = (flight.dimension.clone(), flight.projectile);
    if flight.discard.get() {
        shots().remove(&key);
        return;
    }
    if above_max(&flight.dimension, flight.position[1]) {
        flight.discard.set(true);
        shots().remove(&key);
    }
}

pub(crate) fn register() {
    // The type itself: a llama spit in movement, at the original's size and tracking, with no
    // velocity updates (FluidEffectProjectile's registration in TinkerModifiers). A duplicate or a
    // registration after collection keeps the earlier one, as the registry answers.
    let _ = EntityTypeRegistry::register_derived(
        Identifier::parse(FLUID_SPIT).expect("valid spit id"),
        Identifier::parse("minecraft:llama_spit").expect("valid spit factory"),
        0.25,
        0.25,
        4,
        false,
    );
    ProjectileEvents::IMPACT.register(on_impact);
    ProjectileEvents::FLIGHT.register(on_flight);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluid_effects::Effects;
    use pluto_lib::{with_world, MemoryWorldOps};

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    /// A data set with one entry: blazing blood lights entities on fire, molten obsidian sets blocks.
    fn effects_data() -> fluid_effects::Data {
        let files = vec![
            (id("tconstruct:blazing_blood"), serde_json::json!({
                "fluid": {"fluid": "tconstruct:blazing_blood", "amount": 250},
                "entity_effects": [{"type": "tconstruct:fire", "action": "add", "time": 2}],
            })),
            (id("tconstruct:molten_obsidian"), serde_json::json!({
                "fluid": {"fluid": "tconstruct:molten_obsidian", "amount": 50},
                "block_effects": [{"type": "tconstruct:set_block", "block": "minecraft:obsidian"}],
            })),
        ];
        let context = crate::conditions::Context { items: None, config: Default::default() };
        fluid_effects::Data { effects: Effects::load(&files, &context), loaded: true }
    }

    #[test]
    fn the_shot_motion_is_the_facing_with_the_rise_normalized_and_scaled() {
        let motion = shot_motion([0, 0, -1], 1.1, 4.5, [0.0, 0.0, 0.0]);
        let length = (0.1f64 * 0.1 + 1.0).sqrt();
        // the velocity is a float, as in the original: the expectations widen that same float
        let velocity = f64::from(1.1f32);
        assert!((motion[2] - (-1.0 / length * velocity)).abs() < 1e-9);
        assert!((motion[1] - (0.1 / length * velocity)).abs() < 1e-9);
        assert_eq!(motion[0], 0.0);
        let jittered = shot_motion([0, 0, -1], 1.1, 4.5, [1.0, 0.0, 0.0]);
        assert!((jittered[0] - 0.0075 * 4.5 * velocity).abs() < 1e-9, "one gaussian of jitter on the x axis");
    }

    #[test]
    fn directions_order_nearest_the_flight_first() {
        let ordered = order_by_nearest([0.0, 0.0, -1.0]);
        assert_eq!(ordered[0], Direction::North, "the way it flies is nearest");
        assert_eq!(ordered[5], Direction::South);
    }

    #[test]
    fn a_launch_records_the_shot_and_an_entity_hit_burns_it_down_to_the_end() {
        let world = World::new("minecraft:overworld");
        let mut ops = MemoryWorldOps::default();
        let fluid = FluidStack { fluid: id("tconstruct:blazing_blood"), amount: 250, tag: None };
        let spawned = with_world(&mut ops, || launch(&world, [0, 64, 0], Direction::North, fluid, 1.0, 1.1, 4.5, Stack::empty()));
        assert_eq!(spawned, Some(1));
        assert_eq!(ops.summons.len(), 1);
        assert_eq!(ops.summons[0].kind, FLUID_SPIT);
        let data = effects_data();
        let tags = FluidTags::default();
        let mut live = LiveWorld::new(world.clone());
        // one level (250) buys 2 seconds of fire on an unburning zombie: the whole shot is spent
        let hit = ProjectileImpact {
            dimension: "minecraft:overworld".to_owned(), projectile: 1, kind: FLUID_SPIT.to_owned(), owner: -1,
            entity: Some(9), entity_kind: "minecraft:zombie".to_owned(), entity_effects: Vec::new(), block: [0, 64, -1], face: [0, 0, 1], at: [0.5, 64.5, -0.5],
            motion: [0.0; 3], data: NbtCompound::new(), piercing_ignored: Vec::new(), discard: std::cell::Cell::new(false), motion_after: std::cell::RefCell::new(None),
            ignore_entities: std::cell::RefCell::new(Vec::new()), actions: std::cell::RefCell::new(Vec::new()),
        };
        let shot = shots().get(&("minecraft:overworld".to_owned(), 1)).cloned().unwrap();
        assert!(matches!(on_entity_hit(&hit, &shot, &mut live, &data, &tags), Flight::Ends), "the fire spends the whole level");
        let queued = hit.actions.borrow();
        assert_eq!(queued.len(), 1);
        assert!(matches!(queued[0], pluto_lib::EntityAction::SetFire { seconds: 2, .. }));
    }

    #[test]
    fn a_block_hit_spends_on_the_block_and_ends_unless_the_block_is_gone() {
        let world = World::new("minecraft:overworld");
        let mut ops = MemoryWorldOps::default();
        ops.blocks.insert(("minecraft:overworld".to_owned(), [0, 64, -1]), "minecraft:stone".to_owned());
        let data = effects_data();
        let tags = FluidTags::default();
        let shot = Shot { fluid: FluidStack { fluid: id("tconstruct:molten_obsidian"), amount: 50, tag: None }, power: 1.0, cannon: Some([0, 64, 0]), stack: Stack::empty() };
        let hit = ProjectileImpact {
            dimension: "minecraft:overworld".to_owned(), projectile: 3, kind: FLUID_SPIT.to_owned(), owner: -1,
            entity: None, entity_kind: String::new(), entity_effects: Vec::new(), block: [0, 64, -1], face: [0, 0, 1], at: [0.5, 64.5, -0.5],
            motion: [0.0; 3], data: NbtCompound::new(), piercing_ignored: Vec::new(), discard: std::cell::Cell::new(false), motion_after: std::cell::RefCell::new(None),
            ignore_entities: std::cell::RefCell::new(Vec::new()), actions: std::cell::RefCell::new(Vec::new()),
        };
        let mut live = LiveWorld::new(world);
        let flight = with_world(&mut ops, || on_block_hit(&hit, &shot, &mut live, &data, &tags));
        assert!(matches!(flight, Flight::Ends), "the block stands afterwards (it became obsidian), so the flight ends");
        assert_eq!(ops.blocks[&("minecraft:overworld".to_owned(), [0, 64, -1])], "minecraft:obsidian");
    }

    #[test]
    fn a_block_hits_neighbours_follow_the_flights_direction() {
        // the shot arrives travelling mostly east against a south face: the leftover is spent on
        // the neighbours in the flight's order — east's far side first, then north's — not in the
        // order the face alone would give (north's far side, then above)
        let world = World::new("minecraft:overworld");
        let mut ops = MemoryWorldOps::default();
        for pos in [[0, 64, -1], [-1, 64, -1], [1, 64, -1], [0, 64, -2], [0, 64, 0], [0, 63, -1], [0, 65, -1]] {
            ops.blocks.insert(("minecraft:overworld".to_owned(), pos), "minecraft:stone".to_owned());
        }
        let data = effects_data();
        let tags = FluidTags::default();
        let shot = Shot { fluid: FluidStack { fluid: id("tconstruct:molten_obsidian"), amount: 150, tag: None }, power: 1.0, cannon: Some([0, 64, 0]), stack: Stack::empty() };
        let hit = ProjectileImpact {
            dimension: "minecraft:overworld".to_owned(), projectile: 3, kind: FLUID_SPIT.to_owned(), owner: -1,
            entity: None, entity_kind: String::new(), entity_effects: Vec::new(), block: [0, 64, -1], face: [0, 0, 1], at: [0.5, 64.5, -0.5],
            motion: [1.0, 0.0, -0.2], data: NbtCompound::new(), piercing_ignored: Vec::new(), discard: std::cell::Cell::new(false), motion_after: std::cell::RefCell::new(None),
            ignore_entities: std::cell::RefCell::new(Vec::new()), actions: std::cell::RefCell::new(Vec::new()),
        };
        let mut live = LiveWorld::new(world);
        let flight = with_world(&mut ops, || on_block_hit(&hit, &shot, &mut live, &data, &tags));
        assert!(matches!(flight, Flight::Ends));
        let at = |pos: [i32; 3]| ops.blocks.get(&("minecraft:overworld".to_owned(), pos)).map(String::as_str);
        assert_eq!(at([0, 64, -1]), Some("minecraft:obsidian"), "the hit block takes the first level");
        assert_eq!(at([-1, 64, -1]), Some("minecraft:obsidian"), "east's far side is nearest the flight");
        assert_eq!(at([0, 64, 0]), Some("minecraft:obsidian"), "north's far side is next, and the shot is spent");
        assert_eq!(at([0, 64, -2]), Some("minecraft:stone"), "the face's order would have spent here instead");
        assert_eq!(at([0, 65, -1]), Some("minecraft:stone"), "three levels were all the shot carried");
    }

    /// A flight telling for the tracked shot at height `y`, in a dimension no other test uses.
    fn flight_at(y: f64) -> ProjectileFlight {
        ProjectileFlight {
            dimension: "example:flight".to_owned(), projectile: 4242, kind: FLUID_SPIT.to_owned(), owner: -1,
            owner_position: None, age: 1, position: [0.0, y, 0.0], motion: [0.0, 1.0, 0.0], in_ground: false, no_gravity: false,
            arrow: None, data: NbtCompound::new(), motion_after: std::cell::RefCell::new(None), discard: std::cell::Cell::new(false),
            no_gravity_after: std::cell::Cell::new(None), pickup_after: std::cell::Cell::new(None),
            release_from_ground: std::cell::Cell::new(false), actions: std::cell::RefCell::new(Vec::new()),
        }
    }

    #[test]
    fn a_shot_above_the_build_range_is_discarded_in_flight() {
        let mut ops = MemoryWorldOps::default();
        ops.build_heights.insert("example:flight".into(), [-64, 320]);
        let key = ("example:flight".to_owned(), 4242);
        let shot = Shot { fluid: FluidStack { fluid: id("tconstruct:molten_iron"), amount: 50, tag: None }, power: 1.0, cannon: None, stack: Stack::empty() };
        with_world(&mut ops, || {
            shots().insert(key.clone(), shot.clone());
            // Up to 64 above the max the flight stands and the record stays.
            let low = flight_at(384.0);
            on_flight(&low);
            assert!(!low.discard.get(), "at the max + 64 the shot still flies");
            assert!(shots().contains_key(&key), "and its record stays");
            // Above that the shot is discarded and its record goes with it.
            let high = flight_at(385.0);
            on_flight(&high);
            assert!(high.discard.get(), "above the max + 64 the shot is discarded");
            assert!(!shots().contains_key(&key), "the record goes with the discarded shot");
            // A flight another consumer already ended is only forgotten, not re-decided.
            shots().insert(key.clone(), shot.clone());
            let ended = flight_at(100.0);
            ended.discard.set(true);
            on_flight(&ended);
            assert!(!shots().contains_key(&key), "an ended flight's record is forgotten");
            // Another kind of projectile is not this module's business.
            shots().insert(key.clone(), shot.clone());
            let mut other = flight_at(400.0);
            other.kind = "minecraft:arrow".into();
            on_flight(&other);
            assert!(!other.discard.get());
            assert!(shots().contains_key(&key));
            shots().remove(&key);
        });
    }
}
