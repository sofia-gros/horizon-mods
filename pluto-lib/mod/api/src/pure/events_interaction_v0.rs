//! Counterpart of Fabric's `fabric-events-interaction-v0`: callbacks for what a player does with items, blocks and entities.
//!
//! Callback data is reduced and owned (Fabric passes player, world, hand and hit result). Results follow Fabric: the first
//! callback that does not return `ActionResult::Pass` decides; any other result stops the game's own handling.

use std::cell::Cell;

use super::base::{ActionResult, Event};
use super::player_api::ServerPlayer;
use super::world_api_v1::BlockState;

/// A player used an item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseItemContext {
    /// The item id, such as `tconstruct:flint_pickaxe`.
    pub item: String,
    pub count: u32,
}

/// A player used (right-clicked) a block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseBlockContext {
    pub pos: [i32; 3],
    /// The face that was clicked, as a unit offset.
    pub face: [i32; 3],
}

/// A player used an item on a block: the held item, the block it was used on and who did it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseItemOnBlockContext {
    /// The item id, such as `tconstruct:earth_slime_grass_seeds`.
    pub item: String,
    /// How many of the item are held.
    pub count: u32,
    /// The block it was used on.
    pub pos: [i32; 3],
    /// The face that was clicked, as a unit offset.
    pub face: [i32; 3],
    /// The dimension, such as `minecraft:overworld`.
    pub dimension: String,
    /// The player; its methods work inside the callback.
    pub player: ServerPlayer,
    /// Whether the player is in creative mode (nothing is used up).
    pub creative: bool,
}

/// How fast the player digs a block (NeoForge: `PlayerEvent.BreakSpeed`).
#[derive(Debug)]
pub struct BreakSpeedContext {
    /// The block being dug.
    pub pos: [i32; 3],
    /// Its state.
    pub block: BlockState,
    /// The speed as vanilla works it out (the tool, efficiency, haste and fatigue, the water); it does not change.
    pub original: f32,
    /// The speed now, which callbacks may change with `set`; every callback is called in turn, each seeing the last one's.
    pub speed: Cell<f32>,
    /// The stack the player digs with, with its damage, max damage and custom data; None for an empty hand or when the game does not
    /// say who digs.
    pub held: Option<super::player_api::StackData>,
}

/// A player finished using an item over time (ate or drank it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseItemFinishContext {
    /// The item id, such as `tconstruct:cheese_ingot`.
    pub item: String,
    /// The player; its methods work inside the callback.
    pub player: ServerPlayer,
}

/// A player attacked an entity.
#[derive(Clone, Debug, PartialEq)]
pub struct AttackEntityContext {
    pub entity: i32,
    pub kind: String,
    pub pos: [f64; 3],
}

/// A block break (before it happens).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockBreakContext {
    pub pos: [i32; 3],
}

static USE_ITEM: Event<UseItemContext, ActionResult> = Event::first_not_pass();
static USE_BLOCK: Event<UseBlockContext, ActionResult> = Event::first_not_pass();
static USE_ITEM_ON_BLOCK: Event<UseItemOnBlockContext, ActionResult> = Event::first_not_pass();
static USE_ITEM_FINISH: Event<UseItemFinishContext, ()> = Event::void();
static BREAK_SPEED: Event<BreakSpeedContext, ()> = Event::void();
static ATTACK_ENTITY: Event<AttackEntityContext, ActionResult> = Event::first_not_pass();
static BEFORE_BLOCK_BREAK: Event<BlockBreakContext, bool> = Event::allow_all();
static AFTER_BLOCK_BREAK: Event<BlockBreakContext, ()> = Event::void();
static BLOCK_BREAK_CANCELED: Event<BlockBreakContext, ()> = Event::void();

/// Fabric: `UseItemCallback.EVENT`.
pub struct UseItemCallback;

impl UseItemCallback {
    pub const EVENT: &'static Event<UseItemContext, ActionResult> = &USE_ITEM;
}

/// Fabric: `UseBlockCallback.EVENT`.
pub struct UseBlockCallback;

impl UseBlockCallback {
    pub const EVENT: &'static Event<UseBlockContext, ActionResult> = &USE_BLOCK;
}

/// A player used an item on a block (like Fabric's `UseBlockCallback`, which also fires for an empty hand, with the held item and the player
/// in the context). Answer `Success` after doing the job, and the game does not handle it.
pub struct UseItemOnBlockCallback;

impl UseItemOnBlockCallback {
    pub const EVENT: &'static Event<UseItemOnBlockContext, ActionResult> = &USE_ITEM_ON_BLOCK;
}

/// A player finished eating or drinking an item (NeoForge: `LivingEntityUseItemEvent.Finish`). Called after the food and the item's own effects
/// have been given; only for a player on the server.
pub struct UseItemFinishCallback;

impl UseItemFinishCallback {
    pub const EVENT: &'static Event<UseItemFinishContext, ()> = &USE_ITEM_FINISH;
}

/// How fast a player digs a block (NeoForge: `PlayerEvent.BreakSpeed`). Not asked in creative mode or for an unbreakable block.
pub struct BreakSpeedCallback;

impl BreakSpeedCallback {
    pub const EVENT: &'static Event<BreakSpeedContext, ()> = &BREAK_SPEED;
}

/// Fabric: `AttackEntityCallback.EVENT`.
pub struct AttackEntityCallback;

impl AttackEntityCallback {
    pub const EVENT: &'static Event<AttackEntityContext, ActionResult> = &ATTACK_ENTITY;
}

/// Fabric: `PlayerBlockBreakEvents`.
pub struct PlayerBlockBreakEvents;

impl PlayerBlockBreakEvents {
    /// Fabric: `BEFORE`: return `false` to cancel the break.
    pub const BEFORE: &'static Event<BlockBreakContext, bool> = &BEFORE_BLOCK_BREAK;
    /// Fabric: `AFTER`. Stub: no matching Horizon hook point is wired yet (see the support matrix).
    pub const AFTER: &'static Event<BlockBreakContext, ()> = &AFTER_BLOCK_BREAK;
    /// Fabric: `CANCELED`. Stub: no matching Horizon hook point is wired yet.
    pub const CANCELED: &'static Event<BlockBreakContext, ()> = &BLOCK_BREAK_CANCELED;
}

/// True when the callbacks' result means the game must not handle the action itself.
fn handled(result: ActionResult) -> bool {
    result != ActionResult::Pass
}

pub(crate) fn use_item(item: &str, count: u32) -> bool {
    handled(USE_ITEM.invoke(&UseItemContext { item: item.to_owned(), count }))
}

pub(crate) fn use_block(pos: [i32; 3], face: [i32; 3]) -> bool {
    handled(USE_BLOCK.invoke(&UseBlockContext { pos, face }))
}

pub(crate) fn use_item_on_block(context: &UseItemOnBlockContext) -> bool {
    handled(USE_ITEM_ON_BLOCK.invoke(context))
}

pub(crate) fn use_item_finish(context: &UseItemFinishContext) {
    USE_ITEM_FINISH.invoke(context);
}

/// The dig speed after every callback has had its say.
pub(crate) fn break_speed(pos: [i32; 3], block: BlockState, original: f32, speed: f32, held: Option<super::player_api::StackData>) -> f32 {
    let context = BreakSpeedContext { pos, block, original, speed: Cell::new(speed), held };
    BREAK_SPEED.invoke(&context);
    context.speed.get()
}

pub(crate) fn attack_entity(entity: i32, kind: &str, pos: [f64; 3]) -> bool {
    handled(ATTACK_ENTITY.invoke(&AttackEntityContext { entity, kind: kind.to_owned(), pos }))
}

/// True when the break may go ahead.
pub(crate) fn before_block_break(pos: [i32; 3]) -> bool {
    BEFORE_BLOCK_BREAK.invoke(&BlockBreakContext { pos })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_non_pass_result_stops_the_games_handling_and_pass_does_not() {
        UseBlockCallback::EVENT.register(|c| if c.pos == [1, 2, 3] { ActionResult::Success } else { ActionResult::Pass });
        assert!(use_block([1, 2, 3], [0, 1, 0]));
        assert!(!use_block([9, 9, 9], [0, 1, 0]));
    }

    #[test]
    fn a_veto_before_a_block_break_cancels_it() {
        PlayerBlockBreakEvents::BEFORE.register(|c| c.pos[1] >= 0);
        assert!(before_block_break([0, 64, 0]));
        assert!(!before_block_break([0, -5, 0]));
    }
}
