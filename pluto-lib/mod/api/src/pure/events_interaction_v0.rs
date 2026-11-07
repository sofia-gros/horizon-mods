//! Counterpart of Fabric's `fabric-events-interaction-v0`: callbacks for what a player does with items, blocks and entities.
//!
//! Callback data is reduced and owned (Fabric passes player, world, hand and hit result). Results follow Fabric: the first
//! callback that does not return `ActionResult::Pass` decides; any other result stops the game's own handling.

use super::base::{ActionResult, Event};

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
