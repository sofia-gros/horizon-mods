//! Counterpart of Fabric's `fabric-item-api-v1` (so far: item tooltips).
//!
//! Fabric's `ItemTooltipCallback` lets callbacks read and edit the tooltip's lines. `with_lines` edits the current list while
//! preserving untouched native rich text; the original `added` list still appends simple lines after the editable list.

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
    /// True when `text` is a translation key, shown in the player's language.
    pub translate: bool,
}

impl TooltipLine {
    /// A grey line, like most tooltip text.
    pub fn grey(text: impl Into<String>) -> Self {
        Self { text: text.into(), color: 0xAAAA_AAFF, translate: false }
    }

    /// A grey line whose text is the translation `key` (from the mod's language files), shown in the player's language.
    pub fn translatable(key: impl Into<String>) -> Self {
        Self { text: key.into(), color: 0xAAAA_AAFF, translate: true }
    }
}

/// An editable tooltip entry. Cloning, moving or recoloring an existing entry preserves its native rich text.
#[derive(Clone)]
pub struct TooltipText {
    content: TooltipContent,
    color: u32,
}

#[derive(Clone)]
enum TooltipContent {
    Existing { plain: String, component: serde_json::Value, native: std::sync::Arc<dyn std::any::Any + Send + Sync> },
    Simple(TooltipLine),
}

impl TooltipText {
    /// A literal grey entry; replacing a rich entry with this deliberately replaces its formatting too.
    pub fn literal(text: impl Into<String>) -> Self {
        Self::from_line(TooltipLine::grey(text))
    }
    /// A grey translated entry, resolved in the client's language.
    pub fn translatable(key: impl Into<String>) -> Self {
        Self::from_line(TooltipLine::translatable(key))
    }
    /// Visible text, without styling. Existing entries use the native text's resolved snapshot.
    pub fn plain_text(&self) -> String {
        match &self.content {
            TooltipContent::Existing { plain, .. } => plain.clone(),
            TooltipContent::Simple(line) if line.translate => super::i18n_api::I18n::translate(&line.text),
            TooltipContent::Simple(line) => line.text.clone(),
        }
    }
    /// The component's JSON form for inspection; changing the returned copy does not edit this entry.
    pub fn component(&self) -> serde_json::Value {
        match &self.content {
            TooltipContent::Existing { component, .. } => component.clone(),
            TooltipContent::Simple(line) if line.translate => serde_json::json!({ "translate": line.text }),
            TooltipContent::Simple(line) => serde_json::json!({ "text": line.text }),
        }
    }
    /// The base line tint, 0xRRGGBBAA; embedded rich-text colors still take precedence.
    pub fn color(&self) -> u32 { self.color }
    /// Change only the base tint, keeping rich text intact.
    pub fn with_color(mut self, color: u32) -> Self { self.color = color; self }

    pub(crate) fn from_line(line: TooltipLine) -> Self {
        Self { color: line.color, content: TooltipContent::Simple(line) }
    }
    pub(crate) fn existing<T: std::any::Any + Send + Sync>(plain: String, component: serde_json::Value, color: u32, native: T) -> Self {
        Self { color, content: TooltipContent::Existing { plain, component, native: std::sync::Arc::new(native) } }
    }
    pub(crate) fn native<T: std::any::Any>(&self) -> Option<&T> {
        match &self.content { TooltipContent::Existing { native, .. } => native.downcast_ref(), _ => None }
    }
    pub(crate) fn simple(&self) -> Option<&TooltipLine> {
        match &self.content { TooltipContent::Simple(line) => Some(line), _ => None }
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

struct TooltipScope {
    context: *const ItemTooltipContext,
    lines: std::rc::Rc<RefCell<Vec<TooltipText>>>,
}
thread_local! {
    static TOOLTIP_SCOPES: RefCell<Vec<TooltipScope>> = const { RefCell::new(Vec::new()) };
}
struct RestoreTooltip;
impl Drop for RestoreTooltip {
    fn drop(&mut self) { TOOLTIP_SCOPES.with(|scopes| { scopes.borrow_mut().pop(); }); }
}
impl ItemTooltipContext {
    /// Edit this callback's current lines. None outside its dispatch scope or while the same list is already borrowed.
    /// Existing `added` lines are appended after all callbacks, preserving the original API's behavior.
    pub fn with_lines<R>(&self, edit: impl FnOnce(&mut Vec<TooltipText>) -> R) -> Option<R> {
        let lines = TOOLTIP_SCOPES.with(|scopes| scopes.borrow().iter().rev()
            .find(|scope| std::ptr::eq(scope.context, self)).map(|scope| scope.lines.clone()))?;
        let mut lines = lines.try_borrow_mut().ok()?;
        Some(edit(&mut lines))
    }
}

/// Runs the callbacks against native snapshots, then appends the legacy simple additions.
pub(crate) fn item_tooltip(stack: super::player_api::StackData, lines: Vec<TooltipText>) -> Vec<TooltipText> {
    let context = ItemTooltipContext { item: stack.item.clone(), count: stack.count, stack, added: RefCell::new(Vec::new()) };
    let lines = std::rc::Rc::new(RefCell::new(lines));
    TOOLTIP_SCOPES.with(|scopes| scopes.borrow_mut().push(TooltipScope { context: &context, lines: lines.clone() }));
    let restore = RestoreTooltip;
    ITEM_TOOLTIP.invoke(&context);
    drop(restore);
    let mut result = std::mem::take(&mut *lines.borrow_mut());
    result.extend(context.added.into_inner().into_iter().map(TooltipText::from_line));
    result
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
        let lines = item_tooltip(StackData { item: "tconstruct:flint_pickaxe".into(), count: 1, ..Default::default() }, vec![]);
        assert_eq!(lines.iter().map(TooltipText::plain_text).collect::<Vec<_>>(), ["Tinkers tool", "x1"]);
        assert_eq!(item_tooltip(StackData { item: "minecraft:stone".into(), count: 3, ..Default::default() }, vec![]).len(), 1);

        ItemTooltipCallback::EVENT.register(|c| {
            if c.item == "test:editable" {
                c.with_lines(|lines| {
                    assert_eq!(lines[0].plain_text(), "Native title");
                    // A nested tooltip may run while the outer list is borrowed.
                    let nested = item_tooltip(StackData { item: "test:nested".into(), count: 2, ..Default::default() }, vec![]);
                    assert_eq!(nested[0].plain_text(), "x2");
                    assert!(c.with_lines(|_| ()).is_none(), "same-list reentry is rejected");
                    lines.remove(2);
                    lines.swap(0, 1);
                    lines[1] = TooltipText::literal("Edited title").with_color(0x55FF55FF);
                }).unwrap();
                c.added.borrow_mut().push(TooltipLine::grey("Legacy addition"));
            }
        });
        let native = std::sync::Arc::new(vec![1u8, 2, 3]);
        let rich = TooltipText::existing("Rich detail".into(), serde_json::json!({"text":"Rich detail", "bold":true}), 0xAABBCCFF, native.clone());
        let lines = item_tooltip(StackData { item: "test:editable".into(), count: 1, ..Default::default() }, vec![TooltipText::literal("Native title"), rich, TooltipText::literal("Remove me")]);
        assert_eq!(lines.iter().map(TooltipText::plain_text).collect::<Vec<_>>(), ["Rich detail", "Edited title", "x1", "Legacy addition"]);
        assert!(std::sync::Arc::ptr_eq(lines[0].native::<std::sync::Arc<Vec<u8>>>().unwrap(), &native), "rich native payload survives without reconstruction");
        assert_eq!(lines[0].component()["bold"], true);
        assert_eq!(lines[1].color(), 0x55FF55FF);
        super::super::i18n_api::Translations::register("en_us", [("test.tooltip.translation".into(), "Translated detail".into())]);
        let translated = TooltipText::translatable("test.tooltip.translation");
        assert_eq!(translated.plain_text(), "Translated detail");
        assert_eq!(translated.component()["translate"], "test.tooltip.translation");
        let outside = ItemTooltipContext { item: "test:outside".into(), count: 1, stack: StackData::default(), added: RefCell::new(vec![]) };
        assert!(outside.with_lines(|_| ()).is_none());
        ItemTooltipCallback::EVENT.register(|c| { if c.item == "test:panic" { panic!("expected tooltip unwind"); } });
        let failed = std::panic::catch_unwind(|| item_tooltip(StackData { item: "test:panic".into(), ..Default::default() }, vec![]));
        assert!(failed.is_err());
        TOOLTIP_SCOPES.with(|scopes| assert!(scopes.borrow().is_empty(), "unwind restores the scope"));
    }
}
