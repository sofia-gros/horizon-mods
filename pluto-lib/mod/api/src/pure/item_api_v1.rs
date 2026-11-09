//! Counterpart of Fabric's `fabric-item-api-v1` (so far: item tooltips).
//!
//! Fabric's `ItemTooltipCallback` lets a callback read and edit the tooltip's `List<Text>`. PlutoLib can only add lines for now: it cannot
//! show a callback the lines already there (see the support matrix).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::base::{Event, Identifier};

/// One tooltip line to add.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TooltipLine {
    pub text: String,
    /// The line colour as `0xRRGGBBAA`.
    pub color: u32,
}

impl TooltipLine {
    /// A grey line, like most tooltip text.
    pub fn grey(text: impl Into<String>) -> Self {
        Self { text: text.into(), color: 0xAAAA_AAFF }
    }
}

/// The tooltip being built for an item stack.
pub struct ItemTooltipContext {
    /// The item id, such as `tconstruct:flint_pickaxe`.
    pub item: String,
    pub count: u32,
    /// The stack the tooltip is for, with its damage, max damage and custom data.
    pub stack: super::player_api::StackData,
    /// Lines callbacks want added after the game's own.
    pub added: RefCell<Vec<TooltipLine>>,
}

static ITEM_TOOLTIP: Event<ItemTooltipContext, ()> = Event::void();

/// Fabric: `ItemTooltipCallback`.
pub struct ItemTooltipCallback;

impl ItemTooltipCallback {
    /// Fabric: `ItemTooltipCallback.EVENT`.
    pub const EVENT: &'static Event<ItemTooltipContext, ()> = &ITEM_TOOLTIP;
}

/// Runs the tooltip callbacks and returns the lines they added.
pub(crate) fn item_tooltip(stack: super::player_api::StackData) -> Vec<TooltipLine> {
    let context = ItemTooltipContext { item: stack.item.clone(), count: stack.count, stack, added: RefCell::new(Vec::new()) };
    ITEM_TOOLTIP.invoke(&context);
    context.added.into_inner()
}

/// How the player holds an item while using it over time (Fabric: `UseAction`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UseAction {
    /// No pose: the item is held still.
    None,
    /// Brought to the mouth, eating.
    Eat,
    /// Brought to the mouth, drinking.
    Drink,
    /// Raised like a shield.
    Block,
    /// Drawn like a bow.
    Bow,
    /// Raised like a trident.
    Trident,
    /// Loaded like a crossbow.
    Crossbow,
    /// Held to the eye like a spyglass.
    Spyglass,
    /// Blown like a goat horn.
    TootHorn,
    /// Swept like a brush.
    Brush,
    /// Held like a bundle.
    Bundle,
    /// Held like a spear.
    Spear,
}

/// What eating an item gives (Fabric: `FoodComponent`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Food {
    /// Hunger points restored; 2 are one drumstick.
    pub nutrition: i32,
    /// Saturation restored.
    pub saturation: f32,
}

/// An item that is used over time (Fabric: `Item.getMaxUseTime`, `Item.getUseAction`, `FoodComponent` and `useRemainder`).
#[derive(Clone, Debug, PartialEq)]
pub struct ItemUse {
    /// Ticks the use takes before it completes; 20 ticks are one second.
    pub use_time: i32,
    /// The pose while using.
    pub action: UseAction,
    /// What completing the use feeds, if anything. A feeding item cannot be started by a player who is not hungry unless `can_always_eat`.
    pub food: Option<Food>,
    /// A feeding item may be started at full hunger (Fabric: `FoodComponent.canAlwaysEat`).
    pub can_always_eat: bool,
    /// Handed back when one of the item is used up: an item id and a count (Fabric: `useRemainder`).
    pub remainder: Option<(Identifier, u32)>,
}

impl ItemUse {
    /// An item used for `use_time` ticks in the given pose, with no food and no remainder.
    pub fn new(use_time: i32, action: UseAction) -> Self {
        Self { use_time, action, food: None, can_always_eat: false, remainder: None }
    }

    /// The same, feeding `nutrition` hunger points and `saturation` saturation when completed.
    pub fn food(mut self, nutrition: i32, saturation: f32) -> Self {
        self.food = Some(Food { nutrition, saturation });
        self
    }

    /// The same, startable at full hunger.
    pub fn can_always_eat(mut self) -> Self {
        self.can_always_eat = true;
        self
    }

    /// The same, handing back `count` of `item` when one is used up.
    pub fn remainder(mut self, item: Identifier, count: u32) -> Self {
        self.remainder = Some((item, count));
        self
    }
}

static ITEM_USES: Mutex<Option<HashMap<String, ItemUse>>> = Mutex::new(None);
static SINK: OnceLock<fn(&str, &ItemUse)> = OnceLock::new();

/// Tells the game side about every registration, those already made and those to come. Called once by the game-facing half.
pub(crate) fn set_register_sink(sink: fn(&str, &ItemUse)) {
    if SINK.set(sink).is_ok() {
        if let Ok(all) = ITEM_USES.lock() {
            for (item, behavior) in all.iter().flatten() {
                sink(item, behavior);
            }
        }
    }
}

/// Registers how items are used over time (Fabric: overriding `getMaxUseTime` / `getUseAction` in an `Item` subclass).
pub struct ItemUseRegistry;

impl ItemUseRegistry {
    /// Makes `item` usable over time. Applies to every stack of that item; registering the same item again replaces the earlier entry.
    pub fn register(item: Identifier, behavior: ItemUse) {
        if let Some(sink) = SINK.get() {
            sink(&item.to_string(), &behavior);
        }
        if let Ok(mut all) = ITEM_USES.lock() {
            all.get_or_insert_with(HashMap::new).insert(item.to_string(), behavior);
        }
    }

    /// What was registered for `item` (an id such as `tconstruct:cheese_block`), if anything.
    pub fn get(item: &str) -> Option<ItemUse> {
        ITEM_USES.lock().ok()?.as_ref()?.get(item).cloned()
    }
}

/// The use time the game should use for `item`: the registered one, else the game's own.
pub(crate) fn use_duration(item: &str, game_default: i32) -> i32 {
    ItemUseRegistry::get(item).map_or(game_default, |behavior| behavior.use_time)
}

/// The pose for `item`: the registered one, else None (the game keeps its own).
pub(crate) fn use_animation(item: &str) -> Option<UseAction> {
    ItemUseRegistry::get(item).map(|behavior| behavior.action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::player_api::StackData;

    #[test]
    fn a_registered_item_gets_its_use_time_and_pose_and_others_keep_the_games() {
        ItemUseRegistry::register(Identifier::new("test", "wheel").unwrap(), ItemUse::new(32, UseAction::Eat));
        assert_eq!(use_duration("test:wheel", 0), 32);
        assert_eq!(use_animation("test:wheel"), Some(UseAction::Eat));
        assert_eq!(use_duration("test:other", 7), 7);
        assert_eq!(use_animation("test:other"), None);
        ItemUseRegistry::register(Identifier::new("test", "wheel").unwrap(), ItemUse::new(10, UseAction::Drink).food(4, 0.5));
        assert_eq!(ItemUseRegistry::get("test:wheel"), Some(ItemUse::new(10, UseAction::Drink).food(4, 0.5)));
    }

    #[test]
    fn callbacks_add_lines_in_registration_order_for_the_item_they_choose() {
        ItemTooltipCallback::EVENT.register(|c| {
            if c.item == "tconstruct:flint_pickaxe" {
                c.added.borrow_mut().push(TooltipLine::grey("Tinkers tool"));
            }
        });
        ItemTooltipCallback::EVENT.register(|c| c.added.borrow_mut().push(TooltipLine::grey(format!("x{}", c.count))));
        let lines = item_tooltip(StackData { item: "tconstruct:flint_pickaxe".into(), count: 1, ..Default::default() });
        assert_eq!(lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), ["Tinkers tool", "x1"]);
        assert_eq!(item_tooltip(StackData { item: "minecraft:stone".into(), count: 3, ..Default::default() }).len(), 1);
    }
}
