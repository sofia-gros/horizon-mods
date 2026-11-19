//! Counterpart of Fabric's `fabric-events-interaction-v0`: callbacks for what a player does with items, blocks and entities.
//!
//! Callback data is reduced and owned (Fabric passes player, world, hand and hit result). Results follow Fabric: the first
//! callback that does not return `ActionResult::Pass` decides; any other result stops the game's own handling.

use std::cell::{Cell, RefCell};

use super::base::{ActionResult, Event};
use super::block_entity_api_v1::ItemSlot;
use super::player_api::{ServerPlayer, StackData};
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
#[derive(Clone, Debug, PartialEq)]
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
    /// The stack that was used, with its damage, max damage and custom data (a tool a mod made keeps its state there).
    pub held: Option<super::player_api::StackData>,
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
    /// The digging player's state when the game says who digs (Player.onGround, isEyeInFluid(water)).
    pub player: Option<DiggingPlayer>,
}

/// What a break-speed callback may read about the player who digs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiggingPlayer {
    /// On the ground, as the player's last accepted move said.
    pub on_ground: bool,
    /// The player's eyes are inside water (their block, at eye height, holds water).
    pub eyes_in_water: bool,
}

/// The hand a player used an item in (Minecraft: `InteractionHand`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    /// The main hand.
    Main,
    /// The off hand.
    Off,
}

/// A player finished using an item over time (ate or drank it).
#[derive(Clone, Debug, PartialEq)]
pub struct UseItemFinishContext {
    /// The item id, such as `tconstruct:cheese_ingot`.
    pub item: String,
    /// The player; its methods work inside the callback.
    pub player: ServerPlayer,
    /// The stack that was used, with its damage, max damage and custom data, as it was when the use started.
    pub stack: StackData,
    /// The hand the stack was used in.
    pub hand: Hand,
    /// What that hand holds now, with all its components, after the game's own food and remainder were given; None when the hand is empty
    /// (the last of a stack was eaten). To change it, put the changed copy in `replacement`.
    pub slot: Option<ItemSlot>,
    /// A whole new stack for the hand, which a callback may set (clone `slot`, edit the clone, store it here); the later callbacks see the
    /// one before. A count of 0 empties the hand. Left None, the hand keeps what it holds.
    pub replacement: RefCell<Option<ItemSlot>>,
    /// The dimension the player is in, such as `minecraft:overworld`.
    pub dimension: String,
    /// Where the player is.
    pub position: [f64; 3],
    /// Whether the player is in creative mode (nothing is used up).
    pub creative: bool,
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
/// have been given; only for a player on the server. The context names the stack, the hand and the player's place, and a callback may change
/// what the hand holds through `replacement`.
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
    /// Fabric: `CANCELED`. Called once when `BEFORE` vetoes a break; the block is kept.
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

/// Calls the finish callbacks and returns the stack a callback put in the hand, if any.
pub(crate) fn use_item_finish(context: &UseItemFinishContext) -> Option<ItemSlot> {
    USE_ITEM_FINISH.invoke(context);
    context.replacement.borrow_mut().take()
}

/// The dig speed after every callback has had its say.
pub(crate) fn break_speed(pos: [i32; 3], block: BlockState, original: f32, speed: f32, held: Option<super::player_api::StackData>, player: Option<DiggingPlayer>) -> f32 {
    let context = BreakSpeedContext { pos, block, original, speed: Cell::new(speed), held, player };
    BREAK_SPEED.invoke(&context);
    context.speed.get()
}

pub(crate) fn attack_entity(entity: i32, kind: &str, pos: [f64; 3]) -> bool {
    handled(ATTACK_ENTITY.invoke(&AttackEntityContext { entity, kind: kind.to_owned(), pos }))
}

/// True when the break may go ahead.
pub(crate) fn before_block_break(pos: [i32; 3]) -> bool {
    let context = BlockBreakContext { pos };
    let allowed = BEFORE_BLOCK_BREAK.invoke(&context);
    if !allowed { BLOCK_BREAK_CANCELED.invoke(&context); }
    allowed
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

    fn finish(item: &str, creative: bool) -> UseItemFinishContext {
        let mut data = StackData { item: item.into(), count: 1, damage: 3, ..Default::default() };
        data.custom_data.insert("bites".into(), super::super::base::Nbt::Int(2));
        let mut slot = ItemSlot::new(super::super::base::Identifier::parse(item).unwrap(), 1);
        slot.damage = 3;
        UseItemFinishContext {
            item: item.into(), player: ServerPlayer { uuid: 7, name: "Alex".into() }, stack: data, hand: Hand::Off, slot: Some(slot),
            replacement: RefCell::new(None), dimension: "minecraft:overworld".into(), position: [1.0, 64.0, 2.0], creative,
        }
    }

    #[test]
    fn a_finish_callback_sees_the_stack_and_hand_and_may_put_a_new_stack_in_the_hand() {
        UseItemFinishCallback::EVENT.register(|done| {
            if done.item != "test:snack_tool" || done.creative { return; }
            assert_eq!((done.hand, done.stack.damage, done.stack.custom_data.len()), (Hand::Off, 3, 1));
            // wear the hand's stack by two, as an edible tool's durability cost does
            let mut worn = done.slot.clone().expect("the hand still holds the tool");
            worn.damage += 2;
            *done.replacement.borrow_mut() = Some(worn);
        });
        UseItemFinishCallback::EVENT.register(|done| {
            // a later callback sees the earlier one's replacement
            if done.item == "test:snack_tool" && !done.creative {
                let seen = done.replacement.borrow().as_ref().map(|slot| slot.damage);
                assert_eq!(seen, Some(5));
            }
        });
        let used = finish("test:snack_tool", false);
        assert_eq!(use_item_finish(&used).map(|slot| slot.damage), Some(5));
        assert!(used.replacement.borrow().is_none(), "the replacement is handed over once");
        assert_eq!(use_item_finish(&finish("test:snack_tool", true)), None, "a callback that sets nothing leaves the hand alone");
        assert_eq!(use_item_finish(&finish("test:other", false)), None);
    }

    #[test]
    fn break_speed_callbacks_read_the_digging_player_and_change_the_speed() {
        BreakSpeedCallback::EVENT.register(|c| {
            if let Some(player) = c.player {
                if !player.on_ground { c.speed.set(c.speed.get() * 5.0); }
                if player.eyes_in_water { c.speed.set(c.speed.get() + 1.0); }
            }
        });
        let block = BlockState { block: super::super::base::Identifier::parse("test:rock").unwrap(), properties: Default::default() };
        let state = |on_ground, eyes_in_water| Some(DiggingPlayer { on_ground, eyes_in_water });
        assert_eq!(break_speed([0, 0, 0], block.clone(), 2.0, 2.0, None, state(true, false)), 2.0);
        assert_eq!(break_speed([0, 0, 0], block.clone(), 2.0, 2.0, None, state(false, false)), 10.0);
        assert_eq!(break_speed([0, 0, 0], block.clone(), 2.0, 2.0, None, state(true, true)), 3.0);
        assert_eq!(break_speed([0, 0, 0], block, 2.0, 2.0, None, None), 2.0, "no player state leaves the speed alone");
    }

    #[test]
    fn a_veto_before_a_block_break_cancels_it() {
        let canceled = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = canceled.clone();
        PlayerBlockBreakEvents::CANCELED.register(move |c| seen.lock().unwrap().push(c.pos));
        PlayerBlockBreakEvents::BEFORE.register(|c| c.pos[1] >= 0);
        PlayerBlockBreakEvents::BEFORE.register(|c| c.pos[1] >= -1);
        assert!(before_block_break([0, 64, 0]));
        assert!(canceled.lock().unwrap().is_empty(), "allowed breaks are not canceled");
        assert!(!before_block_break([0, -5, 0]));
        assert_eq!(*canceled.lock().unwrap(), [[0, -5, 0]], "two vetoes still report one cancellation");
    }
}
