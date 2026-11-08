# Screens and menus for blocks

A block with item slots can open a menu when a player uses it. The slots come from the block entity (`BlockEntityType::item_slots`); a screen is the window around them: where the slots and the player's inventory sit, what is drawn, which presses are menu buttons, what the tooltips say and what the server does when a button is pressed.

Register the screen in your mod's `hooks()`, together with the block entity type. This page first walks through using screens, then lists every item in the [Reference](#reference).

## Basic Example

A screen is the window around a block's menu slots. The block entity declares the slots and the menu; the screen says where they sit and what is drawn:

```rust
use pluto_lib::{BlockEntityType, Identifier, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout};

let oven = Identifier::new("example", "oven").unwrap();
BlockEntityType::new(oven.clone()).item_slots(3).menu("minecraft:furnace"); // the menu type the game opens
ScreenHandlerRegistry::register(
    ScreenHandlerType::new(oven, ScreenLayout::new(176.0, 166.0).slot(56.0, 17.0).slot(56.0, 53.0).slot(116.0, 35.0))
        .on_draw(|draw| {
            let [x, y] = draw.origin;
            draw.canvas.text("Oven", [x + 8.0, y + 6.0], 0x404040, false);
        }),
);
```

## A Button That Reaches the Server

A press on the window is handled on the client first. `on_click` decides whether it is on one of your buttons and answers with an id; the game sends that id to the server, where `on_button` changes the block entity:

```rust
use pluto_lib::{Identifier, Nbt, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout};

ScreenHandlerRegistry::register(
    ScreenHandlerType::new(Identifier::new("example", "switch_box").unwrap(), ScreenLayout::new(176.0, 100.0))
        .on_click(|press| ((150.0..170.0).contains(&press.at[0]) && !press.right).then_some(1))
        .on_tooltip(|tip| {
            if (150.0..170.0).contains(&tip.at[0]) {
                tip.lines.push("Toggle".to_owned());
            }
        })
        .on_button(|press| {
            press.data.insert("on".to_owned(), Nbt::Byte(1));
        }),
);
```

## Reading the Machine's State

The draw, click and tooltip callbacks get the menu's *data slots*, the numbers the block entity shares with its menu (a flame and an arrow in a furnace). They are empty until the server's first values arrive, so read them with `get`:

```rust
use pluto_lib::{Identifier, ScreenHandlerType, ScreenLayout};

let window = ScreenHandlerType::new(Identifier::new("example", "oven").unwrap(), ScreenLayout::new(176.0, 166.0)).on_draw(|draw| {
    let flame = draw.data.first().copied().unwrap_or(0);
    draw.canvas.fill([draw.origin[0] + 57.0, draw.origin[1] + 37.0, 14.0, flame.min(14) as f32], 0xFFFF6600);
});
let _ = window;
```

> **Use with Caution.** The slot count of the menu type must match the block entity's `item_slots`. The window is drawn over the game's panel; set `with_own_background` on the layout if you draw your own.

## What the Slots Take

`on_slot_rule` decides what each slot of the menu takes and how many it holds. A result slot, which only gives, answers `may_place = false`; a fuel slot can take fewer. `on_quick_move` says where a shift-clicked stack goes, and `on_accepts` whether a hopper may put an item in:

```rust
use pluto_lib::{Identifier, ItemSlot, ScreenHandlerType, ScreenLayout};

let window = ScreenHandlerType::new(Identifier::new("example", "oven").unwrap(), ScreenLayout::new(176.0, 166.0).slot(56.0, 17.0).slot(56.0, 53.0).slot(116.0, 35.0))
    .on_slot_rule(|rule| {
        if rule.slot == 2 {
            rule.may_place = false; // the result slot only gives
        }
    })
    .on_quick_move(|moved| {
        // fuel goes to the second slot
        if moved.stack.item.to_string() == "minecraft:coal" && moved.items.get(1).is_some_and(|slot| slot.is_none()) {
            moved.items[1] = Some(ItemSlot { item: moved.stack.item.clone(), count: moved.stack.count });
            moved.stack.count = 0;
            return true;
        }
        false
    })
    .on_accepts(|put| put.item != "minecraft:dirt");
let _ = window;
```

> **Use with Caution.** The slot rule runs inside every click, drag and shift-click, several times for one, with no world or player at hand. Answer from the stack alone, and fast.

## Reference

### `ScreenLayout`

Where things are in the window, in GUI pixels from its top left. `width` and `height` are the window's size (vanilla container windows are 176 wide), `slots` where each container slot is (`None` hides the slot, and it is not in the menu either, so shift-clicks skip it), `inventory_x` and `inventory_y` the player's inventory (vanilla's are 8 and 84; the hotbar is 58 below), `own_labels` whether the screen draws the title and the "Inventory" label itself, `own_background` whether it draws its own background.

```rust
use pluto_lib::ScreenLayout;

let layout = ScreenLayout::new(176.0, 166.0);
assert_eq!((layout.width, layout.height), (176.0, 166.0));
assert!(layout.slots.is_empty());
assert_eq!((layout.inventory_x, layout.inventory_y), (8.0, 84.0));
assert!(!layout.own_labels && !layout.own_background);
```

### `ScreenLayout::new`

`fn new(width: f32, height: f32) -> ScreenLayout`. A window with no slots, the inventory at vanilla's place and the game drawing the panel and the labels.

```rust
use pluto_lib::ScreenLayout;

let layout = ScreenLayout::new(176.0, 200.0);
assert_eq!(layout.height, 200.0);
```

### `ScreenLayout::slot`

`fn slot(self, x: f32, y: f32) -> ScreenLayout`. One more container slot at `(x, y)`. Slots are numbered in the order they are added, the same as the block entity's item slots.

```rust
use pluto_lib::ScreenLayout;

let layout = ScreenLayout::new(176.0, 166.0).slot(56.0, 17.0).slot(56.0, 53.0);
assert_eq!(layout.slots, vec![Some([56.0, 17.0]), Some([56.0, 53.0])]);
```

### `ScreenLayout::hidden_slot`

`fn hidden_slot(self) -> ScreenLayout`. One more container slot that is hidden: it keeps its number but is not shown and shift-clicks skip it.

```rust
use pluto_lib::ScreenLayout;

let layout = ScreenLayout::new(176.0, 166.0).slot(8.0, 8.0).hidden_slot();
assert_eq!(layout.slots[1], None);
```

### `ScreenLayout::inventory_at`

`fn inventory_at(self, x: f32, y: f32) -> ScreenLayout`. The player's inventory at `(x, y)`, its first row and left column.

```rust
use pluto_lib::ScreenLayout;

let layout = ScreenLayout::new(176.0, 200.0).inventory_at(8.0, 118.0);
assert_eq!(layout.inventory_y, 118.0);
```

### `ScreenLayout::with_own_labels`

`fn with_own_labels(self) -> ScreenLayout`. The screen draws the title and the "Inventory" label itself.

```rust
use pluto_lib::ScreenLayout;

assert!(ScreenLayout::new(176.0, 166.0).with_own_labels().own_labels);
```

### `ScreenLayout::with_own_background`

`fn with_own_background(self) -> ScreenLayout`. The screen draws its own background; the plain panel and the slot frames are left out.

```rust
use pluto_lib::ScreenLayout;

assert!(ScreenLayout::new(176.0, 166.0).with_own_background().own_background);
```

### `ScreenCanvas`

The surface a screen draws on, in GUI pixels: `fill(rect, argb)`, `sprite(name, src, dst) -> bool` (a part of a GUI texture into a rectangle), `text(text, at, rgb, shadow)` and `text_width(text) -> f32`. PlutoLib passes one to the draw callback; a test can implement it to check what a screen draws.

```rust
use pluto_lib::ScreenCanvas;

struct Counting(u32);
impl ScreenCanvas for Counting {
    fn fill(&mut self, _: [f32; 4], _: u32) { self.0 += 1; }
    fn sprite(&mut self, _: &str, _: [f32; 4], _: [f32; 4]) -> bool { self.0 += 1; true }
    fn text(&mut self, _: &str, _: [f32; 2], _: u32, _: bool) { self.0 += 1; }
    fn text_width(&self, text: &str) -> f32 { text.len() as f32 * 6.0 }
}
let mut canvas = Counting(0);
canvas.fill([0.0, 0.0, 10.0, 10.0], 0xFF000000);
canvas.text("hi", [0.0, 0.0], 0xFFFFFF, true);
assert_eq!(canvas.0, 2);
assert_eq!(canvas.text_width("ab"), 12.0);
```

### `ScreenDraw`

What the draw callback receives. `origin` is the window's top left in GUI pixels, `data` the menu's data slots (what the block entity set; empty until the server's first values arrive, so read it with `get`), `canvas` where to draw.

```rust
use pluto_lib::{ScreenCanvas, ScreenDraw};

fn draw_bar(draw: &mut ScreenDraw<'_>) {
    let heat = draw.data.first().copied().unwrap_or(0) as f32;
    let [x, y] = draw.origin;
    draw.canvas.fill([x + 56.0, y + 36.0, heat.min(14.0), 14.0], 0xFFFF6600);
}
struct Null;
impl ScreenCanvas for Null {
    fn fill(&mut self, rect: [f32; 4], _: u32) { assert_eq!(rect, [156.0, 86.0, 10.0, 14.0]); }
    fn sprite(&mut self, _: &str, _: [f32; 4], _: [f32; 4]) -> bool { false }
    fn text(&mut self, _: &str, _: [f32; 2], _: u32, _: bool) {}
    fn text_width(&self, _: &str) -> f32 { 0.0 }
}
draw_bar(&mut ScreenDraw { origin: [100.0, 50.0], data: &[10], canvas: &mut Null });
```

### `ScreenClick`

What the click callback receives: a left or right press over no slot. `at` is where, in GUI pixels from the window's top left (outside it too), `right` whether it is the right button, `data` the menu's data slots.

```rust
use pluto_lib::ScreenClick;

let press = ScreenClick { at: [120.0, 40.0], right: false, data: &[] };
let on_button = press.at[0] > 100.0 && !press.right;
assert!(on_button);
assert!(press.data.is_empty());
```

### `ScreenTooltip`

What the tooltip callback receives when the mouse is over no item. `at` is where, `data` the menu's data slots, `lines` the lines to show (push to add; legacy colour codes `§` work).

```rust
use pluto_lib::ScreenTooltip;

let mut lines = Vec::new();
let mut tip = ScreenTooltip { at: [10.0, 10.0], data: &[42], lines: &mut lines };
tip.lines.push(format!("Heat: {}", tip.data[0]));
assert_eq!(lines, ["Heat: 42"]);
```

### `MenuButton`

What the button callback receives on the server when a player presses a menu button. `pos` is the block entity's position, `id` the button (what the click callback answered), `data` the block entity's saved data, `items` its item slots and `carried` the stack on the pressing player's cursor, which the callback may change.

```rust
use pluto_lib::{MenuButton, Nbt, NbtCompound};

fn toggle(button: &mut MenuButton<'_>) {
    if button.id == 1 {
        button.data.insert("lit".to_owned(), Nbt::Byte(1));
    }
}
let mut data = NbtCompound::new();
let mut items = Vec::new();
let mut carried = None;
toggle(&mut MenuButton { pos: [0, 64, 0], id: 1, data: &mut data, items: &mut items, carried: &mut carried });
assert!(data.contains_key("lit"));
```

### `SlotRule`

What a slot of the menu takes. `slot` is the container slot counted from 0, `item` and `count` the stack being asked about, and `may_place` and `max_stack` come in with the menu's own answer; change them. `max_stack` is kept between 1 and 99.

```rust
use pluto_lib::SlotRule;

let mut rule = SlotRule { slot: 2, item: "minecraft:coal", count: 1, may_place: true, max_stack: 64 };
rule.may_place = false;
rule.max_stack = 16;
assert_eq!((rule.slot, rule.item, rule.count), (2, "minecraft:coal", 1));
assert!(!rule.may_place && rule.max_stack == 16);
```

### `QuickMove`

A shift-click on a slot. `pos` is the block entity's position, `slot` the menu's index (below `container_slots` the block's own slot, then the player's 27 main slots, then the hotbar), `container_slots` how many slots the block's container has, `stack` a copy of what the slot holds (take what you move out of it), `data` the saved data and `items` the item slots, which the callback may change.

```rust
use pluto_lib::{Identifier, ItemSlot, NbtCompound, QuickMove};

let mut stack = ItemSlot { item: Identifier::new("minecraft", "coal").unwrap(), count: 12 };
let mut data = NbtCompound::new();
let mut items = vec![None, None];
let mut moved = QuickMove { pos: [0, 64, 0], slot: 30, container_slots: 2, stack: &mut stack, data: &mut data, items: &mut items };
moved.items[0] = Some(moved.stack.clone());
moved.stack.count = 0;
assert_eq!((moved.pos, moved.slot, moved.container_slots), ([0, 64, 0], 30, 2));
assert!(moved.data.is_empty());
assert_eq!(items[0].as_ref().map(|s| s.count), Some(12));
```

### `ContainerAccept`

A hopper, dropper or other block asks whether the container takes an item. `pos` is the block entity's position, `slot` where it would go, `item` and `count` the stack, and `face` the side it comes in from, if any.

```rust
use pluto_lib::{ContainerAccept, Direction};

let put = ContainerAccept { pos: [0, 64, 0], slot: 0, item: "minecraft:dirt", count: 1, face: Some(Direction::Up) };
assert_eq!((put.pos, put.slot, put.item, put.count), ([0, 64, 0], 0, "minecraft:dirt", 1));
assert_eq!(put.face, Some(Direction::Up));
```

### `ScreenHandlerType::on_slot_rule`

`fn on_slot_rule(self, rule: impl Fn(&mut SlotRule<'_>) + Send + Sync + 'static) -> ScreenHandlerType`. `rule` decides what each slot of the menu takes (Minecraft: `Slot.mayPlace`, `Slot.getMaxStackSize`). A slot that only gives answers `may_place = false`. The game asks the rule only for a menu type that a content mod adds (`data/<id>/horizon/menu/<name>.json`), not for a vanilla one such as the furnace.

```rust
use pluto_lib::{Identifier, ScreenHandlerType, ScreenLayout};

let window = ScreenHandlerType::new(Identifier::new("example", "oven").unwrap(), ScreenLayout::new(176.0, 166.0))
    .on_slot_rule(|rule| rule.max_stack = if rule.item == "minecraft:coal" { 16 } else { rule.max_stack });
let _ = window;
```

### `ScreenHandlerType::on_quick_move`

`fn on_quick_move(self, quick_move: impl Fn(&mut QuickMove<'_>) -> bool + Send + Sync + 'static) -> ScreenHandlerType`. `quick_move` says where a shift-clicked stack goes (Minecraft: `quickMoveStack`). Answer `true` when you handled it: the slot then holds what is left of `stack` and the game's own routing is skipped. Answer `false` and what you changed is undone.

```rust
use pluto_lib::{Identifier, ScreenHandlerType, ScreenLayout};

let window = ScreenHandlerType::new(Identifier::new("example", "oven").unwrap(), ScreenLayout::new(176.0, 166.0))
    .on_quick_move(|moved| moved.slot >= moved.container_slots && moved.stack.count == 0);
let _ = window;
```

### `ScreenHandlerType::on_accepts`

`fn on_accepts(self, accepts: impl Fn(&ContainerAccept<'_>) -> bool + Send + Sync + 'static) -> ScreenHandlerType`. `accepts` says whether the container takes an item that a hopper, a dropper or another block puts in (NeoForge: `IItemHandler.isItemValid`). Players' clicks are the slot rules' instead.

```rust
use pluto_lib::{Identifier, ScreenHandlerType, ScreenLayout};

let window = ScreenHandlerType::new(Identifier::new("example", "millstone").unwrap(), ScreenLayout::new(176.0, 166.0))
    .on_accepts(|put| put.item == "minecraft:wheat");
let _ = window;
```

### `ScreenHandlerType`

The window of a block's menu. `block` is the block it belongs to and `layout` its layout; the callbacks are added with `on_draw`, `on_click`, `on_tooltip`, `on_button`, `on_slot_rule`, `on_quick_move` and `on_accepts`.

```rust
use pluto_lib::{Identifier, ScreenHandlerType, ScreenLayout};

let window = ScreenHandlerType::new(Identifier::new("example", "oven").unwrap(), ScreenLayout::new(176.0, 166.0));
assert_eq!(window.block.path(), "oven");
assert_eq!(window.layout.width, 176.0);
```

### `ScreenHandlerType::new`

`fn new(block: Identifier, layout: ScreenLayout) -> ScreenHandlerType`. A window for `block`'s menu; nothing is drawn beyond the game's panel until `on_draw` is given.

```rust
use pluto_lib::{Identifier, ScreenHandlerType, ScreenLayout};

let window = ScreenHandlerType::new(Identifier::new("example", "oven").unwrap(), ScreenLayout::new(176.0, 166.0).slot(8.0, 8.0));
assert_eq!(window.layout.slots.len(), 1);
```

### `ScreenHandlerType::on_draw`

`fn on_draw(self, draw: impl Fn(&mut ScreenDraw<'_>) + Send + Sync + 'static) -> ScreenHandlerType`. `draw` draws the window each frame, over the panel and the slot frames and under the items and labels (Fabric: `HandledScreen.drawBackground`).

```rust
use pluto_lib::{Identifier, ScreenHandlerType, ScreenLayout};

let window = ScreenHandlerType::new(Identifier::new("example", "oven").unwrap(), ScreenLayout::new(176.0, 166.0)).on_draw(|draw| {
    let [x, y] = draw.origin;
    draw.canvas.fill([x + 8.0, y + 8.0, 20.0, 4.0], 0xFF808080);
    draw.canvas.text("Oven", [x + 8.0, y + 20.0], 0x404040, false);
});
let _ = window;
```

### `ScreenHandlerType::on_click`

`fn on_click(self, click: impl Fn(&ScreenClick<'_>) -> Option<i32> + Send + Sync + 'static) -> ScreenHandlerType`. `click` decides which menu button a press over no slot is on: answer `Some(id)` and the press is sent to the server as button `id`; `None` leaves it to the game.

```rust
use pluto_lib::{Identifier, ScreenHandlerType, ScreenLayout};

let window = ScreenHandlerType::new(Identifier::new("example", "oven").unwrap(), ScreenLayout::new(176.0, 166.0)).on_click(|press| {
    // a button at x 150..170, y 20..40
    let inside = (150.0..170.0).contains(&press.at[0]) && (20.0..40.0).contains(&press.at[1]);
    (inside && !press.right).then_some(1)
});
let _ = window;
```

### `ScreenHandlerType::on_tooltip`

`fn on_tooltip(self, tooltip: impl Fn(&mut ScreenTooltip<'_>) + Send + Sync + 'static) -> ScreenHandlerType`. `tooltip` adds lines when the mouse is over something of the screen's own and over no item.

```rust
use pluto_lib::{Identifier, ScreenHandlerType, ScreenLayout};

let window = ScreenHandlerType::new(Identifier::new("example", "oven").unwrap(), ScreenLayout::new(176.0, 166.0)).on_tooltip(|tip| {
    if (150.0..170.0).contains(&tip.at[0]) {
        tip.lines.push("Toggle the burner".to_owned());
    }
});
let _ = window;
```

### `ScreenHandlerType::on_button`

`fn on_button(self, button: impl Fn(&mut MenuButton<'_>) + Send + Sync + 'static) -> ScreenHandlerType`. `button` runs on the server when a player presses a menu button (Minecraft: `clickMenuButton`).

```rust
use pluto_lib::{Identifier, Nbt, ScreenHandlerType, ScreenLayout};

let window = ScreenHandlerType::new(Identifier::new("example", "oven").unwrap(), ScreenLayout::new(176.0, 166.0)).on_button(|press| {
    press.data.insert("burner".to_owned(), Nbt::Byte(press.id as i8));
});
let _ = window;
```

### `ScreenHandlerRegistry`

Registers windows (Fabric: `HandledScreens.register`).

```rust
use pluto_lib::{Identifier, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout};

ScreenHandlerRegistry::register(ScreenHandlerType::new(Identifier::new("example", "plain").unwrap(), ScreenLayout::new(176.0, 166.0)));
```

### `ScreenHandlerRegistry::register`

`fn register(handler: ScreenHandlerType)`. Gives `handler.block`'s menu its window. Registering the same block again replaces the earlier window.

```rust
use pluto_lib::{Identifier, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout};

let block = Identifier::new("example", "swapped").unwrap();
ScreenHandlerRegistry::register(ScreenHandlerType::new(block.clone(), ScreenLayout::new(100.0, 100.0)));
ScreenHandlerRegistry::register(ScreenHandlerType::new(block, ScreenLayout::new(200.0, 100.0)));
assert_eq!(ScreenHandlerRegistry::get("example:swapped").unwrap().layout.width, 200.0);
```

### `ScreenHandlerRegistry::get`

`fn get(block: &str) -> Option<ScreenHandlerType>`. The window registered for a block id such as `example:oven`, or `None`.

```rust
use pluto_lib::ScreenHandlerRegistry;

assert!(ScreenHandlerRegistry::get("example:no_window").is_none());
```

### `BlockEntityType::menu`

`fn menu(self, menu_type: &str) -> BlockEntityType`. The menu the game opens when the block is used; its slots are the block entity's item slots. One of `minecraft:furnace`, `minecraft:blast_furnace`, `minecraft:smoker` (3 slots), `minecraft:hopper` (5), `minecraft:generic_3x3` (9), `minecraft:generic_9x1` to `minecraft:generic_9x6` (9 to 54), or a menu type of the mod's content; the slot count must match.

```rust
use pluto_lib::{BlockEntityType, Identifier};

let kind = BlockEntityType::new(Identifier::new("example", "crate").unwrap()).item_slots(27).menu("minecraft:generic_9x3");
let _ = kind;
```
