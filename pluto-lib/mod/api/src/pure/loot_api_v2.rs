//! Counterpart of Fabric's `fabric-loot-api-v2`: change loot tables as they load.
//!
//! Fabric hands the callback a `LootTable.Builder`; Horizon's loot tables are JSON, so PlutoLib hands over the table's JSON and takes back
//! whatever the callbacks leave in it.

use std::cell::RefCell;

use super::base::Event;
use super::block_entity_api_v1::{BlockPos, ItemSlot};
use super::entity_events_v1::EntityAction;
use super::player_api::StackData;

/// A loot table being loaded. Callbacks edit `table` in place.
pub struct LootTableContext {
    /// The table's id, such as `minecraft:blocks/stone`.
    pub id: String,
    pub table: RefCell<serde_json::Value>,
}

/// The items a broken block, a dead mob or a player's use is about to drop, heard on a server before they drop. Callbacks change `drops`.
pub struct LootDrops {
    /// The block's or mob's own loot table (`minecraft:blocks/stone`, `minecraft:entities/zombie`); empty when none rolled.
    pub table: String,
    /// The dimension they drop in, such as `minecraft:overworld`.
    pub dimension: String,
    /// Where they drop: a broken block's centre, a mob's position.
    pub origin: [f64; 3],
    /// The block that broke, when a block did.
    pub block: Option<BlockPos>,
    /// The mob that died, or the entity a player's use dropped them from (the sheep sheared): its entity id and type id.
    pub entity: Option<(u32, String)>,
    /// The player who broke the block or killed the mob, when the game knows them (a mob remembers a player's hit for 100 ticks).
    pub player: Option<u128>,
    /// What broke the block, or what the player used; None when the game reports none, as it does for a mob's death.
    pub tool: Option<StackData>,
    /// The radius of the explosion that broke the block, when one did.
    pub explosion: Option<f32>,
    /// True when a player's use dropped them (shears, a brush, an item frame) rather than a break or a death.
    pub used: bool,
    /// The stacks about to drop, in the game's order. Add, change or remove stacks; stacks left with a count of 0 do not drop.
    pub drops: RefCell<Vec<ItemSlot>>,
    /// What to do to the player (`ActionTarget::Attacker`) who broke the block or used the tool, which callbacks add to; the game does it
    /// after the callbacks ran. `EditStackData` acts on the stack the player holds then (a tool that gathers what it mines keeps it here).
    /// Dropped when no player is known.
    pub actions: RefCell<Vec<EntityAction>>,
}

static MODIFY: Event<LootTableContext, ()> = Event::void();
static MODIFY_DROPS: Event<LootDrops, ()> = Event::void();

/// Fabric: `LootTableEvents`.
pub struct LootTableEvents;

impl LootTableEvents {
    /// Fabric: `MODIFY`: runs for every loot table as it loads; edit `context.table`.
    pub const MODIFY: &'static Event<LootTableContext, ()> = &MODIFY;
    /// Fabric: `MODIFY_DROPS` (NeoForge: global loot modifiers): runs for the items a block, a mob or a player's use is about to drop, after
    /// the loot table rolled. Edit `context.drops`. Other breaks (leaves decaying, a piston) are not heard.
    pub const MODIFY_DROPS: &'static Event<LootDrops, ()> = &MODIFY_DROPS;
}

/// Whether anyone listens to the drops, so the game only converts stacks when someone does.
pub(crate) fn drops_heard() -> bool {
    !MODIFY_DROPS.is_empty()
}

/// Runs the `MODIFY_DROPS` callbacks and returns the stacks they leave.
pub(crate) fn modify_drops(context: &LootDrops) -> Vec<ItemSlot> {
    MODIFY_DROPS.invoke(context);
    context.drops.borrow_mut().drain(..).collect()
}

/// Runs the `MODIFY` callbacks over `table` and returns the (possibly edited) table.
pub(crate) fn modify(id: &str, table: serde_json::Value) -> serde_json::Value {
    let context = LootTableContext { id: id.to_owned(), table: RefCell::new(table) };
    MODIFY.invoke(&context);
    context.table.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_callback_edit_comes_back_and_only_for_the_tables_it_targets() {
        LootTableEvents::MODIFY.register(|c| {
            if c.id == "minecraft:blocks/stone" {
                c.table.borrow_mut()["pools"] = json!([{"rolls": 1}]);
            }
        });
        let edited = modify("minecraft:blocks/stone", json!({"type": "minecraft:block"}));
        assert_eq!(edited["pools"][0]["rolls"], 1);
        let untouched = modify("minecraft:blocks/dirt", json!({"type": "minecraft:block"}));
        assert!(untouched.get("pools").is_none());
    }

    #[test]
    fn drops_are_changed_in_place_by_the_callbacks_that_know_the_mob() {
        use super::super::base::Identifier;
        use super::super::entity_events_v1::{ActionTarget, EntityAction};
        assert!(!drops_heard());
        // A callback may queue player actions beside the changed drops.
        LootTableEvents::MODIFY_DROPS.register(|context| {
            if context.table == "example:blocks/rock" {
                context.drops.borrow_mut().clear();
                context.actions.borrow_mut().push(EntityAction::Heal { target: ActionTarget::Attacker, amount: 1.0 });
            }
        });
        LootTableEvents::MODIFY_DROPS.register(|context| {
            if context.entity.as_ref().is_some_and(|(_, kind)| kind == "example:beast") && !context.used {
                let mut drops = context.drops.borrow_mut();
                drops.retain(|slot| slot.item.to_string() != "example:rotten_flesh");
                drops.push(ItemSlot::new(Identifier::parse("example:trophy").unwrap(), 1));
            }
        });
        assert!(drops_heard());
        let drops = |kind: &str| LootDrops {
            table: "example:entities/beast".into(), dimension: "minecraft:overworld".into(), origin: [0.0; 3], block: None,
            entity: Some((5, kind.into())), player: Some(1), tool: None, explosion: None, used: false,
            drops: RefCell::new(vec![ItemSlot::new(Identifier::parse("example:rotten_flesh").unwrap(), 2), ItemSlot::new(Identifier::parse("example:bone").unwrap(), 1)]),
            actions: RefCell::new(Vec::new()),
        };
        let items = |slots: Vec<ItemSlot>| slots.iter().map(|slot| format!("{}x{}", slot.count, slot.item)).collect::<Vec<_>>();
        assert_eq!(items(modify_drops(&drops("example:beast"))), ["1xexample:bone", "1xexample:trophy"]);
        assert_eq!(items(modify_drops(&drops("example:other"))), ["2xexample:rotten_flesh", "1xexample:bone"], "another mob's drops are untouched");
        let rock = LootDrops {
            table: "example:blocks/rock".into(), dimension: "minecraft:overworld".into(), origin: [0.5; 3], block: Some([0, 0, 0]),
            entity: None, player: Some(1), tool: None, explosion: None, used: false,
            drops: RefCell::new(vec![ItemSlot::new(Identifier::parse("example:rock").unwrap(), 3)]),
            actions: RefCell::new(Vec::new()),
        };
        assert!(modify_drops(&rock).is_empty());
        assert_eq!(rock.actions.take(), [EntityAction::Heal { target: ActionTarget::Attacker, amount: 1.0 }]);
    }
}
