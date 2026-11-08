# Screens and menus for blocks

A block with item slots can open a menu when a player uses it. The slots come from the block entity (`BlockEntityType::item_slots`); a screen is the window around them: where the slots and the player's inventory sit, what is drawn, which presses are menu buttons, what the tooltips say and what the server does when a button is pressed.

Register the screen in your mod's `hooks()`, together with the block entity type:

```rust
use pluto_lib::{BlockEntityType, Identifier, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout};

let oven = Identifier::new("example", "oven").unwrap();
BlockEntityType::new(oven.clone()).item_slots(3).menu("minecraft:furnace"); // the menu type the game opens
ScreenHandlerRegistry::register(ScreenHandlerType::new(
    oven,
    ScreenLayout::new(176.0, 166.0).slot(56.0, 17.0).slot(56.0, 53.0).slot(116.0, 35.0),
));
```

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

What the button callback receives on the server when a player presses a menu button. `pos` is the block entity's position, `id` the button (what the click callback answered), `data` the block entity's saved data and `items` its item slots, which the callback may change.

```rust
use pluto_lib::{MenuButton, Nbt, NbtCompound};

fn toggle(button: &mut MenuButton<'_>) {
    if button.id == 1 {
        button.data.insert("lit".to_owned(), Nbt::Byte(1));
    }
}
let mut data = NbtCompound::new();
let mut items = Vec::new();
toggle(&mut MenuButton { pos: [0, 64, 0], id: 1, data: &mut data, items: &mut items });
assert!(data.contains_key("lit"));
```

### `ScreenHandlerType`

The window of a block's menu. `block` is the block it belongs to and `layout` its layout; the callbacks are added with `on_draw`, `on_click`, `on_tooltip` and `on_button`.

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
