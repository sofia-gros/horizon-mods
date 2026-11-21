//! Counterpart of Fabric's `fabric-events-interaction-v0`: callbacks for what a player does with items, blocks and entities.
//!
//! Callback data is reduced and owned (Fabric passes player, world, hand and hit result). Results follow Fabric: the first
//! callback that does not return `ActionResult::Pass` decides; any other result stops the game's own handling.

use std::cell::{Cell, RefCell};

use super::base::{ActionResult, Event};
use super::block_entity_api_v1::ItemSlot;
use super::entity_events_v1::EntityAction;
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
    /// The block light and the sky light at the block being dug, each 0 to 15 as the level reports them, and how much of the
    /// sky light the time of day and the weather take off; the lights are None when the game did not report them.
    pub block_light: Option<u8>,
    pub sky_light: Option<u8>,
    pub sky_darken: u8,
    /// The temperature of the biome at the block being dug, as `Biome.getTemperature` reports it there; None for a biome
    /// PlutoLib has no temperature for.
    pub biome_temperature: Option<f32>,
}

/// What the world reports about a dig besides the block itself: its light and the temperature of its biome.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DiggingFacts {
    pub block_light: Option<u8>,
    pub sky_light: Option<u8>,
    pub sky_darken: u8,
    pub biome_temperature: Option<f32>,
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

/// A player breaking a block: what `PlayerBlockBreakEvents` reports (Fabric: world, player, pos, state; the block entity is not given, see
/// `AFTER`).
#[derive(Clone, Debug, PartialEq)]
pub struct BlockBreakContext {
    /// The block's position.
    pub pos: [i32; 3],
    /// The dimension it is in, such as `minecraft:overworld`; empty when the game does not name it.
    pub dimension: String,
    /// The block as it was before the break; None when the game cannot read it.
    pub state: Option<BlockState>,
    /// The player who breaks it; None when the game names none.
    pub player: Option<ServerPlayer>,
    /// The stack in the player's main hand: before the break for `BEFORE` and `CANCELED`, at the tick that reports it (with the wear the break caused) for `AFTER`.
    /// None for an empty hand.
    pub held: Option<StackData>,
    /// What to do to the player (`ActionTarget::Attacker`), which callbacks add to. The game does it right after the callbacks ran: for
    /// `BEFORE` even when a callback vetoes the break, for `AFTER` once the break is done. `EditStackData` acts on the main hand as it is then.
    pub actions: RefCell<Vec<EntityAction>>,
}

impl BlockBreakContext {
    /// A context with only a position, for a game that reports nothing else (and for callbacks that only read `pos`).
    pub fn at(pos: [i32; 3]) -> Self {
        Self { pos, dimension: String::new(), state: None, player: None, held: None, actions: RefCell::new(Vec::new()) }
    }
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
    /// Fabric: `AFTER`: a player's break went through and the block is gone. The game has no hook at the end of a break, so it is heard at
    /// the player's next tick (or their next break), at most one tick late, and only when the block at `pos` is no longer the one `BEFORE`
    /// saw. `held` is the tool as of then; the block entity is not given. Creative breaks are heard too. Not heard for the breaks
    /// `BEFORE` does not hear (leaves decaying, a piston, `/setblock`).
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
pub(crate) fn break_speed(pos: [i32; 3], block: BlockState, original: f32, speed: f32, held: Option<super::player_api::StackData>, player: Option<DiggingPlayer>, facts: DiggingFacts) -> f32 {
    let context = BreakSpeedContext { pos, block, original, speed: Cell::new(speed), held, player, block_light: facts.block_light, sky_light: facts.sky_light, sky_darken: facts.sky_darken, biome_temperature: facts.biome_temperature };
    BREAK_SPEED.invoke(&context);
    context.speed.get()
}

pub(crate) fn attack_entity(entity: i32, kind: &str, pos: [f64; 3]) -> bool {
    handled(ATTACK_ENTITY.invoke(&AttackEntityContext { entity, kind: kind.to_owned(), pos }))
}

/// True when the break may go ahead.
pub(crate) fn before_block_break(context: &BlockBreakContext) -> bool {
    let allowed = BEFORE_BLOCK_BREAK.invoke(context);
    if !allowed { BLOCK_BREAK_CANCELED.invoke(context); }
    allowed
}

/// Calls the `AFTER` callbacks for a break that went through.
pub(crate) fn after_block_break(context: &BlockBreakContext) {
    AFTER_BLOCK_BREAK.invoke(context);
}

/// Whether anyone listens to block breaks, so the game only reads the player and the block when someone does.
pub(crate) fn block_break_heard() -> bool {
    !(BEFORE_BLOCK_BREAK.is_empty() && AFTER_BLOCK_BREAK.is_empty() && BLOCK_BREAK_CANCELED.is_empty())
}

/// Whether anyone listens to `AFTER`.
pub(crate) fn after_block_break_heard() -> bool {
    !AFTER_BLOCK_BREAK.is_empty()
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
        assert_eq!(break_speed([0, 0, 0], block.clone(), 2.0, 2.0, None, state(true, false), DiggingFacts::default()), 2.0);
        assert_eq!(break_speed([0, 0, 0], block.clone(), 2.0, 2.0, None, state(false, false), DiggingFacts::default()), 10.0);
        assert_eq!(break_speed([0, 0, 0], block.clone(), 2.0, 2.0, None, state(true, true), DiggingFacts::default()), 3.0);
        assert_eq!(break_speed([0, 0, 0], block.clone(), 2.0, 2.0, None, None, DiggingFacts::default()), 2.0, "no player state leaves the speed alone");
        // The facts of the dig reach the callbacks as they were reported.
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        BreakSpeedCallback::EVENT.register(move |c| if c.pos == [9, 9, 9] { log.lock().unwrap().push((c.block_light, c.sky_light, c.sky_darken, c.biome_temperature)); });
        let facts = DiggingFacts { block_light: Some(3), sky_light: Some(15), sky_darken: 4, biome_temperature: Some(0.8) };
        break_speed([9, 9, 9], block, 2.0, 2.0, None, None, facts);
        assert_eq!(*seen.lock().unwrap(), vec![(Some(3), Some(15), 4, Some(0.8))]);
    }

    #[test]
    fn a_veto_before_a_block_break_cancels_it() {
        let canceled = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = canceled.clone();
        PlayerBlockBreakEvents::CANCELED.register(move |c| seen.lock().unwrap().push(c.pos));
        PlayerBlockBreakEvents::BEFORE.register(|c| c.pos[1] >= 0);
        PlayerBlockBreakEvents::BEFORE.register(|c| c.pos[1] >= -1);
        assert!(before_block_break(&BlockBreakContext::at([0, 64, 0])));
        assert!(canceled.lock().unwrap().is_empty(), "allowed breaks are not canceled");
        assert!(!before_block_break(&BlockBreakContext::at([0, -5, 0])));
        assert_eq!(*canceled.lock().unwrap(), [[0, -5, 0]], "two vetoes still report one cancellation");
    }

    #[test]
    fn an_after_callback_sees_the_player_and_the_worn_tool_and_queues_actions_for_the_game() {
        use super::super::entity_events_v1::{ActionTarget, StackSlot, StackDataEdit};
        use super::super::base::Nbt;
        AFTER_BLOCK_BREAK.register(|broken| {
            if broken.dimension != "test:break_after" { return; }
            let held = broken.held.as_ref().expect("the player holds the tool");
            assert_eq!((held.damage, broken.player.as_ref().map(|p| p.uuid)), (4, Some(9)));
            assert_eq!(broken.state.as_ref().map(|s| s.block.to_string()), Some("test:rock".to_owned()));
            broken.actions.borrow_mut().push(EntityAction::EditStackData {
                target: ActionTarget::Attacker, slot: StackSlot::MainHand, item: held.item.clone(),
                edits: vec![StackDataEdit { path: vec!["mined".into()], value: Some(Nbt::Int(1)) }],
            });
        });
        let context = BlockBreakContext {
            pos: [1, 2, 3], dimension: "test:break_after".into(),
            state: Some(BlockState { block: super::super::base::Identifier::parse("test:rock").unwrap(), properties: Default::default() }),
            player: Some(ServerPlayer { uuid: 9, name: "Alex".into() }),
            held: Some(StackData { item: "test:pick".into(), count: 1, damage: 4, ..Default::default() }),
            actions: RefCell::new(Vec::new()),
        };
        assert!(after_block_break_heard());
        after_block_break(&context);
        assert_eq!(context.actions.borrow().len(), 1);
        let quiet = BlockBreakContext { dimension: "test:elsewhere".into(), ..BlockBreakContext::at([0, 0, 0]) };
        after_block_break(&quiet);
        assert!(quiet.actions.borrow().is_empty(), "a callback that does not match queues nothing");
    }
}
