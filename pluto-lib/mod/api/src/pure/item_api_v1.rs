//! Counterpart of Fabric's `fabric-item-api-v1`: item tooltips/use and
//! Minecraft standing/wall block-item binding declarations (native binding stub).
//!
//! Fabric's `ItemTooltipCallback` lets callbacks read and edit the tooltip's lines. `with_lines` edits the current list while
//! preserving untouched native rich text; the original `added` list still appends simple lines after the editable list.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::base::{Event, Identifier};
pub use super::standing_wall_item::StandingAndWallBlockItem;
pub use super::item_registry::{ItemRegistry,ItemRegistrySnapshot};
pub use super::item_behavior::{EnchantabilityContext, EnderMaskContext, ItemBehavior, ItemBehaviors, MaxStackSizeContext, ToolActionContext};
pub(crate) use super::item_behavior::ender_mask;
pub(crate) use super::item_behavior::max_stack_size;
pub(crate) use super::item_behavior::enchantability;
pub(crate) use super::item_behavior::can_perform_action;
pub(crate) use super::standing_wall_item::standing_wall_bindings;

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

type StackCheck = std::sync::Arc<dyn Fn(&super::player_api::StackData) -> bool + Send + Sync>;
type StackTime = std::sync::Arc<dyn Fn(&super::player_api::StackData) -> i32 + Send + Sync>;
type StackFood = std::sync::Arc<dyn Fn(&super::player_api::StackData) -> Option<Food> + Send + Sync>;
type StackAction = std::sync::Arc<dyn Fn(&super::player_api::StackData) -> Option<UseAction> + Send + Sync>;

/// How one stack of an item is used over time, decided per stack (Fabric: `Item.getMaxUseTime(ItemStack)` and `Item.use` reading the stack's
/// components, which `ItemUse` cannot express because it answers for every stack of the item alike). Build it with [`StackUse::new`] and the
/// chained methods, then register it with [`ItemUseRegistry::register_stack`].
#[derive(Clone)]
pub struct StackUse {
    action: UseAction,
    action_for: Option<StackAction>,
    can_use: StackCheck,
    use_time: StackTime,
    food: Option<StackFood>,
    requires_hunger: bool,
}

impl std::fmt::Debug for StackUse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StackUse").field("action", &self.action).field("action_for", &self.action_for.is_some()).field("food", &self.food.is_some()).field("requires_hunger", &self.requires_hunger).finish()
    }
}

impl StackUse {
    /// A use in the given pose that every stack may start, for 0 ticks (so nothing happens until [`StackUse::use_time`] says otherwise), with no
    /// food.
    pub fn new(action: UseAction) -> Self {
        Self { action, action_for: None, can_use: std::sync::Arc::new(|_| true), use_time: std::sync::Arc::new(|_| 0), food: None, requires_hunger: false }
    }

    /// The same, with the pose chosen per stack (a tool that blocks with one modifier and is eaten with
    /// another): the closure answers the stack's pose, or None to leave the game its own for that stack.
    /// Only asked for a stack `can_use` accepts; without it, every accepted stack takes the fixed pose.
    pub fn action_for(mut self, action_for: impl Fn(&super::player_api::StackData) -> Option<UseAction> + Send + Sync + 'static) -> Self {
        self.action_for = Some(std::sync::Arc::new(action_for));
        self
    }

    /// The same, where `can_use` decides whether this registration applies to the stack (a broken tool, a tool without the stat that makes it
    /// edible). A stack it refuses is left to the game: its own use time, pose and rules (usually no use at all). It runs often (when a use
    /// starts and as the hand draws one in use): answer from the stack alone and cheaply.
    pub fn can_use(mut self, can_use: impl Fn(&super::player_api::StackData) -> bool + Send + Sync + 'static) -> Self {
        self.can_use = std::sync::Arc::new(can_use);
        self
    }

    /// The same, with the ticks this stack's use takes (Fabric: `getMaxUseTime(stack)`). Only asked for a stack `can_use` accepts; a time of 0
    /// or less means the stack is not used over time.
    pub fn use_time(mut self, use_time: impl Fn(&super::player_api::StackData) -> i32 + Send + Sync + 'static) -> Self {
        self.use_time = std::sync::Arc::new(use_time);
        self
    }

    /// The same, feeding what `food` answers for the stack when the use completes (None feeds nothing). The player is fed on the server once the
    /// use is done, before the finish event is called. Do not also register a plain [`ItemUse`] with food for the same item.
    pub fn food(mut self, food: impl Fn(&super::player_api::StackData) -> Option<Food> + Send + Sync + 'static) -> Self {
        self.food = Some(std::sync::Arc::new(food));
        self
    }

    /// The same, but a player who is not hungry (food level 20) cannot start using the stack unless they are in creative mode (Fabric: the
    /// `canEat(false)` check of a food item). Use it for a stack whose finish callback feeds the player itself.
    pub fn requires_hunger(mut self) -> Self {
        self.requires_hunger = true;
        self
    }

    /// The pose while using.
    pub(crate) fn action(&self) -> UseAction { self.action }
}

static STACK_USES: Mutex<Option<HashMap<String, StackUse>>> = Mutex::new(None);

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

    /// Makes `item` usable over time, decided per stack (see [`StackUse`]). Registering the same item again replaces the earlier entry. An item
    /// registers either this or [`ItemUseRegistry::register`], not both. The game side asks it where it asks the item's use time and pose, when a
    /// use starts, and when it finishes (see `UseItemFinishCallback`).
    pub fn register_stack(item: Identifier, behavior: StackUse) {
        if let Ok(mut all) = STACK_USES.lock() {
            all.get_or_insert_with(HashMap::new).insert(item.to_string(), behavior);
        }
    }

    /// The per-stack use registered for `item`, if any.
    pub fn stack_use(item: &str) -> Option<StackUse> {
        STACK_USES.lock().ok()?.as_ref()?.get(item).cloned()
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

/// Whether `item` has a per-stack use, so the game side reads the stack only for those items.
pub(crate) fn has_stack_use(item: &str) -> bool {
    STACK_USES.lock().ok().is_some_and(|all| all.as_ref().is_some_and(|all| all.contains_key(item)))
}

/// The pose of `stack` when its item has a per-stack use that accepts the stack; None leaves the game's own pose.
pub(crate) fn stack_use_action(stack: &super::player_api::StackData) -> Option<UseAction> {
    let behavior = ItemUseRegistry::stack_use(&stack.item)?;
    if !(behavior.can_use)(stack) { return None; }
    match &behavior.action_for {
        Some(action_for) => action_for(stack),
        None => Some(behavior.action()),
    }
}

/// The use time of `stack` when its item has a per-stack use that accepts the stack (never below 0); None leaves the game's own time.
pub(crate) fn stack_use_duration(stack: &super::player_api::StackData) -> Option<i32> {
    let behavior = ItemUseRegistry::stack_use(&stack.item)?;
    (behavior.can_use)(stack).then(|| (behavior.use_time)(stack).max(0))
}

/// Whether the player may start using `stack`: false only for a per-stack use that accepts the stack and needs hunger when the player has
/// none and is not in creative mode. True otherwise: a stack the use refuses is left to the game.
pub(crate) fn stack_use_may_start(stack: &super::player_api::StackData, hungry: bool, creative: bool) -> bool {
    let Some(behavior) = ItemUseRegistry::stack_use(&stack.item) else { return true };
    !((behavior.can_use)(stack) && behavior.requires_hunger && !hungry && !creative)
}

/// What completing the use of `stack` feeds, when its item has a per-stack use that feeds.
pub(crate) fn stack_use_food(stack: &super::player_api::StackData) -> Option<Food> {
    let behavior = ItemUseRegistry::stack_use(&stack.item)?;
    if !(behavior.can_use)(stack) { return None; }
    behavior.food.as_ref().and_then(|food| food(stack))
}

/// A player let go of an item they were using, before its use time ran out (vanilla's `Item.releaseUsing`, NeoForge's
/// `LivingEntityUseItemEvent` release half, the game's `native_release_using`, API 131): the charge-and-release half of a
/// use. A bow-like item shoots by the charge it gathered; a thrown tool leaves the hand. The stacks in both hands are
/// given, as a bow shot gives them, so a callback knows which hand's stack it is spending; spending it is an `edits`
/// entry, as an ammunition callback spends its source. Only a player's release is heard, and only for an item a
/// [`StackUse`] or [`ItemUse`] registration gave a use time: the game asks nothing for items it uses itself.
#[derive(Clone, Debug)]
pub struct ReleaseUsing {
    /// The stack that was being used (the one let go).
    pub stack: super::player_api::StackData,
    /// The player who let go.
    pub player: super::player_api::ServerPlayer,
    /// The dimension the player is in, such as `minecraft:overworld`; also available as [`ReleaseUsing::world`].
    pub dimension: String,
    /// Where the player stands (their feet).
    pub position: [f64; 3],
    /// Where the player's eyes are: where a thrown or shot projectile leaves from.
    pub eye: [f64; 3],
    /// The direction the player looks, as a unit vector: how a release aims what it throws or shoots.
    pub look: [f64; 3],
    /// How many ticks the use was held before the release: the charge. A use that ran its whole time out is finished,
    /// not released, and is not heard here.
    pub ticks_used: u32,
    /// The stacks in the player's main hand and off hand as the use ends; the used stack is one of them.
    pub main_hand: Option<super::player_api::StackData>,
    pub off_hand: Option<super::player_api::StackData>,
    /// Whether the release counts as a use of the item: set it (with [`ReleaseUsing::set_used`]) and the game starts
    /// the item's cooldown, as a bow's release does. Left alone, no cooldown starts.
    pub used: std::cell::Cell<bool>,
    /// Whole new stacks for places the player carries, which callbacks add to: how a callback spends the stack it
    /// threw. The host writes them after the callbacks ran.
    pub edits: RefCell<Vec<super::entity_events_v1::CarriedEdit>>,
}

impl ReleaseUsing {
    /// Mark the release as a use of the item, so its cooldown starts.
    pub fn set_used(&self) {
        self.used.set(true);
    }

    /// The world the player is in.
    pub fn world(&self) -> super::world_api_v1::World {
        super::world_api_v1::World::new(&self.dimension)
    }
}

static RELEASE_USING: Event<ReleaseUsing, ()> = Event::void();

/// Events about items.
pub struct ItemEvents;

impl ItemEvents {
    /// A player released an item they were using (see [`ReleaseUsing`]). Server only.
    pub const RELEASE_USING: &'static Event<ReleaseUsing, ()> = &RELEASE_USING;
}

/// Whether anyone listens to releases, so the host hook answers without building the payload.
pub(crate) fn release_heard() -> bool {
    !RELEASE_USING.is_empty()
}

/// A release the callbacks answered: whether it counts as a use, and the edits to the player's carried stacks.
pub(crate) fn released(release: &ReleaseUsing) -> (bool, Vec<super::entity_events_v1::CarriedEdit>) {
    RELEASE_USING.invoke(release);
    (release.used.get(), release.edits.borrow_mut().drain(..).collect())
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
    fn a_stack_use_decides_per_stack_whether_how_long_and_what_it_feeds() {
        // An item whose stacks are edible only when they carry a `hunger` value in their data.
        fn hunger(stack: &StackData) -> i32 {
            match stack.custom_data.get("hunger") { Some(crate::pure::base::Nbt::Int(value)) => *value, _ => 0 }
        }
        let stack = |value: i32| {
            let mut stack = StackData { item: "test:tool".into(), count: 1, ..Default::default() };
            if value != 0 { stack.custom_data.insert("hunger".into(), crate::pure::base::Nbt::Int(value)); }
            stack
        };
        assert!(!has_stack_use("test:tool"));
        assert_eq!(stack_use_duration(&stack(3)), None, "an item with no per-stack use is left to the game");
        assert!(stack_use_may_start(&stack(0), false, false));
        ItemUseRegistry::register_stack(
            Identifier::new("test", "tool").unwrap(),
            StackUse::new(UseAction::Eat)
                .can_use(|stack| hunger(stack) > 0)
                .use_time(|stack| 8 * hunger(stack))
                .food(|stack| Some(Food { nutrition: hunger(stack), saturation: 0.5 }))
                .requires_hunger(),
        );
        assert!(has_stack_use("test:tool"));
        assert_eq!((stack_use_action(&stack(1)), stack_use_action(&stack(0))), (Some(UseAction::Eat), None), "the pose follows the stack too");
        assert_eq!(use_animation("test:tool"), None, "the per-item pose is for plain registrations");
        assert_eq!((stack_use_duration(&stack(0)), stack_use_duration(&stack(1)), stack_use_duration(&stack(4))), (None, Some(8), Some(32)), "a refused stack keeps the game's time");
        assert_eq!(stack_use_food(&stack(0)), None, "a refused stack feeds nothing");
        assert_eq!(stack_use_food(&stack(3)), Some(Food { nutrition: 3, saturation: 0.5 }));
        assert!(stack_use_may_start(&stack(0), false, false), "a refused stack is left to the game, hungry or not");
        assert!(!stack_use_may_start(&stack(2), false, false), "no hunger, no eating");
        assert!(stack_use_may_start(&stack(2), true, false));
        assert!(stack_use_may_start(&stack(2), false, true), "creative players eat when full");
        ItemUseRegistry::register_stack(Identifier::new("test", "tool").unwrap(), StackUse::new(UseAction::Drink).use_time(|_| -5));
        assert_eq!(stack_use_duration(&stack(0)), Some(0), "an accepted stack with a negative time is no use at all");
        assert!(stack_use_may_start(&stack(0), false, false), "the default accepts every stack and needs no hunger");
        // A per-stack pose: the same item blocks with one stack and is eaten with another.
        ItemUseRegistry::register_stack(
            Identifier::new("test", "tool").unwrap(),
            StackUse::new(UseAction::Block)
                .can_use(|stack| hunger(stack) >= 0)
                .action_for(|stack| match hunger(stack) { 0 => None, 1 => Some(UseAction::Eat), _ => Some(UseAction::Block) }),
        );
        assert_eq!(stack_use_action(&stack(2)), Some(UseAction::Block));
        assert_eq!(stack_use_action(&stack(1)), Some(UseAction::Eat), "the selector's pose wins over the fixed one");
        assert_eq!(stack_use_action(&stack(0)), None, "a stack the selector declines keeps the game's pose");
    }

    #[test]
    fn a_release_reports_the_charge_and_collects_the_spend() {
        use super::super::entity_events_v1::{CarriedEdit, CarriedSlot, StackSlot};
        assert!(!release_heard());
        ItemEvents::RELEASE_USING.register(|release| {
            if release.stack.item != "test:thrown_tool" { return; }
            if release.ticks_used > 10 {
                release.set_used();
                // The tool leaves the main hand: the hand's place is written back with a count of 0, which empties it.
                let empty = super::super::block_entity_api_v1::ItemSlot::new(Identifier::parse("test:thrown_tool").unwrap(), 0);
                release.edits.borrow_mut().push(CarriedEdit { slot: CarriedSlot::Equipment(StackSlot::MainHand), stack: empty });
            }
        });
        assert!(release_heard());
        let tool = StackData { item: "test:thrown_tool".into(), count: 1, ..Default::default() };
        let release = |ticks_used: u32| ReleaseUsing {
            stack: tool.clone(),
            player: super::super::player_api::ServerPlayer { uuid: 7, name: "thrower".into() },
            dimension: "minecraft:overworld".into(),
            position: [0.5, 64.0, 0.5],
            eye: [0.5, 65.62, 0.5],
            look: [0.0, 0.0, 1.0],
            ticks_used,
            main_hand: Some(tool.clone()),
            off_hand: None,
            used: std::cell::Cell::new(false),
            edits: RefCell::new(Vec::new()),
        };
        let (used, edits) = released(&release(20));
        assert!(used, "a charged release counts as a use");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].stack.count, 0, "the thrown tool is spent from the hand");
        let (used, edits) = released(&release(5));
        assert!(!used, "a tap is no throw");
        assert!(edits.is_empty());
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
