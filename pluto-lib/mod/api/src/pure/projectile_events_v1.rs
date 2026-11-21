//! Projectiles hitting things (Forge: `ProjectileImpactEvent`). A callback sees what flew, who threw it and what it met, can act on the
//! world with [`World`], can queue actions for the hit's parties and can cancel the hit.

use std::cell::RefCell;

use super::base::Event;
use super::block_entity_api_v1::BlockPos;
use super::entity_events_v1::EntityAction;
use super::player_api::StackData;
use super::world_api_v1::World;

/// A projectile that has found what it meets.
#[derive(Clone, Debug, PartialEq)]
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

static LAUNCH: Event<ProjectileLaunch, ()> = Event::void();
static SHOOT_BOW: Event<BowShot, ()> = Event::void();
static IMPACT: Event<ProjectileImpact, bool> = Event::allow_all();

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
        launched(&ProjectileLaunch { player: Some(9), name: "archer".into(), projectile: 4040, kind: "minecraft:arrow".into() });
        launched(&ProjectileLaunch { player: None, name: String::new(), projectile: 4040, kind: "minecraft:arrow".into() });
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
    fn a_listener_acts_on_the_world_where_the_projectile_landed_and_can_cancel_the_hit() {
        assert!(!heard());
        ProjectileEvents::IMPACT.register(|hit| {
            if hit.kind == "example:light_ball" {
                hit.world().set_block_state(hit.block, &BlockState::new(Identifier::parse("minecraft:light").unwrap()));
                false
            } else {
                true
            }
        });
        assert!(heard());
        let hit = |kind: &str| ProjectileImpact {
            dimension: "minecraft:overworld".into(),
            projectile: 7,
            kind: kind.into(),
            owner: 1,
            entity: None,
            entity_kind: String::new(),
            block: [4, 70, 4],
            face: [0, 1, 0],
            at: [4.5, 71.0, 4.5],
            actions: RefCell::new(Vec::new()),
        };
        let mut ops = MemoryWorldOps::default();
        with_world(&mut ops, || {
            assert!(!impact(&hit("example:light_ball")), "the ball's hit is cancelled");
            assert!(impact(&hit("minecraft:snowball")), "other projectiles hit as usual");
        });
        assert_eq!(ops.blocks.len(), 1);
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
