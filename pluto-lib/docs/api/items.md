# Items: tooltips

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

> **Use with Caution.** `with_lines` is available only while this context is being dispatched by the game. It returns `None` for a manually invoked event or if the same list is already borrowed. Nested tooltips have separate lists. Legacy `added` entries are appended after all callbacks; they are not part of the editable list during dispatch.

## Reference

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

### `ItemUseRegistry::get`

`fn get(item: &str) -> Option<ItemUse>`. What is registered for an item id such as `example:tonic`, or `None`.

```rust
use pluto_lib::{Identifier, ItemUse, ItemUseRegistry, UseAction};

assert_eq!(ItemUseRegistry::get("example:unknown"), None);
ItemUseRegistry::register(Identifier::new("example", "horn").unwrap(), ItemUse::new(140, UseAction::TootHorn));
assert_eq!(ItemUseRegistry::get("example:horn").unwrap().action, UseAction::TootHorn);
```
