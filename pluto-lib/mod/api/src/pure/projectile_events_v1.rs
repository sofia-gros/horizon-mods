//! Projectiles hitting things (Forge: `ProjectileImpactEvent`). A callback sees what flew, who threw it and what it met, can act on the
//! world with [`World`] and can cancel the hit.
//!
//! What no event here can do is follow a projectile in flight: the game's entity-tick telling covers mobs, item entities, minecarts
//! and boats, not projectiles (pinned `native_entity_tick_pre`, API 146), so a callback hears a projectile when it enters
//! ([`ProjectileEvents::LAUNCH`]) and when it lands ([`ProjectileEvents::IMPACT`]) and nowhere between. Flight behaviours — a
//! projectile that falls faster, a burning fuse, one that pulls mobs toward it or lights its path — stay unimplementable until the
//! game tells projectile ticks. What acts at the two ends is all here: the impact reports the motion and the saved data the
//! projectile arrived with and lets a callback discard it or send it flying on with a new motion, which is what pierce, bounce
//! and an explosion where it lands are made of.

use std::cell::{Cell, RefCell};

use super::base::{Event, NbtCompound};
use super::block_entity_api_v1::{BlockPos, ItemSlot};
use super::entity_events_v1::{CarriedEdit, CarriedStack, EntityAction};
use super::player_api::StackData;
use super::world_api_v1::World;

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
    /// Set to true to discard the projectile once the callbacks ran: the game removes it as the hook
    /// returns, before its own hit handling continues, so cancel the hit as well when the hit itself
    /// should not land. How a projectile that explodes where it lands consumes itself.
    pub discard: Cell<bool>,
    /// The motion the projectile flies on with, which a callback may set: applied as the hook returns
    /// when the projectile is not discarded. With a cancelled hit this is a bounce — reflect
    /// [`ProjectileImpact::motion`] yourself; the game does not reflect it for you.
    pub motion_after: RefCell<Option<[f64; 3]>>,
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

static LAUNCH: Event<ProjectileLaunch, ()> = Event::void();
static SHOOT_BOW: Event<BowShot, ()> = Event::void();
static IMPACT: Event<ProjectileImpact, bool> = Event::allow_all();
static AMMO: Event<AmmoRequest, ()> = Event::void();

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
    /// modifier answers from. Server only.
    pub const AMMO: &'static Event<AmmoRequest, ()> = &AMMO;
}

/// Whether anyone asks for ammunition, so the host builds the carried stacks only when someone does.
pub(crate) fn ammo_heard() -> bool {
    !AMMO.is_empty()
}

/// The ammunition the callbacks settled on: what they set in `supplied` (None when they left it alone, so the host keeps the
/// game's own pick and its own consumption; a set stack of count 0 means no projectile) and the edits to the shooter's
/// carried stacks.
pub(crate) fn ammo(request: &AmmoRequest) -> (Option<ItemSlot>, Vec<CarriedEdit>) {
    AMMO.invoke(request);
    (request.supplied.borrow_mut().take(), request.edits.borrow_mut().drain(..).collect())
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

/// The actions the callbacks queued for the player who shot.
pub(crate) fn shot(shot: &BowShot) -> Vec<EntityAction> {
    SHOOT_BOW.invoke(shot);
    shot.actions.borrow_mut().drain(..).collect()
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
                block: [4, 70, 4],
                face: [0, 1, 0],
                at: [4.5, 71.0, 4.5],
                motion: [0.0, -1.5, 0.25],
                data,
                discard: Cell::new(false),
                motion_after: RefCell::new(None),
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
            block: [4, 70, 4],
            face: [0, 1, 0],
            at: [4.5, 71.0, 4.5],
            motion: [0.0; 3],
            data: NbtCompound::new(),
            discard: Cell::new(false),
            motion_after: RefCell::new(None),
            actions: RefCell::new(Vec::new()),
        };
        assert!(impact(&hit));
        assert_eq!(
            hit.actions.borrow().as_slice(),
            [EntityAction::AddEffect { target: ActionTarget::Victim, effect: "minecraft:poison".into(), amplifier: 0, duration: 60 }],
            "the queued action stays on the hit for the host to do"
        );
    }
}
