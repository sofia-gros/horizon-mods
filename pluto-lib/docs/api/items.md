# Items: tooltips

Add lines to an item's tooltip. Callbacks can add lines after the game's own; reading or editing the lines already there is a stub (support matrix).

### `TooltipLine`

One tooltip line to add. `text` is the line, `color` is `0xRRGGBBAA`.

```rust
use pluto_lib::TooltipLine;

let gold = TooltipLine { text: "Rare".to_owned(), color: 0xFFAA00FF };
assert_eq!(gold.color >> 24, 0xFF);
```

### `TooltipLine::grey`

`fn grey(text: impl Into<String>) -> TooltipLine`. A grey line, like most tooltip text.

```rust
use pluto_lib::TooltipLine;

assert_eq!(TooltipLine::grey("Hello").text, "Hello");
```

### `ItemTooltipContext`

The tooltip being built for an item stack. `item` is the item id (`tconstruct:flint_pickaxe`), `count` the stack size, `added` the lines callbacks want after the game's own (push onto it with `borrow_mut`).

```rust
use pluto_lib::{ItemTooltipCallback, ItemTooltipContext, TooltipLine};
use std::cell::RefCell;

ItemTooltipCallback::EVENT.register(|tip: &ItemTooltipContext| {
    if tip.count > 1 {
        tip.added.borrow_mut().push(TooltipLine::grey(format!("x{}", tip.count)));
    }
});
let tip = ItemTooltipContext { item: "minecraft:stick".into(), count: 8, added: RefCell::new(Vec::new()) };
ItemTooltipCallback::EVENT.invoke(&tip);
assert_eq!(tip.added.borrow()[0].text, "x8");
```

### `ItemTooltipCallback`

Holder of the tooltip event.

```rust
use pluto_lib::ItemTooltipCallback;

ItemTooltipCallback::EVENT.register(|_| {});
```

### `ItemTooltipCallback::EVENT`

Runs whenever the game builds a tooltip. Add lines to `added`.

```rust
use pluto_lib::{ItemTooltipCallback, ItemTooltipContext, TooltipLine};
use std::cell::RefCell;

ItemTooltipCallback::EVENT.register(|tip| {
    if tip.item == "minecraft:diamond" {
        tip.added.borrow_mut().push(TooltipLine::grey("Shiny"));
    }
});
let tip = ItemTooltipContext { item: "minecraft:diamond".into(), count: 1, added: RefCell::new(vec![]) };
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

How an item is used over time. `use_time` is the number of ticks the use takes (20 ticks are one second), `action` the pose.

```rust
use pluto_lib::{ItemUse, UseAction};

let slow_snack = ItemUse { use_time: 64, action: UseAction::Eat };
assert_eq!(slow_snack.use_time, 64);
assert_eq!(slow_snack.action, UseAction::Eat);
```

### `ItemUseRegistry`

Registers the use behavior of items.

```rust
use pluto_lib::{Identifier, ItemUse, ItemUseRegistry, UseAction};

ItemUseRegistry::register(Identifier::new("example", "jerky").unwrap(), ItemUse { use_time: 16, action: UseAction::Eat });
```

### `ItemUseRegistry::register`

`fn register(item: Identifier, behavior: ItemUse)`. Makes `item` usable over time. Registering the same item again replaces the earlier entry.

```rust
use pluto_lib::{Identifier, ItemUse, ItemUseRegistry, UseAction};

let tonic = Identifier::new("example", "tonic").unwrap();
ItemUseRegistry::register(tonic.clone(), ItemUse { use_time: 32, action: UseAction::Drink });
ItemUseRegistry::register(tonic, ItemUse { use_time: 40, action: UseAction::Drink }); // replaces
assert_eq!(ItemUseRegistry::get("example:tonic").unwrap().use_time, 40);
```

### `ItemUseRegistry::get`

`fn get(item: &str) -> Option<ItemUse>`. What is registered for an item id such as `example:tonic`, or `None`.

```rust
use pluto_lib::{Identifier, ItemUse, ItemUseRegistry, UseAction};

assert_eq!(ItemUseRegistry::get("example:unknown"), None);
ItemUseRegistry::register(Identifier::new("example", "horn").unwrap(), ItemUse { use_time: 140, action: UseAction::TootHorn });
assert_eq!(ItemUseRegistry::get("example:horn").unwrap().action, UseAction::TootHorn);
```
