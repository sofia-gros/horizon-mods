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
