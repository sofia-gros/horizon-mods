//! Projectiles hitting things (Forge: `ProjectileImpactEvent`). A callback sees what flew, who threw it and what it met, can act on the
//! world with [`World`] and can cancel the hit.
//!
//! A callback also hears a projectile when it is spawned ([`ProjectileEvents::SPAWN`], before its first tick of flight, with the
//! spawn-time facts a shot is made of — an arrow's pierce, a flight without gravity, a clamped direction, a pickup status
//! for the arrow) and on every tick it
//! flies ([`ProjectileEvents::FLIGHT`], at the end of the tick, with the motion it will fly the next tick with). The game's own
//! entity-tick telling covers mobs, item entities, minecarts and boats, not projectiles (pinned `native_entity_tick_pre`,
//! API 146), so PlutoLib reaches into the game's projectile tick itself to tell these two; flight behaviours — a fuse that
//! burns down, gravity that returns after a hover, a stuck tool that pulls items toward it and flies back to its owner — are
//! built from them, on top of what acts at the two ends: the impact reports the motion and the saved data the projectile
//! arrived with and lets a callback discard it, send it flying on with a new motion, or tell the game which entities a
//! piercing arrow has passed through, which is what pierce, bounce and an explosion where it lands are made of.

use std::cell::{Cell, RefCell};

use super::base::{Event, NbtCompound};
use super::block_entity_api_v1::{BlockPos, ItemSlot};
use super::entity_events_v1::{CarriedEdit, CarriedStack, EntityAction, StackSlot};
use super::player_api::StackData;
use super::world_api_v1::{ExplosionInteraction, World};

/// A projectile that has found what it meets.
#[derive(Clone, Debug)]
pub struct ProjectileImpact {
    /// The dimension, such as `minecraft:overworld`; also available as [`ProjectileImpact::world`].
    pub dimension: String,
    /// The projectile's entity id.
    pub projectile: u32,
    /// The projectile's type, such as `minecraft:snowball` or a mod's thrown type.
    pub kind: String,
    /// The entity that threw it (negative if none, such as a dispenser).
    pub owner: i64,
    /// The entity it hit, if it hit one.
    pub entity: Option<u32>,
    /// The type of the entity it hit; empty if it hit a block.
    pub entity_kind: String,
    /// The mob effects the entity it hit carries, by id (such as `tconstruct:enderference`); empty
    /// if it hit a block, or when the host could not reach the target. Read off the live target, as
    /// the original reads the target's effects to decide whether a projectile may hit it at all.
    pub entity_effects: Vec<String>,
    /// The block it hit.
    pub block: BlockPos,
    /// The face it hit as a direction vector, `[0, 1, 0]` for the top.
    pub face: [i32; 3],
    /// Where it hit.
    pub at: [f64; 3],
    /// How fast the projectile was moving when it hit, in blocks a tick (`Entity.getDeltaMovement`, the
    /// game's own report of its motion); zero when the host could not reach the projectile.
    pub motion: [f64; 3],
    /// The projectile's saved data (the entity's save, as the host reports it): what its spawn data and
    /// the stack that fired it left on it — a supplied arrow's stack data rides here, as a spawn's does
    /// in `ProjectileSpawn`. Empty when the host could not reach the projectile. A launcher that wants
    /// its modifiers known when the shot lands puts them in the fired stack's data (see
    /// [`ProjectileEvents::AMMO`]); this is where they are read back.
    pub data: NbtCompound,
    /// The entities a piercing arrow has already gone through, by id, as the game reports them when the
    /// hit lands — the game's own ignore list (`piercingIgnoreEntityIds` in the original, the arrow's
    /// `pierced` here). For a piercing arrow the entity this hit is on is already in it, the game adds
    /// it before it tells the hit, so the list's length less one is how many it had passed through
    /// before. Empty for a projectile the game reports none for (a non-piercing arrow, a thrown type),
    /// or when the host could not reach the projectile. A callback that re-deals a piercing hit reads
    /// this to know whether the arrow has spent its pierce, and writes the victim to
    /// [`ProjectileImpact::ignore_entities`] so the game skips it next time.
    pub piercing_ignored: Vec<u32>,
    /// The entities a piercing arrow has pierced and killed so far, by id, as the game reports them when the
    /// hit lands (`piercedAndKilledEntities` in the original): the list the crossbow-kill criterion is fed
    /// from. The list lives on the arrow in the original; the game keeps no such list, so PlutoLib keeps it
    /// itself, one per projectile, from the kills its impacts settle — a piercing arrow whose hit kills the
    /// entity it was told on joins that entity here from the next hit on. Empty for a projectile that has
    /// killed nothing it pierced, a non-piercing arrow, or when the host could not reach the projectile.
    pub pierced_and_killed: Vec<u32>,
    /// The punch (knockback) level the shot was fired with, as the game reports it when the hit is told:
    /// the shooter's bow bonus, recorded on the projectile at the shot — what a shooter holding an
    /// enchanted bow contributes to its arrow even when no weapon is saved on the arrow itself, as for a
    /// skeleton's shot, whose save names no weapon. Zero when the game reports none (a shot with no bow
    /// bonus, a projectile that is no arrow, or one the host could not reach). A callback re-dealing an
    /// arrow's hit reads the punch off the saved weapon when the save names one and falls back to this.
    pub shot_punch: i32,
    /// Set to true to discard the projectile once the callbacks ran: the game removes it as the hook
    /// returns, before its own hit handling continues, so cancel the hit as well when the hit itself
    /// should not land. How a projectile that explodes where it lands consumes itself.
    pub discard: Cell<bool>,
    /// The motion the projectile flies on with, which a callback may set: applied as the hook returns
    /// when the projectile is not discarded. With a cancelled hit this is a bounce — reflect
    /// [`ProjectileImpact::motion`] yourself; the game does not reflect it for you.
    pub motion_after: RefCell<Option<[f64; 3]>>,
    /// The entities this hit has passed through, by id, for the game to add to the projectile's ignore
    /// list: applied as the tick settles, before the projectile flies its next tick, so a piercing
    /// arrow does not meet the same entity again. A callback that re-deals a piercing hit writes the
    /// entity it hit here — the game resets the list to what it was before the tick when a hit is
    /// cancelled, so a victim recorded here is what keeps it from being met again. Only the entities
    /// a callback writes are added; the rest of the list is left alone.
    pub ignore_entities: RefCell<Vec<u32>>,
    /// The actions to do to the parties of the hit (`Victim` the entity it hit, `Attacker` the entity
    /// that threw it), which callbacks add to. They are done after the callbacks answer, whether the
    /// hit is cancelled or not, and only for a party the hit has: on a block hit there is no victim,
    /// so `Victim` actions go nowhere.
    pub actions: RefCell<Vec<EntityAction>>,
}

impl ProjectileImpact {
    /// The world the projectile is in.
    pub fn world(&self) -> World {
        World::new(&self.dimension)
    }
}

/// A projectile that entered a server level, heard after it exists. Read-only: the projectile is already flying.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectileLaunch {
    /// The launching player's uuid; None for a projectile no player launched (a dispenser's, a mob's).
    pub player: Option<u128>,
    /// The launching player's name; empty when `player` is None.
    pub name: String,
    /// The projectile's entity id.
    pub projectile: u32,
    /// The projectile's type, such as `minecraft:arrow` or `minecraft:snowball`.
    pub kind: String,
    /// How fast the projectile is moving as it enters, in blocks a tick; zero when the host could not
    /// reach the projectile.
    pub motion: [f64; 3],
    /// The projectile's saved data, as in [`ProjectileImpact::data`]; empty when the host could not
    /// reach the projectile.
    pub data: NbtCompound,
}

/// A player's bow or crossbow shot an arrow, heard after the arrow entered the level.
#[derive(Clone, Debug)]
pub struct BowShot {
    /// The shooting player's uuid and name.
    pub player: u128,
    pub name: String,
    /// The arrow's entity id.
    pub projectile: u32,
    /// The arrow's launch speed as the game reports it: a fully drawn bow gives 3.0, a weaker draw less, and a crossbow its fixed power.
    pub force: f32,
    /// The stacks in the player's main hand and off hand as the arrow left (damage, max damage and custom data); the bow is one of them.
    /// None for an empty hand or a player the game could not reach.
    pub main_hand: Option<StackData>,
    pub off_hand: Option<StackData>,
    /// The actions to do to the player (`Attacker`), which callbacks add to.
    pub actions: RefCell<Vec<EntityAction>>,
}

/// A living entity is about to shoot `weapon` and the game asks what projectile it fires (NeoForge:
/// `LivingGetProjectileEvent`): heard before the shot, while the ammunition is still where it lies. The game has its own answer
/// (`vanilla`); a callback that leaves `supplied` alone changes nothing and the game takes `vanilla` from where it found it,
/// shrinking that stack itself. A callback that sets `supplied` takes over: the game fires that stack and shrinks nothing, so
/// the callback consumes its source itself — take the arrow out of the quiver stack it came from and queue the shrunken quiver
/// in `edits`. Heard for players and for mobs that shoot (a skeleton's arrows); for a mob, `carried` is empty and `player` None.
#[derive(Clone, Debug)]
pub struct AmmoRequest {
    /// The shooting player's uuid; None when a mob shoots.
    pub player: Option<u128>,
    /// The shooting player's name; empty when `player` is None.
    pub name: String,
    /// The shooting mob's entity id; None when a player shoots.
    pub entity: Option<u32>,
    /// The shooter's type, such as `minecraft:player` or `minecraft:skeleton`.
    pub kind: String,
    /// The weapon being shot (a bow, a crossbow), with its damage, max damage, custom data and enchantments.
    pub weapon: StackData,
    /// The projectile stack the game picked itself (a plain arrow for a creative player with none); None when it found none.
    pub vanilla: Option<ItemSlot>,
    /// Every stack the shooter carries, each with its place, when the shooter is a player the host could read: the hands, the
    /// armor and the main inventory's non-empty slots. A quiver's arrows are inside its stack's custom data, not listed apart.
    pub carried: Vec<CarriedStack>,
    /// The projectile stack to fire instead of `vanilla`, which a callback may set; the last one set wins. A count of 0 means no
    /// projectile at all. Left alone, the game fires `vanilla`.
    pub supplied: RefCell<Option<ItemSlot>>,
    /// Whole new stacks for places the shooter carries, which callbacks add to: how a callback consumes the source of a
    /// `supplied` stack (the quiver it came from, with one arrow less). The host writes them after the callbacks ran.
    pub edits: RefCell<Vec<CarriedEdit>>,
}

/// What a flying projectile's arrow half keeps, when it has one: the facts a shot is set up with and a stuck arrow rests with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowFacts {
    /// How many entities the arrow passes through (`AbstractArrow.getPierceLevel`).
    pub pierce: u8,
    /// Whether the arrow is stuck in a block (`AbstractArrow.inGround`).
    pub in_ground: bool,
    /// How many ticks the arrow has been stuck (`AbstractArrow.inGroundTime`).
    pub in_ground_time: u32,
    /// Whether it has dealt its damage (`AbstractArrow.dealtDamage` on a trident body): it hits no more, and a loyal
    /// flight comes home.
    pub dealt_damage: bool,
}

/// An arrow's pickup status (`AbstractArrow.Pickup`): who may pick a stuck arrow up. The discriminants are the
/// host's saved byte (`Pickup.byte`): 0 disallowed, 1 allowed, 2 creative-only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArrowPickup {
    /// No player may pick the arrow up.
    #[default]
    Disallowed = 0,
    /// Any player may pick the arrow up and is handed its pickup stack.
    Allowed = 1,
    /// Only a player in creative may take the arrow, and takes no item for it: an arrow its shooter did not
    /// spend, such as one a launcher's modifier supplied.
    CreativeOnly = 2,
}

/// A projectile at the moment it joins the flying list, before its first tick: a callback may still set the shot up —
/// give an arrow its pierce level, lift gravity off a flight, clamp the direction it leaves in, set an arrow's pickup
/// status, or name the stack a stuck arrow hands back when picked up. Heard for every projectile the game adds on a
/// server: a bow's arrow, a thrown tool a callback itself summoned a moment earlier, a mob's shot. The game's own launch
/// telling ([`ProjectileEvents::LAUNCH`]) follows read-only; this one is the set-up pass before it.
#[derive(Clone, Debug)]
pub struct ProjectileSpawned {
    /// The dimension, such as `minecraft:overworld`.
    pub dimension: String,
    /// The projectile's entity id: 0 at this pass when the game has not numbered it yet (it is numbered as it joins the
    /// list, after the pass). Bind per-projectile state at [`ProjectileEvents::LAUNCH`] or on the first
    /// [`ProjectileEvents::FLIGHT`], which carry the number.
    pub projectile: u32,
    /// The projectile's type, such as `minecraft:arrow` or a mod's thrown type.
    pub kind: String,
    /// The entity that threw or shot it (negative if none, such as a dispenser).
    pub owner: i64,
    /// Where it enters.
    pub position: [f64; 3],
    /// How fast it is moving as it enters, in blocks a tick.
    pub motion: [f64; 3],
    /// The arrow facts, when the projectile is an arrow.
    pub arrow: Option<ArrowFacts>,
    /// The projectile's own stack as data (an arrow's pickup stack, a thrown type's item): how a callback knows what was
    /// fired. None when the projectile carries no stack the game reports.
    pub stack: Option<StackData>,
    /// The projectile's saved data, as in [`ProjectileImpact::data`].
    pub data: NbtCompound,
    /// The motion it flies its first tick with, which a callback may replace (a direction clamped to fixed angles, a
    /// speed set by the stack that fired it).
    pub motion_after: RefCell<Option<[f64; 3]>>,
    /// The pierce level to give an arrow, when a callback sets one.
    pub pierce: Cell<Option<u8>>,
    /// The pickup status to give an arrow, when a callback sets one (a supplied arrow is creative-only, so a
    /// player who did not spend an arrow cannot pick it up again). Only an arrow reads it; on any other
    /// projectile it does nothing.
    pub pickup: Cell<Option<ArrowPickup>>,
    /// Whether the flight is without gravity, when a callback sets it (a hover that a later flight tick ends).
    pub no_gravity: Cell<Option<bool>>,
    /// The stack a stuck arrow hands back when a player picks it up, when a callback sets it (the tool a throw carries).
    pub pickup_stack: RefCell<Option<StackData>>,
}

/// One thing a flight callback asks the game to do around a flying projectile, in plain data: the flight telling runs
/// outside the game's own hook scope, so world and entity work is queued here and done by the game as the tick settles,
/// in the order queued.
#[derive(Clone, Debug, PartialEq)]
pub enum FlightAction {
    /// Set off an explosion where the projectile is (the game's own blast, as [`World::explode`] asks for).
    Explode { center: [f64; 3], radius: f32, fire: bool, interaction: ExplosionInteraction },
    /// Spawn a particle (a fuse's sparks, a trail).
    Particle { name: String, position: [f64; 3], count: u32, spread: [f64; 3], speed: f64 },
    /// Add a motion to an entity (a pull toward the projectile, a push away from it).
    PushEntity { entity: u32, delta: [f64; 3] },
    /// Pull the item entities within `radius` of `center` toward it, each by up to `strength` blocks a tick (a stuck
    /// tool's magnet). The game does the pulling; how far each item moves is its own physics.
    PullItems { center: [f64; 3], radius: f64, strength: f64 },
    /// Pull the mobs within `radius` of `center` toward it (an attraction where a projectile lands or its fuse ends):
    /// the game pulls each mob by `strength`, weakened by the square root of its distance, and pulls at most 25.
    PullEntities { center: [f64; 3], radius: f64, strength: f64 },
    /// Drop a stack as an item entity where the projectile is (the tool a throw leaves behind when it cannot be picked up).
    DropStack { stack: StackData, position: [f64; 3] },
    /// Hand a stack to the projectile's owner: into a player owner's inventory, or dropped at their feet when it does not
    /// fit or the owner is no player (a returning tool's loyalty, when the flight itself brought it home).
    GiveStack { stack: StackData },
}

/// A projectile at the end of a tick it flew (or rested stuck) in: a callback may steer the next tick — set the motion it
/// flies on with, lift or restore its gravity, set an arrow's pickup status, free a stuck arrow so it can fly home,
/// discard it — and queue
/// [`FlightAction`]s around it. A fuse is a callback counting `age`; gravity that returns is a callback answering the age
/// it was set for; a magnet is a callback queueing [`FlightAction::PullItems`] while its tool rests stuck.
#[derive(Clone, Debug)]
pub struct ProjectileFlight {
    /// The dimension, such as `minecraft:overworld`.
    pub dimension: String,
    /// The projectile's entity id.
    pub projectile: u32,
    /// The projectile's type, such as `minecraft:arrow` or a mod's thrown type.
    pub kind: String,
    /// The entity that threw or shot it (negative if none).
    pub owner: i64,
    /// Where the owner stands, when the game could reach it: how a callback steers a flight home.
    pub owner_position: Option<[f64; 3]>,
    /// How many ticks the projectile has flown (`Entity.tickCount` on the projectile, its own age).
    pub age: u32,
    /// Where it ended the tick.
    pub position: [f64; 3],
    /// How fast it is moving, in blocks a tick: the motion it will fly the next tick with, before drag and gravity.
    pub motion: [f64; 3],
    /// Whether it rests stuck in a block (an arrow in the ground, a thrown item that landed).
    pub in_ground: bool,
    /// Whether its flight is without gravity.
    pub no_gravity: bool,
    /// The arrow facts, when the projectile is an arrow.
    pub arrow: Option<ArrowFacts>,
    /// The projectile's saved data, as in [`ProjectileImpact::data`].
    pub data: NbtCompound,
    /// The motion it flies the next tick with, which a callback may replace (steering home, a pull, a push).
    pub motion_after: RefCell<Option<[f64; 3]>>,
    /// Set to true to discard the projectile as the tick settles (a fuse that went off, a tool that arrived home).
    pub discard: Cell<bool>,
    /// Whether its flight is without gravity from the next tick, when a callback sets it (gravity returning to a hover).
    pub no_gravity_after: Cell<Option<bool>>,
    /// The pickup status to give an arrow from the next tick, when a callback sets it (a launcher's modifier marking
    /// the fired arrow once the shot's profile is bound; a stuck arrow reads it no later than the tick it lands).
    /// Only an arrow reads it; on any other projectile it does nothing.
    pub pickup_after: Cell<Option<ArrowPickup>>,
    /// Set to true to free a stuck arrow so the motion it is given can fly it (a loyal tool leaving the ground).
    /// Only an arrow can be freed; on any other projectile it does nothing.
    pub release_from_ground: Cell<bool>,
    /// The [`FlightAction`]s to do around the projectile as the tick settles, which callbacks add to.
    pub actions: RefCell<Vec<FlightAction>>,
}

static LAUNCH: Event<ProjectileLaunch, ()> = Event::void();
static SHOOT_BOW: Event<BowShot, ()> = Event::void();
static IMPACT: Event<ProjectileImpact, bool> = Event::allow_all();
static AMMO: Event<AmmoRequest, ()> = Event::void();
static SPAWN: Event<ProjectileSpawned, ()> = Event::void();
static FLIGHT: Event<ProjectileFlight, ()> = Event::void();

/// Events about projectiles.
pub struct ProjectileEvents;

impl ProjectileEvents {
    /// A projectile hit something, before the hit acts. Answer `false` to cancel the hit (the projectile flies on); one `false` is enough.
    /// The entities around the hit are in reach while callbacks run, and the actions callbacks queue on the hit are done to its parties after.
    /// Only projectiles the game flies (arrows, tridents, snowballs and mod thrown types, eggs, pearls, potions, fireballs, ...) are heard.
    pub const IMPACT: &'static Event<ProjectileImpact, bool> = &IMPACT;
    /// A projectile entered a server level (Bukkit: `ProjectileLaunchEvent`; Fabric has no counterpart). Heard for every projectile the
    /// game adds, a bow's arrow included. Server only.
    pub const LAUNCH: &'static Event<ProjectileLaunch, ()> = &LAUNCH;
    /// A player's bow or crossbow shot an arrow, after the arrow entered the level, once for each arrow (a multishot crossbow reports each
    /// of its three). Queue `actions` for the player. Server only.
    pub const SHOOT_BOW: &'static Event<BowShot, ()> = &SHOOT_BOW;
    /// A living entity is about to shoot and the game asks what projectile it fires (see [`AmmoRequest`]): the surface a quiver
    /// modifier answers from. A player's request also notes the weapon, so the damage event of the shot can name the hand that
    /// launched. Server only.
    pub const AMMO: &'static Event<AmmoRequest, ()> = &AMMO;
    /// A projectile joined the flying list, before its first tick: the set-up pass (see [`ProjectileSpawned`]). Server only.
    pub const SPAWN: &'static Event<ProjectileSpawned, ()> = &SPAWN;
    /// A projectile ended a tick of flight: the steering pass (see [`ProjectileFlight`]). Heard once a tick for every
    /// projectile the game flies on a server, stuck arrows included, for as long as it stays in the flying list. Server only.
    pub const FLIGHT: &'static Event<ProjectileFlight, ()> = &FLIGHT;
}

/// Whether anyone asks for ammunition, so the host builds the carried stacks only when someone does.
pub(crate) fn ammo_heard() -> bool {
    !AMMO.is_empty()
}

/// The ammunition the callbacks settled on: what they set in `supplied` (None when they left it alone, so the host keeps the
/// game's own pick and its own consumption; a set stack of count 0 means no projectile) and the edits to the shooter's
/// carried stacks. A player's request also notes the weapon being shot (see [`PLAYER_WEAPON`]).
pub(crate) fn ammo(request: &AmmoRequest) -> (Option<ItemSlot>, Vec<CarriedEdit>) {
    if request.player.is_some() {
        PLAYER_WEAPON.with(|weapon| *weapon.borrow_mut() = Some(request.weapon.clone()));
    }
    AMMO.invoke(request);
    (request.supplied.borrow_mut().take(), request.edits.borrow_mut().drain(..).collect())
}

thread_local! {
    /// The weapon a player's last ammunition request was about (the bow or crossbow being shot), as the game asked
    /// about it: the damage event of the shot's hit matches it against the shooter's hands as the hit lands, which
    /// names the hand that launched. A mob's request leaves it alone (a mob's hit never reads it); a later shot
    /// overwrites it. Read, never taken, so one piercing arrow names its launcher on every hit it lands.
    static PLAYER_WEAPON: RefCell<Option<StackData>> = const { RefCell::new(None) };
}

/// The hand that launched the hit, and the launcher's stack as the hit landed, when the shooter's last weapon is
/// still held in one of their hands: the main hand when both hold it (the game tries the main hand first). None
/// when no weapon was noted, or when neither hand holds it any more (the shooter swapped it away after the shot).
pub(crate) fn launcher_for(main_hand: &Option<StackData>, off_hand: &Option<StackData>) -> Option<(StackSlot, StackData)> {
    let weapon = PLAYER_WEAPON.with(|weapon| weapon.borrow().clone())?;
    let is_weapon = |stack: &StackData| stack.item == weapon.item && stack.custom_data == weapon.custom_data;
    if let Some(stack) = main_hand.as_ref().filter(|stack| is_weapon(stack)) {
        Some((StackSlot::MainHand, stack.clone()))
    } else if let Some(stack) = off_hand.as_ref().filter(|stack| is_weapon(stack)) {
        Some((StackSlot::OffHand, stack.clone()))
    } else {
        None
    }
}

/// Whether anyone listens, so the game only keeps the copies it needs when someone does.
pub(crate) fn heard() -> bool {
    !IMPACT.is_empty()
}

pub(crate) fn impact(impact: &ProjectileImpact) -> bool {
    IMPACT.invoke(impact)
}

/// Whether anyone listens to launches or shots, so the game builds the payloads only when someone does.
pub(crate) fn launch_heard() -> bool {
    !LAUNCH.is_empty()
}

pub(crate) fn shot_heard() -> bool {
    !SHOOT_BOW.is_empty()
}

pub(crate) fn launched(launch: &ProjectileLaunch) {
    LAUNCH.invoke(launch);
}

/// Remembers the stack `projectile` (an entity id a spawn handed back) was launched from, for
/// the experience drop of a mob it goes on to kill: the drop reads the projectile's recorded
/// experience float from this stack instead of the credited player's held tool. The game names
/// the launcher's stack itself for the launches it makes (a shot from a bow, a dispenser's
/// shot, a mob's shot); a launch a mod makes with a stack the game never held at the launch —
/// a tool thrown from the hand, whose hand is already empty when the launch is reported —
/// names it here, with the stack as launched (a thrown tool: the one stack thrown).
pub fn note_launch_stack(projectile: u32, stack: StackData) {
    super::entity_events_v1::note_projectile_tool(projectile, stack);
}

/// The actions the callbacks queued for the player who shot.
pub(crate) fn shot(shot: &BowShot) -> Vec<EntityAction> {
    SHOOT_BOW.invoke(shot);
    shot.actions.borrow_mut().drain(..).collect()
}

/// Whether anyone listens to spawns or flights, so the game-side reach only builds their payloads when someone does.
pub(crate) fn spawn_heard() -> bool {
    !SPAWN.is_empty()
}

pub(crate) fn flight_heard() -> bool {
    !FLIGHT.is_empty()
}

pub(crate) fn spawned(spawn: &ProjectileSpawned) {
    SPAWN.invoke(spawn);
}

pub(crate) fn flight(flight: &ProjectileFlight) {
    FLIGHT.invoke(flight);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::base::Identifier;
    use crate::pure::world_api_v1::{with_world, BlockState, MemoryWorldOps};

    #[test]
    fn a_bow_shot_carries_the_hands_and_collects_the_players_actions_while_a_launch_is_heard_as_it_is() {
        use super::super::entity_events_v1::ActionTarget;
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ProjectileEvents::LAUNCH.register(move |launch| if launch.projectile == 4040 { log.lock().unwrap().push((launch.player, launch.kind.clone())); });
        ProjectileEvents::SHOOT_BOW.register(|shot| {
            if shot.projectile != 4040 { return; }
            if shot.force >= 1.0 && shot.main_hand.as_ref().is_some_and(|bow| bow.item == "example:longbow") {
                shot.actions.borrow_mut().push(EntityAction::AddEffect { target: ActionTarget::Attacker, effect: "minecraft:speed".into(), amplifier: 1, duration: 60 });
            }
        });
        launched(&ProjectileLaunch { player: Some(9), name: "archer".into(), projectile: 4040, kind: "minecraft:arrow".into(), motion: [0.0, 0.5, -3.0], data: NbtCompound::new() });
        launched(&ProjectileLaunch { player: None, name: String::new(), projectile: 4040, kind: "minecraft:arrow".into(), motion: [0.0; 3], data: NbtCompound::new() });
        assert_eq!(*seen.lock().unwrap(), vec![(Some(9), "minecraft:arrow".to_string()), (None, "minecraft:arrow".to_string())]);
        let shot_with = |force: f32, item: &str| BowShot {
            player: 9, name: "archer".into(), projectile: 4040, force,
            main_hand: Some(StackData { item: item.into(), count: 1, ..StackData::default() }), off_hand: None, actions: RefCell::new(Vec::new()),
        };
        assert_eq!(shot(&shot_with(1.0, "example:longbow")), vec![EntityAction::AddEffect { target: ActionTarget::Attacker, effect: "minecraft:speed".into(), amplifier: 1, duration: 60 }]);
        assert!(shot(&shot_with(0.4, "example:longbow")).is_empty(), "a weak draw queues nothing");
        assert!(shot(&shot_with(1.0, "minecraft:bow")).is_empty());
    }

    #[test]
    fn an_ammo_request_can_supply_a_projectile_from_a_carried_quiver_and_shrink_it() {
        use super::super::entity_events_v1::{CarriedSlot, StackSlot};
        use crate::pure::base::{Nbt, NbtCompound, NbtCompoundExt};
        ProjectileEvents::AMMO.register(|request| {
            if request.name != "quiverer" { return; }
            // The quiver in the off hand holds two arrows in its custom data; supply one and write the quiver back with one less.
            let quiver = request.carried.iter().find(|carried| carried.slot == CarriedSlot::Equipment(StackSlot::OffHand)).expect("the quiver is carried");
            let held = quiver.stack.custom_data.get_int("arrows", 0);
            assert_eq!(held, 2);
            let mut arrow = ItemSlot::new(Identifier::parse("minecraft:arrow").unwrap(), 1);
            arrow.custom_data = quiver.stack.custom_data.clone();
            request.supplied.borrow_mut().replace(arrow);
            let mut shrunk = quiver.stack.clone();
            shrunk.custom_data.insert("arrows".into(), Nbt::Int((held - 1).try_into().unwrap()));
            request.edits.borrow_mut().push(CarriedEdit { slot: quiver.slot, stack: shrunk });
        });
        let mut quiver_data = NbtCompound::new();
        quiver_data.insert("arrows".into(), Nbt::Int(2));
        let mut quiver = ItemSlot::new(Identifier::parse("example:quiver").unwrap(), 1);
        quiver.custom_data = quiver_data;
        let request = AmmoRequest {
            player: Some(3), name: "quiverer".into(), entity: None, kind: "minecraft:player".into(),
            weapon: StackData { item: "minecraft:bow".into(), count: 1, ..StackData::default() },
            vanilla: None,
            carried: vec![CarriedStack { slot: CarriedSlot::Equipment(StackSlot::OffHand), stack: quiver }],
            supplied: RefCell::new(None), edits: RefCell::new(Vec::new()),
        };
        let (supplied, edits) = ammo(&request);
        let supplied = supplied.expect("the callback supplied the projectile");
        assert_eq!(supplied.item, Identifier::parse("minecraft:arrow").unwrap());
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].stack.custom_data.get_int("arrows", 0), 1, "the quiver is written back with one arrow less");
        // A request nobody answers keeps the game's pick: no supplied stack, no edits.
        let plain = AmmoRequest { name: "plain".into(), ..request };
        assert_eq!(ammo(&plain), (None, Vec::new()));
    }

    #[test]
    fn a_players_shot_weapon_names_the_hand_that_launched_and_a_mobs_does_not() {
        use crate::pure::base::{Nbt, NbtCompound};
        let mut bow_data = NbtCompound::new();
        bow_data.insert("tic_level".into(), Nbt::Int(2));
        let bow = StackData { item: "example:longbow".into(), count: 1, custom_data: bow_data, ..StackData::default() };
        let sword = StackData { item: "example:sword".into(), count: 1, ..StackData::default() };
        let request = |player: Option<u128>, weapon: StackData| AmmoRequest {
            player, name: "archer".into(), entity: None, kind: "minecraft:player".into(), weapon,
            vanilla: None, carried: Vec::new(), supplied: RefCell::new(None), edits: RefCell::new(Vec::new()),
        };
        // No weapon noted yet: no hand is named.
        assert_eq!(launcher_for(&Some(bow.clone()), &None), None);
        // A mob's request leaves the note alone (a skeleton's bow never names a player's hand).
        ammo(&request(None, bow.clone()));
        assert_eq!(launcher_for(&Some(bow.clone()), &None), None);
        // A player's request notes the weapon: the hand holding it is the launcher, main first on a tie.
        ammo(&request(Some(7), bow.clone()));
        assert_eq!(launcher_for(&Some(sword.clone()), &Some(bow.clone())), Some((StackSlot::OffHand, bow.clone())));
        assert_eq!(launcher_for(&Some(bow.clone()), &Some(bow.clone())), Some((StackSlot::MainHand, bow.clone())));
        assert_eq!(launcher_for(&Some(sword.clone()), &None), None, "the weapon left the hands");
        // A later shot overwrites the note.
        let other = StackData { item: "example:crossbow".into(), count: 1, ..StackData::default() };
        ammo(&request(Some(7), other.clone()));
        assert_eq!(launcher_for(&Some(bow), &Some(other.clone())), Some((StackSlot::OffHand, other)));
    }

    #[test]
    fn a_listener_acts_on_the_world_where_the_projectile_landed_and_can_cancel_the_hit() {
        use crate::pure::base::Nbt;
        assert!(!heard());
        ProjectileEvents::IMPACT.register(|hit| {
            match hit.kind.as_str() {
                "example:light_ball" => {
                    hit.world().set_block_state(hit.block, &BlockState::new(Identifier::parse("minecraft:light").unwrap()));
                    false
                }
                // A bounce: the hit is cancelled and the ball flies on with its vertical motion reflected.
                "example:bounce_ball" => {
                    assert_eq!(hit.motion, [0.0, -1.5, 0.25], "the motion it arrived with");
                    assert_eq!(hit.data.get("charge"), Some(&Nbt::Int(3)), "the data it was fired with");
                    let mut reflected = hit.motion;
                    reflected[1] = -reflected[1];
                    hit.motion_after.borrow_mut().replace(reflected);
                    false
                }
                // A burst: its hit lands, and the projectile is discarded after the callbacks.
                "example:burst_ball" => {
                    hit.discard.set(true);
                    true
                }
                _ => true,
            }
        });
        assert!(heard());
        let hit = |kind: &str| {
            let mut data = NbtCompound::new();
            data.insert("charge".into(), Nbt::Int(3));
            ProjectileImpact {
                dimension: "minecraft:overworld".into(),
                projectile: 7,
                kind: kind.into(),
                owner: 1,
                entity: None,
                entity_kind: String::new(),
                entity_effects: Vec::new(),
                block: [4, 70, 4],
                face: [0, 1, 0],
                at: [4.5, 71.0, 4.5],
                motion: [0.0, -1.5, 0.25],
                data,
                piercing_ignored: Vec::new(),
                pierced_and_killed: Vec::new(),
                shot_punch: 0,
                discard: Cell::new(false),
                motion_after: RefCell::new(None),
                ignore_entities: RefCell::new(Vec::new()),
                actions: RefCell::new(Vec::new()),
            }
        };
        let mut ops = MemoryWorldOps::default();
        with_world(&mut ops, || {
            assert!(!impact(&hit("example:light_ball")), "the ball's hit is cancelled");
            assert!(impact(&hit("minecraft:snowball")), "other projectiles hit as usual");
        });
        assert_eq!(ops.blocks.len(), 1);
        let bounce = hit("example:bounce_ball");
        assert!(!impact(&bounce), "the bounce cancels the hit");
        assert_eq!(*bounce.motion_after.borrow(), Some([0.0, 1.5, 0.25]), "the reflected motion is what it flies on with");
        assert!(!bounce.discard.get());
        let burst = hit("example:burst_ball");
        assert!(impact(&burst), "the burst's hit itself lands");
        assert!(burst.discard.get(), "and the projectile is discarded after the callbacks");
        assert_eq!(*burst.motion_after.borrow(), None);
    }

    #[test]
    fn a_spawn_callback_sets_the_shot_up_and_a_flight_callback_steers_it_home() {
        assert!(!spawn_heard());
        assert!(!flight_heard());
        ProjectileEvents::SPAWN.register(|spawn| {
            if spawn.projectile != 9090 { return; }
            // A clamped direction and a pierce level, set before the first tick.
            spawn.motion_after.borrow_mut().replace([0.0, 0.2, -2.0]);
            spawn.pierce.set(Some(3));
            // A supplied arrow is creative-only pickup: it cannot be picked up again.
            spawn.pickup.set(Some(ArrowPickup::CreativeOnly));
            spawn.no_gravity.set(Some(true));
        });
        ProjectileEvents::FLIGHT.register(|flight| {
            if flight.projectile != 9090 { return; }
            match flight.age {
                // The hover ends: gravity comes back, and the shot's pickup is settled now that
                // the profile the shot bound is known.
                20 => {
                    flight.no_gravity_after.set(Some(false));
                    flight.pickup_after.set(Some(ArrowPickup::CreativeOnly));
                }
                // Stuck long enough with loyalty: free it and steer it home.
                25 => {
                    flight.release_from_ground.set(true);
                    flight.motion_after.borrow_mut().replace([0.0, 0.5, 0.0]);
                    flight.actions.borrow_mut().push(FlightAction::PullItems { center: flight.position, radius: 3.0, strength: 0.05 });
                }
                // Home: hand the tool back and end the flight.
                30 => {
                    flight.actions.borrow_mut().push(FlightAction::GiveStack { stack: StackData { item: "example:thrown_tool".into(), count: 1, ..StackData::default() } });
                    flight.discard.set(true);
                }
                _ => {}
            }
        });
        assert!(spawn_heard());
        assert!(flight_heard());
        let spawn = ProjectileSpawned {
            dimension: "minecraft:overworld".into(), projectile: 9090, kind: "minecraft:arrow".into(), owner: 4,
            position: [0.5, 64.0, 0.5], motion: [0.1, 0.2, -2.0],
            arrow: Some(ArrowFacts { pierce: 0, in_ground: false, in_ground_time: 0, dealt_damage: false }),
            stack: None, data: NbtCompound::new(),
            motion_after: RefCell::new(None), pierce: Cell::new(None), pickup: Cell::new(None), no_gravity: Cell::new(None), pickup_stack: RefCell::new(None),
        };
        assert_eq!(spawn.pickup.get(), None, "no pickup status is settled until a callback sets one");
        spawned(&spawn);
        assert_eq!(*spawn.motion_after.borrow(), Some([0.0, 0.2, -2.0]));
        assert_eq!(spawn.pierce.get(), Some(3));
        assert_eq!(spawn.pickup.get(), Some(ArrowPickup::CreativeOnly));
        assert_eq!(spawn.no_gravity.get(), Some(true));
        let flight_at = |age: u32| ProjectileFlight {
            dimension: "minecraft:overworld".into(), projectile: 9090, kind: "minecraft:arrow".into(), owner: 4,
            owner_position: Some([0.5, 64.0, 3.5]), age, position: [0.5, 64.0, 0.5], motion: [0.0; 3],
            in_ground: age >= 25, no_gravity: age < 20,
            arrow: Some(ArrowFacts { pierce: 3, in_ground: age >= 25, in_ground_time: 5, dealt_damage: false }),
            data: NbtCompound::new(),
            motion_after: RefCell::new(None), discard: Cell::new(false), no_gravity_after: Cell::new(None), pickup_after: Cell::new(None),
            release_from_ground: Cell::new(false), actions: RefCell::new(Vec::new()),
        };
        let hover_end = flight_at(20);
        assert_eq!(hover_end.pickup_after.get(), None, "no pickup status is settled until a callback sets one");
        flight(&hover_end);
        assert_eq!(hover_end.no_gravity_after.get(), Some(false));
        assert_eq!(hover_end.pickup_after.get(), Some(ArrowPickup::CreativeOnly));
        let stuck = flight_at(25);
        flight(&stuck);
        assert!(stuck.release_from_ground.get());
        assert_eq!(*stuck.motion_after.borrow(), Some([0.0, 0.5, 0.0]));
        assert_eq!(stuck.actions.borrow().as_slice(), [FlightAction::PullItems { center: [0.5, 64.0, 0.5], radius: 3.0, strength: 0.05 }]);
        let home = flight_at(30);
        flight(&home);
        assert!(home.discard.get());
        assert_eq!(home.actions.borrow().len(), 1);
    }

    #[test]
    fn a_listener_queues_actions_for_the_hits_parties() {
        use super::super::entity_events_v1::ActionTarget;
        ProjectileEvents::IMPACT.register(|hit| {
            if hit.kind == "example:venom_ball" {
                if let Some(victim) = hit.entity {
                    let _ = victim;
                    hit.actions.borrow_mut().push(EntityAction::AddEffect { target: ActionTarget::Victim, effect: "minecraft:poison".into(), amplifier: 0, duration: 60 });
                }
            }
            true
        });
        let hit = ProjectileImpact {
            dimension: "minecraft:overworld".into(),
            projectile: 8,
            kind: "example:venom_ball".into(),
            owner: 3,
            entity: Some(12),
            entity_kind: "minecraft:zombie".into(),
            entity_effects: Vec::new(),
            block: [4, 70, 4],
            face: [0, 1, 0],
            at: [4.5, 71.0, 4.5],
            motion: [0.0; 3],
            data: NbtCompound::new(),
            piercing_ignored: Vec::new(),
            pierced_and_killed: Vec::new(),
            shot_punch: 0,
            discard: Cell::new(false),
            motion_after: RefCell::new(None),
            ignore_entities: RefCell::new(Vec::new()),
            actions: RefCell::new(Vec::new()),
        };
        assert!(impact(&hit));
        assert_eq!(
            hit.actions.borrow().as_slice(),
            [EntityAction::AddEffect { target: ActionTarget::Victim, effect: "minecraft:poison".into(), amplifier: 0, duration: 60 }],
            "the queued action stays on the hit for the host to do"
        );
    }

    #[test]
    fn a_piercing_hit_reports_its_ignore_list_and_a_callback_records_the_victim_on_it() {
        ProjectileEvents::IMPACT.register(|hit| {
            if hit.kind == "example:piercing_arrow" {
                // The snapshot already holds the victim the game added before telling the hit.
                assert_eq!(hit.piercing_ignored, vec![12], "the ignore list as the game reports it");
                if let Some(victim) = hit.entity {
                    hit.ignore_entities.borrow_mut().push(victim);
                }
            }
            true
        });
        let hit = ProjectileImpact {
            dimension: "minecraft:overworld".into(),
            projectile: 9,
            kind: "example:piercing_arrow".into(),
            owner: 3,
            entity: Some(12),
            entity_kind: "minecraft:enderman".into(),
            entity_effects: Vec::new(),
            block: [4, 70, 4],
            face: [0, 1, 0],
            at: [4.5, 71.0, 4.5],
            motion: [0.0; 3],
            data: NbtCompound::new(),
            piercing_ignored: vec![12],
            pierced_and_killed: Vec::new(),
            shot_punch: 0,
            discard: Cell::new(false),
            motion_after: RefCell::new(None),
            ignore_entities: RefCell::new(Vec::new()),
            actions: RefCell::new(Vec::new()),
        };
        assert!(impact(&hit));
        assert_eq!(
            hit.ignore_entities.borrow().as_slice(),
            [12],
            "the recorded victim stays on the hit for the host to add to the game's list"
        );
    }


    fn arrow_pickup_codes_are_the_host_saved_bytes() {
        // The transport carries the status as a byte (`SpawnOutcome.pickup`); the codes must stay
        // the host `Pickup.byte` ordinals (0 disallowed, 1 allowed, 2 creative-only).
        assert_eq!((ArrowPickup::Disallowed as u8, ArrowPickup::Allowed as u8, ArrowPickup::CreativeOnly as u8), (0, 1, 2));
        assert_eq!(ArrowPickup::default(), ArrowPickup::Disallowed);
    }
}
