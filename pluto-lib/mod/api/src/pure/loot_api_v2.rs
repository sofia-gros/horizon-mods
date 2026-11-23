//! Counterpart of Fabric's `fabric-loot-api-v2`: change loot tables as they load.
//!
//! Fabric hands the callback a `LootTable.Builder`; Horizon's loot tables are JSON, so PlutoLib hands over the table's JSON and takes back
//! whatever the callbacks leave in it.

use std::cell::RefCell;

use super::base::{Event, Nbt, NbtCompound};
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
    /// The entity's own state (its age, its wool, its kind, the player it is), for drops whose output depends on the mob itself;
    /// None when there is no entity or the game did not report its state, and state-dependent drops then do not happen.
    pub entity_facts: Option<LootEntityFacts>,
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

/// The dropping entity's own state, for loot whose output depends on the entity itself (Minecraft: the entity
/// `LootContextParams.THIS_ENTITY` names, read as the severing recipes read it). Only the facts a drop can turn on are
/// carried; a fact the game did not report reads as its plain value (an adult, an unsheared sheep).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LootEntityFacts {
    /// A baby animal (`AgeableMob.isBaby`): its saved age is below 0.
    pub baby: bool,
    /// Sheared: a sheep's wool is gone; a snow golem's pumpkin is gone with it (the game keeps one flag for both).
    pub sheared: bool,
    /// A sheep's wool colour, as a dye id (0 white to 15 black); None when the entity is not a sheep or its colour was not reported.
    pub wool_color: Option<u8>,
    /// A mooshroom's kind, `red` or `brown`; None when the entity is not a mooshroom.
    pub mooshroom: Option<String>,
    /// The player the entity is, when it is one: their profile, for a head that keeps their skin.
    pub player: Option<PlayerProfile>,
}

/// A player's profile, as a player head keeps it (Minecraft: the game profile a head's `SkullOwner` is written from).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerProfile {
    /// The player's name, as the server's player list writes it.
    pub name: String,
    /// The player's id.
    pub uuid: u128,
}

/// The facts of an entity from its saved data (the game's `Entity.saveWithoutId` form, as the damage events read it): a
/// negative `Age` is a baby, a sheep's `Sheared` and `Color`, a snow golem's `Pumpkin` (kept as the opposite of sheared),
/// a mooshroom's `Type`. A fact the data does not carry reads as its plain value.
pub(crate) fn facts_from_saved(data: &NbtCompound, kind: &str) -> LootEntityFacts {
    let number = |key: &str| data.get(key).and_then(Nbt::as_f64);
    let mut facts = LootEntityFacts {
        baby: number("Age").is_some_and(|age| age < 0.0),
        sheared: number("Sheared").is_some_and(|value| value != 0.0)
            || number("sheared").is_some_and(|value| value != 0.0)
            || number("Pumpkin").is_some_and(|value| value == 0.0),
        ..LootEntityFacts::default()
    };
    if kind == "minecraft:sheep" {
        facts.wool_color = number("Color").map(|color| color as u8);
    }
    if kind == "minecraft:mooshroom" {
        facts.mooshroom = Some(if data.get("Type").and_then(Nbt::as_str) == Some("brown") { "brown".to_owned() } else { "red".to_owned() });
    }
    facts
}

/// A loot function of a mod's own kind being applied to one stack: a loot table entry named a
/// `modifier` whose `type` is not one of the game's, and the game hands the kind to the callbacks
/// registered for [`LootTableEvents::FUNCTION`] (the game's own kinds never arrive here). Callbacks
/// change `stack`; a callback that does not know `kind` leaves it alone.
pub struct LootFunctionContext {
    /// The function's kind, such as `mantle:fill_retextured_block`.
    pub kind: String,
    /// The function's own JSON (its fields beside `type`), for kinds that take settings.
    pub data: serde_json::Value,
    /// Where the table rolled: a broken block's centre, a dead mob's position; None for rolls the
    /// game makes away from a place.
    pub origin: Option<[f64; 3]>,
    /// The dimension the table rolled in, such as `minecraft:overworld`; None away from a place.
    pub dimension: Option<String>,
    /// The block entity the broken block had, saved with its id and position before the break
    /// (Minecraft: `LootContextParams.BLOCK_ENTITY`): a mod's block entity carries its own data in
    /// it, which is what functions like `mantle:fill_retextured_block` read. None when the block
    /// had no block entity or the roll is not a block's.
    pub block_entity: Option<NbtCompound>,
    /// The stack the function applies to. Change it in place.
    pub stack: RefCell<ItemSlot>,
}

static MODIFY: Event<LootTableContext, ()> = Event::void();
static MODIFY_DROPS: Event<LootDrops, ()> = Event::void();
static FUNCTION: Event<LootFunctionContext, ()> = Event::void();

/// Fabric: `LootTableEvents`.
pub struct LootTableEvents;

impl LootTableEvents {
    /// Fabric: `MODIFY`: runs for every loot table as it loads; edit `context.table`.
    pub const MODIFY: &'static Event<LootTableContext, ()> = &MODIFY;
    /// Fabric: `MODIFY_DROPS` (NeoForge: global loot modifiers): runs for the items a block, a mob or a player's use is about to drop, after
    /// the loot table rolled. Edit `context.drops`. Other breaks (leaves decaying, a piston) are not heard.
    pub const MODIFY_DROPS: &'static Event<LootDrops, ()> = &MODIFY_DROPS;
    /// A loot function of a mod's own kind, applied to one stack as its table rolls. Register a
    /// callback and answer only the `kind`s you own; every callback hears every kind.
    pub const FUNCTION: &'static Event<LootFunctionContext, ()> = &FUNCTION;
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

/// Whether anyone answers loot functions of a mod's own kind, so the game only converts stacks when someone does.
pub(crate) fn functions_heard() -> bool {
    !FUNCTION.is_empty()
}

/// Runs the `FUNCTION` callbacks over one stack and returns the stack they leave.
pub(crate) fn apply_function(context: &LootFunctionContext) -> ItemSlot {
    FUNCTION.invoke(context);
    context.stack.borrow().clone()
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
            entity: Some((5, kind.into())), entity_facts: None, player: Some(1), tool: None, explosion: None, used: false,
            drops: RefCell::new(vec![ItemSlot::new(Identifier::parse("example:rotten_flesh").unwrap(), 2), ItemSlot::new(Identifier::parse("example:bone").unwrap(), 1)]),
            actions: RefCell::new(Vec::new()),
        };
        let items = |slots: Vec<ItemSlot>| slots.iter().map(|slot| format!("{}x{}", slot.count, slot.item)).collect::<Vec<_>>();
        assert_eq!(items(modify_drops(&drops("example:beast"))), ["1xexample:bone", "1xexample:trophy"]);
        assert_eq!(items(modify_drops(&drops("example:other"))), ["2xexample:rotten_flesh", "1xexample:bone"], "another mob's drops are untouched");
        let rock = LootDrops {
            table: "example:blocks/rock".into(), dimension: "minecraft:overworld".into(), origin: [0.5; 3], block: Some([0, 0, 0]),
            entity: None, entity_facts: None, player: Some(1), tool: None, explosion: None, used: false,
            drops: RefCell::new(vec![ItemSlot::new(Identifier::parse("example:rock").unwrap(), 3)]),
            actions: RefCell::new(Vec::new()),
        };
        assert!(modify_drops(&rock).is_empty());
        assert_eq!(rock.actions.take(), [EntityAction::Heal { target: ActionTarget::Attacker, amount: 1.0 }]);
    }

    #[test]
    fn a_function_changes_only_the_stacks_of_its_own_kind() {
        use super::super::base::{Identifier, Nbt};
        assert!(!functions_heard());
        LootTableEvents::FUNCTION.register(|context| {
            if context.kind == "example:fill_marker" {
                if let Some(marker) = context.block_entity.as_ref().and_then(|data| data.get("marker")).and_then(|value| match value { Nbt::String(text) => Some(text.clone()), _ => None }) {
                    context.stack.borrow_mut().custom_data.insert("marker".to_owned(), Nbt::String(marker));
                }
            }
        });
        assert!(functions_heard());
        let mut block_entity = NbtCompound::new();
        block_entity.insert("marker".to_owned(), Nbt::String("example:stone".to_owned()));
        let context = |kind: &str| LootFunctionContext {
            kind: kind.into(), data: json!({}), origin: Some([0.5; 3]), dimension: Some("minecraft:overworld".into()),
            block_entity: Some(block_entity.clone()), stack: RefCell::new(ItemSlot::new(Identifier::parse("example:rock").unwrap(), 1)),
        };
        let filled = apply_function(&context("example:fill_marker"));
        assert_eq!(filled.custom_data.get("marker"), Some(&Nbt::String("example:stone".to_owned())));
        let untouched = apply_function(&context("example:other_kind"));
        assert!(untouched.custom_data.is_empty(), "another kind's function leaves the stack alone");
    }

    #[test]
    fn entity_facts_come_from_the_saved_data_of_the_dropping_mob() {
        use super::super::base::Nbt;
        let saved = |pairs: &[(&str, Nbt)]| pairs.iter().map(|(key, value)| (key.to_string(), value.clone())).collect::<NbtCompound>();
        // A baby: a negative age. An adult's age is 0 or above, and a mob that cannot be a baby saves no age at all.
        assert!(facts_from_saved(&saved(&[("Age", Nbt::Int(-20))]), "minecraft:chicken").baby);
        assert!(!facts_from_saved(&saved(&[("Age", Nbt::Int(0))]), "minecraft:chicken").baby);
        assert_eq!(facts_from_saved(&NbtCompound::new(), "minecraft:zombie"), LootEntityFacts::default(), "data that says nothing reports the plain facts");
        // A sheep: sheared, and its wool colour as a dye id (14 is red).
        let sheep = facts_from_saved(&saved(&[("Sheared", Nbt::Byte(1)), ("Color", Nbt::Byte(14))]), "minecraft:sheep");
        assert!(sheep.sheared && sheep.wool_color == Some(14));
        assert_eq!(facts_from_saved(&saved(&[("Color", Nbt::Byte(0))]), "minecraft:sheep").wool_color, Some(0), "white is a colour too");
        assert_eq!(facts_from_saved(&saved(&[("Color", Nbt::Byte(3))]), "minecraft:cow").wool_color, None, "only a sheep's colour is wool");
        // A snow golem's pumpkin is saved as the opposite of sheared.
        assert!(facts_from_saved(&saved(&[("Pumpkin", Nbt::Byte(0))]), "minecraft:snow_golem").sheared);
        assert!(!facts_from_saved(&saved(&[("Pumpkin", Nbt::Byte(1))]), "minecraft:snow_golem").sheared);
        // A mooshroom's kind: brown only when the data says so.
        assert_eq!(facts_from_saved(&saved(&[("Type", Nbt::String("brown".into()))]), "minecraft:mooshroom").mooshroom.as_deref(), Some("brown"));
        assert_eq!(facts_from_saved(&NbtCompound::new(), "minecraft:mooshroom").mooshroom.as_deref(), Some("red"));
        assert_eq!(facts_from_saved(&NbtCompound::new(), "minecraft:cow").mooshroom, None);
    }
}
