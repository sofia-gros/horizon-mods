# Items: tooltips and behavior

Read, reorder, remove or replace an item's existing tooltip lines with `with_lines`, or keep using `added` to append simple lines. Untouched or moved existing entries retain their native rich text rather than being flattened and rebuilt.

## Basic Example

Add a line to an item's tooltip:

```rust
use pluto_lib::{ItemTooltipCallback, TooltipLine};

ItemTooltipCallback::EVENT.register(|tip| {
    if tip.item == "minecraft:diamond" {
        tip.added.borrow_mut().push(TooltipLine::grey("Shiny"));
    }
});
```

Lines you add come after the game's own. The callback runs whenever a tooltip is built, so keep it short.

## Keeping Item Defaults from a Reload

`ItemRegistry::snapshot` captures the current server JSON reload's item prototypes. Retain the owned snapshot alongside data parsed during that callback, rather than querying another world's item registry when the data is used later. `default_stack` returns a fresh copy with count one, zero damage, prototype durability and custom NBT. `crafting_remainder` reads the same generation's item-level crafting remainder, independently of consumption/use remainders.

```rust
use pluto_lib::{Identifier, ItemRegistry, ResourceManagerHelper, ResourceType};
ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
    Identifier::parse("example:item_defaults").unwrap(), "example/rules", |_| {
        let items = ItemRegistry::snapshot().ok_or("item prototypes unavailable")?;
        let id = Identifier::parse("minecraft:iron_pickaxe").unwrap();
        if let Some(mut stack) = items.default_stack(&id) {
            stack.count = 2; // only this copy changes
            assert_eq!(items.default_stack(&id).unwrap().count, 1);
        }
        Ok(())
    }).unwrap();
```

> **Use with Caution.** The source adapter captures the world's selected native Items before folder callbacks, not a builtin catalog or later live lookup. This is pure-tested with full attachment planning/application, not yet native build/game validated. Outside supported server callbacks, or if the whole prototype capture fails validation, `snapshot` returns `None`; there is no fabricated empty/builtin fallback. Duplicate, unknown or missing prototypes invalidate that whole snapshot. Nested callbacks restore outer snapshots, including unwinding; retained snapshots survive reload/teardown. `StackData` is a reduced view: it carries count/damage/max damage/custom NBT, not every native item component. Typed custom NBT is transported as network NBT; unsupported strings/tags, trailing bytes and malformed compounds reject rather than silently truncating the prototype. The pinned loader drops known `minecraft:custom_data` values from `ItemInfo`. PlutoLib now source-captures the final owned `items` map and the loader's already-held `raw` Arc, retaining those values before publication without rereading files. Earlier map insert/borrow accesses must not be selected. The direct Horizon adapter now captures that map at the final Items initializer instead of a Mixins LOAD; full attachment planning/application has passed, while native typechecking and native NBT conversion remain unvalidated; missing or malformed data still invalidates the whole snapshot instead of becoming empty NBT. This does not establish full native-component or rich ItemOutput-codec parity.

## Item extension behavior

Use `ItemBehavior` for Minecraft item-extension queries independently of use-over-time behavior. An ender-mask callback returns `Some(true)` to hide the wearer from Enderman gaze, `Some(false)` to explicitly disable masking, or `None` to retain the native answer (including carved pumpkins).

```rust
use pluto_lib::{Identifier, ItemBehavior, ItemBehaviors};
ItemBehaviors::register(ItemBehavior::new(Identifier::parse("example:visor").unwrap())
    .is_ender_mask(|_| Some(true))).unwrap();
```

> **Use with Caution.** Native attachment is source-declared, not plan/build/game validated. The current AI snapshot has an item ID, wearer UUID and observer entity ID, but not the full stack: `stack` is `None`. Do not invent stack data; return `None` if a decision requires unavailable data. This adapter affects Enderman gaze, not visibility distance, revenge, or unrelated species. Callbacks run outside registry locks; duplicates fail without replacement.

## Standing and Wall Block Items

A standing-and-wall item chooses between two blocks while retaining the native placement path. Register the item, standing block, wall block and attachment direction during mod initialization. This describes a Minecraft `StandingAndWallBlockItem`; it is not a callback returning a different block from `state_for_placement`.

```rust
use pluto_lib::{Direction, Identifier, StandingAndWallBlockItem};
StandingAndWallBlockItem::register(
    Identifier::new("example", "head").unwrap(),
    Identifier::new("example", "head").unwrap(),
    Identifier::new("example", "wall_head").unwrap(),
    Direction::Down,
).unwrap();
```

> **Use with Caution.** This is an explicit native stub: registration records owned declarations, but does not yet change gameplay or declare any content. The existing host has the native standing/wall algorithm; its content wall-pair setting currently accepts signs only, and its block-state callback forbids cross-block switching. A generic binding adapter must validate actual item/block IDs and cover cached and uncached resource tables while preserving native checks and item consumption. The earlier per-owner cache/state draft is superseded: the generic native constructor cannot receive a state field under the pinned Mixins validator. The declaration provider is now installed in source and its transport module is source-pinned. The owned-table overlay and cache-suppression helpers remain undeclared: the proposed `cached` STORE initializer is `build()?`, and the pinned Mixins validator rejects `?` inside that selected value. This is a source-review result, not an executed plan diagnostic. Do not enable the cache hooks alone. A symbolic export alternative is now source-wired: validate all declared items/blocks in the completed behaviour JSON, preserve native metadata and rewrite names before numeric table loading. A SHA-256 identity of the sorted declarations selects a separate content-root path, including on cache hits. Empty batches preserve native behavior; invalid batches abort the new root without publishing a prefix. The writer retains native unlink-before-write semantics so base resources are not changed through links. With bindings but no content overlay, initialization rejects rather than writing to the base root. Native hook/site validation, compilation and placement remain pending under the native attachment guard; no runtime behavior is verified. Overlay/policy tests do not prove native binding or placement. A failed batch would refuse all declared items, while provider failure before their IDs are known would refuse all block placement; these are explicit failures, never a vanilla-wall fallback. No raw block-write or vanilla-wall fallback is substituted. See the support matrix.

## Using Items Over Time

Food, drinks and bows take time to use. Register how long, the pose and, for food, what it feeds and what it leaves behind:

```rust
use pluto_lib::{Identifier, ItemUse, ItemUseRegistry, UseAction};

ItemUseRegistry::register(
    Identifier::new("example", "stew").unwrap(),
    ItemUse::new(32, UseAction::Eat)
        .food(6, 0.6)
        .remainder(Identifier::new("minecraft", "bowl").unwrap(), 1),
);
```

A player who is not hungry cannot start a feeding item unless you add `.can_always_eat()`. When one is used up, the remainder is handed back (the use is refused if it would not fit).

### Deciding per stack

`ItemUse` answers for every stack of an item alike. When the answer depends on the stack (a tool that is edible only while it carries a hunger value, with an eating time of its own), register a `StackUse` instead. The game asks its callbacks with the stack's damage, max damage and custom data; a stack `can_use` refuses is left to the game, with its own use time, pose and rules.

```rust
use pluto_lib::{Food, Identifier, ItemUseRegistry, StackUse, UseAction};

ItemUseRegistry::register_stack(
    Identifier::new("example", "lunchbox").unwrap(),
    StackUse::new(UseAction::Eat)
        .can_use(|stack| stack.custom_data.contains_key("snacks"))
        .use_time(|stack| if stack.damage > 10 { 64 } else { 32 })
        .food(|_| Some(Food { nutrition: 2, saturation: 0.4 }))
        .requires_hunger(),
);
```

`.food` feeds the player when the use completes, before `UseItemFinishCallback` is called; a callback that wants the player's amount to depend on the player (or to wear the stack) leaves `.food` out, feeds with `done.player.add_food`, and changes the hand through `done.replacement`. Register an item with `register` or `register_stack`, not both.

> **Use with Caution.** `with_lines` is available only while this context is being dispatched by the game. It returns `None` for a manually invoked event or if the same list is already borrowed. Nested tooltips have separate lists. Legacy `added` entries are appended after all callbacks; they are not part of the editable list during dispatch.

## Reference

### `ItemRegistry`

Access to owned registered-item prototype snapshots during server JSON reload callbacks. It has no instance constructor and does not read another world's defaults outside that scope.

```rust
use pluto_lib::ItemRegistry;
assert!(ItemRegistry::snapshot().is_none()); // outside a server callback
```

### `ItemRegistry::snapshot`

`fn snapshot() -> Option<ItemRegistrySnapshot>`. Returns a cloneable immutable generation or `None` when capture/scope is unavailable. A captured empty item registry is a present snapshot with no items.

```rust
use pluto_lib::{ItemRegistry, ItemRegistrySnapshot};
let retained: Option<ItemRegistrySnapshot> = ItemRegistry::snapshot();
assert_eq!(retained.clone(), retained);
```

### `ItemRegistrySnapshot`

Owned, cloneable item defaults in `StackData` form. Clones share immutable storage; keeping one never refreshes it. Constructed only by the backend, not from guessed item names.

```rust
use pluto_lib::ItemRegistrySnapshot;
let not_captured: Option<ItemRegistrySnapshot> = None;
assert!(not_captured.is_none());
```

### `ItemRegistrySnapshot::default_stack`

`fn default_stack(&self, item: &Identifier) -> Option<StackData>`. Returns an owned prototype copy or `None` for an unknown item. Its damage is zero and count is one; maximum damage and typed custom NBT come from this generation's item definition. Other native components are outside this reduced representation.

```rust
use pluto_lib::{Identifier, ItemRegistry};
if let Some(items) = ItemRegistry::snapshot() {
    let unknown = Identifier::parse("example:missing").unwrap();
    let _stack = items.default_stack(&unknown);
}
```

### `ItemRegistrySnapshot::crafting_remainder`

`fn crafting_remainder(&self, item: &Identifier) -> Result<Option<Identifier>, String>`. Returns `Ok(Some(id))` when this item's captured crafting default leaves another item, or `Ok(None)` when a known item has no crafting remainder. An unknown item or unavailable property capture returns `Err`; these cases are not negative answers. The returned identifier is owned. Retained snapshots never consult a newer registry.

```rust
use pluto_lib::{Identifier, ItemRegistry};
if let Some(items) = ItemRegistry::snapshot() {
    let id = Identifier::parse("minecraft:water_bucket").unwrap();
    match items.crafting_remainder(&id) {
        Ok(Some(remainder)) => { let _owned_id: Identifier = remainder; }
        Ok(None) => { /* known item, no crafting remainder */ }
        Err(error) => { let _unavailable_or_unknown: String = error; }
    }
}
```

> **Use with Caution.** The source adapter captures `ItemInfo.crafting_remainder`, not `use_remainder` or an item-name/tag guess. Each captured item has an explicit present/absent property; duplicate, incomplete, unknown-item or unknown-target records reject the whole property snapshot. This is an item-level default, not a per-stack crafting callback. Source transport is declared on the existing reload path; native types, attachment and gameplay remain unverified under the native attachment gate.

### `StandingAndWallBlockItem`

Registers standing/wall placement metadata for an already declared item; it neither creates content nor copies rarity/components. Native application is currently a stub, so success means only that the declaration was accepted.

```rust
use pluto_lib::{Direction, Identifier, StandingAndWallBlockItem};
StandingAndWallBlockItem::register(
    Identifier::parse("example:torch").unwrap(),
    Identifier::parse("example:torch").unwrap(),
    Identifier::parse("example:wall_torch").unwrap(),
    Direction::Down,
).unwrap();
```

### `StandingAndWallBlockItem::register`

`register(item: Identifier, standing: Identifier, wall: Identifier, attachment: Direction) -> Result<(), String>` owns the declaration. The original constructor permits identical standing/wall block IDs, so this API does too. Duplicate items, a poisoned registry or registration after native binding collection return an error without replacing an earlier declaration. Collection freezes registration and supplies independent snapshots for resource-table resolution; there is no current native consumer. All six attachment directions are representable: the opposite is skipped, the attachment direction selects standing, and the others select the wall candidate. Ordinary heads/torches/signs use `Down`. A hanging sign's specialized native placement rules are not implied by choosing `Up` here.

```rust
use pluto_lib::{Direction, Identifier, StandingAndWallBlockItem};
let item = Identifier::parse("example:display").unwrap();
let floor = Identifier::parse("example:standing_display").unwrap();
let wall = Identifier::parse("example:wall_display").unwrap();
StandingAndWallBlockItem::register(item.clone(), floor.clone(), wall.clone(), Direction::Down).unwrap();
assert!(StandingAndWallBlockItem::register(item, floor.clone(), wall, Direction::Up).is_err());
StandingAndWallBlockItem::register(Identifier::parse("example:same").unwrap(), floor.clone(), floor, Direction::Down).unwrap();
```

### `TooltipLine`

One tooltip line to add. `text` is the line (or a translation key when `translate` is true), `color` is `0xRRGGBBAA`.

```rust
use pluto_lib::TooltipLine;

let gold = TooltipLine { text: "Rare".to_owned(), color: 0xFFAA00FF, translate: false };
assert_eq!(gold.color >> 24, 0xFF);
```

### `TooltipLine::translatable`

`fn translatable(key: impl Into<String>) -> TooltipLine`. A grey line whose text is the translation `key` from your mod's language files, shown in the player's language.

```rust
use pluto_lib::TooltipLine;

let line = TooltipLine::translatable("item.example.cheese.tooltip");
assert!(line.translate);
assert_eq!(line.text, "item.example.cheese.tooltip");
```

### `TooltipLine::grey`

`fn grey(text: impl Into<String>) -> TooltipLine`. A grey line, like most tooltip text.

```rust
use pluto_lib::TooltipLine;

assert_eq!(TooltipLine::grey("Hello").text, "Hello");
```

### `ItemTooltipContext`

The tooltip being built for an item stack. `item` is the item id (`tconstruct:flint_pickaxe`), `count` the stack size, `stack` the stack itself (damage, max damage, custom data) and `added` the lines callbacks want after the game's own (push onto it with `borrow_mut`).

```rust
use pluto_lib::{ItemTooltipCallback, ItemTooltipContext, StackData, TooltipLine};
use std::cell::RefCell;

ItemTooltipCallback::EVENT.register(|tip: &ItemTooltipContext| {
    if tip.count > 1 {
        tip.added.borrow_mut().push(TooltipLine::grey(format!("x{}", tip.count)));
    }
});
let tip = ItemTooltipContext { item: "minecraft:stick".into(), count: 8, stack: StackData { item: "minecraft:stick".into(), count: 8, ..Default::default() }, added: RefCell::new(Vec::new()) };
ItemTooltipCallback::EVENT.invoke(&tip);
assert_eq!(tip.added.borrow()[0].text, "x8");
```

### `ItemTooltipContext::with_lines`

`fn with_lines<R>(&self, edit: impl FnOnce(&mut Vec<TooltipText>) -> R) -> Option<R>`. Read or mutate the current list in place; `Some(result)` contains the closure's result. Callbacks see earlier edits in event order. Remove, reorder or clone existing entries without losing their styling. To change their text, replace them with a new literal or translated entry; that deliberately replaces the old content/style. The public context fields and `added` behavior are unchanged.

```rust
use pluto_lib::{ItemTooltipCallback, TooltipText};
ItemTooltipCallback::EVENT.register(|tip| {
    if tip.item == "minecraft:diamond" {
        tip.with_lines(|lines| {
            if let Some(first) = lines.first_mut() {
                *first = TooltipText::literal("Edited diamond").with_color(0x55FF55FF);
            }
            lines.retain(|line| line.plain_text() != "Hide this detail");
        });
    }
});
```

### `TooltipText`

An owned editable entry. Existing entries preserve their complete native text internally, including styling not represented by a simple string. Entries can be cloned or cached without referring to a slot index from an earlier tooltip. New entries can be literal or translated; arbitrary rich-component mutation is not exposed here. `component()` provides a read-only copy for inspection.

```rust
use pluto_lib::TooltipText;
let line = TooltipText::literal("Detail");
assert_eq!(line.clone().plain_text(), "Detail");
```

### `TooltipText::literal`

`fn literal(text: impl Into<String>) -> TooltipText`. A literal entry with grey base tint.

```rust
use pluto_lib::TooltipText;
assert_eq!(TooltipText::literal("Detail").plain_text(), "Detail");
```

### `TooltipText::translatable`

`fn translatable(key: impl Into<String>) -> TooltipText`. A translated entry with grey base tint. The native renderer resolves the key in the client's language; `plain_text()` uses PlutoLib's current language lookup.

```rust
use pluto_lib::TooltipText;
let line = TooltipText::translatable("item.minecraft.diamond");
assert_eq!(line.component()["translate"], "item.minecraft.diamond");
```

### `TooltipText::plain_text`

`fn plain_text(&self) -> String`. Visible text without decoration. For existing entries this is the native text's resolved snapshot, not a guessed translation of its component.

```rust
use pluto_lib::TooltipText;
assert_eq!(TooltipText::literal("Read me").plain_text(), "Read me");
```

### `TooltipText::component`

`fn component(&self) -> serde_json::Value`. A copy of the text component, retaining the original rich structure for existing entries. Editing the returned JSON does not change the entry. The separate base tint is read with `color()`.

```rust
use pluto_lib::TooltipText;
let line = TooltipText::literal("Original");
let mut component = line.component();
component["text"] = "Copy only".into();
assert_eq!(line.plain_text(), "Original");
```

### `TooltipText::color`

`fn color(&self) -> u32`. The base line tint as `0xRRGGBBAA`; embedded rich-text colors can override it.

```rust
use pluto_lib::TooltipText;
assert_eq!(TooltipText::literal("Grey").color(), 0xAAAAAAFF);
```

### `TooltipText::with_color`

`fn with_color(self, color: u32) -> TooltipText`. Change the base tint without reconstructing the rich text. For unstyled new text the tint is the visible color; existing embedded span colors retain their precedence.

```rust
use pluto_lib::TooltipText;
let line = TooltipText::literal("Green").with_color(0x55FF55FF);
assert_eq!(line.color(), 0x55FF55FF);
assert_eq!(line.plain_text(), "Green");
```

### `ItemTooltipCallback`

Holder of the tooltip event.

```rust
use pluto_lib::ItemTooltipCallback;

ItemTooltipCallback::EVENT.register(|_| {});
```

### `ItemTooltipCallback::EVENT`

Runs whenever the game builds a tooltip. Edit current lines with `with_lines` or append simple lines to `added`. Manual `EVENT.invoke` still supports `added` but does not create a native editable-list scope.

```rust
use pluto_lib::{ItemTooltipCallback, ItemTooltipContext, StackData, TooltipLine};
use std::cell::RefCell;

ItemTooltipCallback::EVENT.register(|tip| {
    if tip.item == "minecraft:diamond" {
        tip.added.borrow_mut().push(TooltipLine::grey("Shiny"));
    }
});
let tip = ItemTooltipContext { item: "minecraft:diamond".into(), count: 1, stack: StackData { item: "minecraft:diamond".into(), count: 1, ..Default::default() }, added: RefCell::new(vec![]) };
ItemTooltipCallback::EVENT.invoke(&tip);
assert_eq!(tip.added.borrow().len(), 1);
```

## Using an item over time

An item can take time to use (eating, drinking, drawing): register how long it takes and which pose the hand shows. Applies to every stack of the registered item, on both sides. A use that completes can be followed with the game's own finish handling.

### `UseAction`

The pose while using: `None` (held still), `Eat`, `Drink`, `Block` (shield), `Bow`, `Trident`, `Crossbow`, `Spyglass`, `TootHorn`, `Brush`, `Bundle`, `Spear`.

```rust
use pluto_lib::UseAction;

let poses = [UseAction::None, UseAction::Eat, UseAction::Drink, UseAction::Block, UseAction::Bow, UseAction::Trident,
             UseAction::Crossbow, UseAction::Spyglass, UseAction::TootHorn, UseAction::Brush, UseAction::Bundle, UseAction::Spear];
assert_eq!(poses.len(), 12);
```

### `ItemUse`

How an item is used over time. `use_time` is the number of ticks the use takes (20 ticks are one second), `action` the pose, `food` what completing the use feeds, `can_always_eat` whether a feeding item can be started at full hunger, `remainder` what is handed back when one of the item is used up. Build one with `ItemUse::new` and the builder methods.

```rust
use pluto_lib::{Identifier, ItemUse, UseAction};

let stew = ItemUse::new(32, UseAction::Eat)
    .food(6, 0.6)
    .remainder(Identifier::new("minecraft", "bowl").unwrap(), 1);
assert_eq!(stew.use_time, 32);
assert_eq!(stew.action, UseAction::Eat);
assert!(stew.food.is_some());
assert!(!stew.can_always_eat);
assert_eq!(stew.remainder.unwrap().1, 1);
```

### `ItemUse::new`

`fn new(use_time: i32, action: UseAction) -> ItemUse`. An item used for `use_time` ticks in the given pose, with no food and no remainder.

```rust
use pluto_lib::{ItemUse, UseAction};

let horn = ItemUse::new(140, UseAction::TootHorn);
assert!(horn.food.is_none() && horn.remainder.is_none());
```

### `ItemUse::food`

`fn food(self, nutrition: i32, saturation: f32) -> ItemUse`. Completing the use feeds the player. A player who is not hungry cannot start using a feeding item (unless `can_always_eat`), as with vanilla food.

```rust
use pluto_lib::{ItemUse, UseAction};

let snack = ItemUse::new(32, UseAction::Eat).food(4, 0.3);
assert_eq!(snack.food.unwrap().nutrition, 4);
```

### `ItemUse::can_always_eat`

`fn can_always_eat(self) -> ItemUse`. A feeding item that can be started at full hunger (like a golden apple).

```rust
use pluto_lib::{ItemUse, UseAction};

let apple = ItemUse::new(32, UseAction::Eat).food(4, 1.2).can_always_eat();
assert!(apple.can_always_eat);
```

### `ItemUse::remainder`

`fn remainder(self, item: Identifier, count: u32) -> ItemUse`. When one of the item is used up, `count` of `item` is handed back: in the hand if it was the last one, else into the inventory (the use is refused when it would not fit).

```rust
use pluto_lib::{Identifier, ItemUse, UseAction};

let potion = ItemUse::new(32, UseAction::Drink).remainder(Identifier::new("minecraft", "glass_bottle").unwrap(), 1);
assert_eq!(potion.remainder.unwrap().0.to_string(), "minecraft:glass_bottle");
```

### `EnderMaskContext`

A query with `item: String`, `stack: Option<StackData>`, `wearer_uuid: Option<u128>` and `observer_entity: Option<u32>`. Missing context is not a fabricated empty stack or ID zero. `stack` is the helmet the wearer wore as of their last tick when it carries custom data (what a mod's own armor keeps its state in), and `None` otherwise.

```rust
use pluto_lib::EnderMaskContext;
let query = EnderMaskContext { item: "example:visor".into(), stack: None,
    wearer_uuid: Some(7), observer_entity: Some(42) };
assert!(query.stack.is_none());
```

### `ItemBehavior`

Owned Minecraft item extension registration, cloneable without duplicating callback state.

```rust
use pluto_lib::{Identifier, ItemBehavior};
let behavior = ItemBehavior::new(Identifier::parse("example:visor").unwrap());
let copy = behavior.clone();
```

### `ItemBehavior::new`

`fn new(item: Identifier) -> ItemBehavior`. Starts with no overrides, preserving native queries.

```rust
use pluto_lib::{Identifier, ItemBehavior};
let behavior = ItemBehavior::new(Identifier::parse("example:helmet").unwrap());
```

### `ItemBehavior::is_ender_mask`

`fn is_ender_mask(self, callback: impl Fn(&EnderMaskContext) -> Option<bool> + Send + Sync + 'static) -> ItemBehavior`. Sets an explicit answer or native delegation. The callback must be safe to invoke from the native AI query.

```rust
use pluto_lib::{Identifier, ItemBehavior};
let behavior = ItemBehavior::new(Identifier::parse("example:visor").unwrap())
    .is_ender_mask(|query| query.stack.as_ref().map(|stack| stack.damage == 0));
```

### `ItemBehaviors`

Registry for item extension behaviors. Separate items register independently.

```rust
use pluto_lib::{Identifier, ItemBehavior, ItemBehaviors};
ItemBehaviors::register(ItemBehavior::new(Identifier::parse("example:visor").unwrap())).unwrap();
```

### `ItemBehaviors::register`

`fn register(behavior: ItemBehavior) -> Result<(), String>`. Registers by item ID; duplicate or poisoned registry returns an error without replacing the first entry.

```rust
use pluto_lib::{Identifier, ItemBehavior, ItemBehaviors};
let id = Identifier::parse("example:visor").unwrap();
ItemBehaviors::register(ItemBehavior::new(id.clone()).is_ender_mask(|_| Some(true))).unwrap();
assert!(ItemBehaviors::register(ItemBehavior::new(id)).is_err());
```

### `Food`

What eating an item gives. `nutrition` is the hunger points restored (2 are one drumstick), `saturation` the saturation restored.

```rust
use pluto_lib::Food;

let bread = Food { nutrition: 5, saturation: 6.0 };
assert_eq!(bread.nutrition, 5);
assert_eq!(bread.saturation, 6.0);
```

### `ItemUseRegistry`

Registers the use behavior of items.

```rust
use pluto_lib::{Identifier, ItemUse, ItemUseRegistry, UseAction};

ItemUseRegistry::register(Identifier::new("example", "jerky").unwrap(), ItemUse::new(16, UseAction::Eat).food(3, 0.5));
```

### `ItemUseRegistry::register`

`fn register(item: Identifier, behavior: ItemUse)`. Makes `item` usable over time. Registering the same item again replaces the earlier entry.

```rust
use pluto_lib::{Identifier, ItemUse, ItemUseRegistry, UseAction};

let tonic = Identifier::new("example", "tonic").unwrap();
ItemUseRegistry::register(tonic.clone(), ItemUse::new(32, UseAction::Drink));
ItemUseRegistry::register(tonic, ItemUse::new(40, UseAction::Drink)); // replaces
assert_eq!(ItemUseRegistry::get("example:tonic").unwrap().use_time, 40);
```

### `ItemUseRegistry::register_stack`

`fn register_stack(item: Identifier, behavior: StackUse)`. Makes `item` usable over time, decided per stack. Registering the same item again replaces the earlier entry. The use time and pose are asked where the game asks them, a use of a stack the registration accepts never starts for a player who is not hungry when `requires_hunger` is set, and the food is given when it completes.

```rust
use pluto_lib::{Identifier, ItemUseRegistry, StackUse, UseAction};

let id = Identifier::new("example", "flask").unwrap();
ItemUseRegistry::register_stack(id.clone(), StackUse::new(UseAction::Drink).use_time(|_| 20));
ItemUseRegistry::register_stack(id, StackUse::new(UseAction::Drink).use_time(|_| 40)); // replaces
assert!(ItemUseRegistry::stack_use("example:flask").is_some());
```

### `ItemUseRegistry::stack_use`

`fn stack_use(item: &str) -> Option<StackUse>`. The per-stack use registered for an item id such as `example:flask`, or `None`.

```rust
use pluto_lib::ItemUseRegistry;

assert!(ItemUseRegistry::stack_use("example:never_registered").is_none());
```

### `StackUse`

How one stack of an item is used over time, decided per stack (Fabric: `getMaxUseTime(ItemStack)`). It has no public fields: build it with `StackUse::new` and the chained methods.

```rust
use pluto_lib::{StackUse, UseAction};

let use_it = StackUse::new(UseAction::Eat).can_use(|stack| stack.damage == 0).use_time(|_| 16);
assert!(format!("{use_it:?}").contains("Eat"));
```

### `StackUse::new`

`fn new(action: UseAction) -> StackUse`. A use in the given pose that every stack may start, for 0 ticks (so nothing happens until `use_time` says otherwise), with no food.

```rust
use pluto_lib::{StackUse, UseAction};

let _plain = StackUse::new(UseAction::Drink);
```

### `StackUse::can_use`

`fn can_use(self, can_use: impl Fn(&StackData) -> bool + Send + Sync + 'static) -> StackUse`. Whether this registration applies to the stack (a broken tool, a tool without the stat that makes it edible). A refused stack is left to the game: its own use time, pose and rules, usually no use at all. It runs often, when a use starts and as the hand draws one in use: answer from the stack alone and cheaply.

```rust
use pluto_lib::{StackUse, UseAction};

let _only_whole = StackUse::new(UseAction::Eat).can_use(|stack| stack.damage == 0);
```

### `StackUse::use_time`

`fn use_time(self, use_time: impl Fn(&StackData) -> i32 + Send + Sync + 'static) -> StackUse`. The ticks this stack's use takes. Only asked for a stack `can_use` accepts; 0 or less means the stack is not used over time.

```rust
use pluto_lib::{StackUse, UseAction};

let _slow_when_worn = StackUse::new(UseAction::Eat).use_time(|stack| 16 + stack.damage as i32);
```

### `StackUse::food`

`fn food(self, food: impl Fn(&StackData) -> Option<Food> + Send + Sync + 'static) -> StackUse`. What completing the use feeds for this stack (`None` feeds nothing). The player is fed on the server before `UseItemFinishCallback` is called. Do not also register a plain `ItemUse` with food for the item.

```rust
use pluto_lib::{Food, StackUse, UseAction};

let _fed = StackUse::new(UseAction::Eat).food(|stack| Some(Food { nutrition: stack.count as i32, saturation: 0.3 }));
```

### `StackUse::requires_hunger`

`fn requires_hunger(self) -> StackUse`. A player who is not hungry (food level 20) cannot start using the stack unless they are in creative mode (Fabric: the `canEat(false)` check of a food item). Use it for a stack whose finish callback feeds the player itself.

```rust
use pluto_lib::{StackUse, UseAction};

let _only_when_hungry = StackUse::new(UseAction::Eat).requires_hunger();
```

### `ItemUseRegistry::get`

`fn get(item: &str) -> Option<ItemUse>`. What is registered for an item id such as `example:tonic`, or `None`.

```rust
use pluto_lib::{Identifier, ItemUse, ItemUseRegistry, UseAction};

assert_eq!(ItemUseRegistry::get("example:unknown"), None);
ItemUseRegistry::register(Identifier::new("example", "horn").unwrap(), ItemUse::new(140, UseAction::TootHorn));
assert_eq!(ItemUseRegistry::get("example:horn").unwrap().action, UseAction::TootHorn);
```
