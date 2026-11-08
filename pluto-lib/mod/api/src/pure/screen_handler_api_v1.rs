//! Screens for a block's menu: how the window is laid out, how it is drawn, what a click on it does and what a press of a menu button
//! does on the server (Minecraft: `ScreenHandler`, `HandledScreen`; Fabric: `ScreenHandlerRegistry`, `HandledScreens`).
//!
//! The menu itself (which slots, the items in them) comes from the block entity's item slots; here the block gets the window around
//! them. The window is laid out and drawn on the client and answers button presses on the server.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::base::{Identifier, NbtCompound};
use super::block_api_v1::Direction;
use super::block_entity_api_v1::{BlockPos, ItemSlot};

/// The surface a screen draws on, in GUI pixels. PlutoLib passes one to the draw callback; a test can implement it to check what a screen
/// draws.
pub trait ScreenCanvas {
    /// Fills `rect` (`[x, y, width, height]`) with `argb` (`0xAARRGGBB`).
    fn fill(&mut self, rect: [f32; 4], argb: u32);
    /// Draws the part `src` (`[x, y, width, height]`, in texture pixels) of the GUI texture `name` into `dst`; false if there is no such texture.
    fn sprite(&mut self, name: &str, src: [f32; 4], dst: [f32; 4]) -> bool;
    /// Draws `text` with its top left at `at`, in colour `rgb`, with a shadow if asked.
    fn text(&mut self, text: &str, at: [f32; 2], rgb: u32, shadow: bool);
    /// The width of `text` in GUI pixels.
    fn text_width(&self, text: &str) -> f32;
}

/// Where things are in a window, in GUI pixels from its top left (Minecraft: `Slot.x`, `Slot.y`).
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenLayout {
    /// The window's width; vanilla container windows are 176.
    pub width: f32,
    /// The window's height.
    pub height: f32,
    /// Where each container slot is; `None` hides the slot (it is not in the menu either, so shift-clicks skip it).
    pub slots: Vec<Option<[f32; 2]>>,
    /// The left column of the player's inventory (vanilla's is 8).
    pub inventory_x: f32,
    /// The top row of the player's inventory (vanilla's is 84; the hotbar is 58 below it).
    pub inventory_y: f32,
    /// The screen draws the title and the "Inventory" label itself.
    pub own_labels: bool,
    /// The screen draws its own background; the plain panel and the slot frames are left out.
    pub own_background: bool,
}

impl ScreenLayout {
    /// A window `width` by `height` with no slots, the inventory at vanilla's place and the game drawing the panel and the labels.
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height, slots: Vec::new(), inventory_x: 8.0, inventory_y: 84.0, own_labels: false, own_background: false }
    }

    /// The same layout with one more container slot at `(x, y)`.
    pub fn slot(mut self, x: f32, y: f32) -> Self {
        self.slots.push(Some([x, y]));
        self
    }

    /// The same layout with one more container slot that is hidden.
    pub fn hidden_slot(mut self) -> Self {
        self.slots.push(None);
        self
    }

    /// The same layout with the player's inventory at `(x, y)`.
    pub fn inventory_at(mut self, x: f32, y: f32) -> Self {
        self.inventory_x = x;
        self.inventory_y = y;
        self
    }

    /// The same layout, the screen drawing the title and the "Inventory" label.
    pub fn with_own_labels(mut self) -> Self {
        self.own_labels = true;
        self
    }

    /// The same layout, the screen drawing its own background.
    pub fn with_own_background(mut self) -> Self {
        self.own_background = true;
        self
    }
}

/// What the draw callback receives.
pub struct ScreenDraw<'a> {
    /// The window's top left in GUI pixels.
    pub origin: [f32; 2],
    /// The menu's data slots (what the block entity set); empty until the server's first values arrive, so read it with `get`.
    pub data: &'a [i32],
    /// Where to draw.
    pub canvas: &'a mut dyn ScreenCanvas,
}

/// What the click callback receives: a left or right press on the window over no slot.
pub struct ScreenClick<'a> {
    /// Where, in GUI pixels from the window's top left (outside it too).
    pub at: [f32; 2],
    /// The right button.
    pub right: bool,
    /// The menu's data slots.
    pub data: &'a [i32],
}

/// What the tooltip callback receives: the mouse is over no item.
pub struct ScreenTooltip<'a> {
    /// Where, in GUI pixels from the window's top left.
    pub at: [f32; 2],
    /// The menu's data slots.
    pub data: &'a [i32],
    /// Lines to show; push to add. Legacy colour codes (§) work.
    pub lines: &'a mut Vec<String>,
}

/// What the button callback receives on the server: a player pressed menu button `id`.
pub struct MenuButton<'a> {
    /// The block entity's position.
    pub pos: BlockPos,
    /// The pressed button's id (what the click callback answered).
    pub id: i32,
    /// The block entity's saved data, which the callback may change.
    pub data: &'a mut NbtCompound,
    /// The block entity's item slots, which the callback may change.
    pub items: &'a mut Vec<Option<ItemSlot>>,
    /// The stack on the pressing player's cursor, which the callback may change (Minecraft: `Player.containerMenu.getCarried`).
    pub carried: &'a mut Option<ItemSlot>,
}

/// What a slot of the menu takes (Minecraft: `Slot.mayPlace`, `Slot.getMaxStackSize`). Change `may_place` and `max_stack`; they come in with the
/// menu's own answer.
pub struct SlotRule<'a> {
    /// The container slot, counted from 0.
    pub slot: usize,
    /// The item being asked about.
    pub item: &'a str,
    /// How many of it.
    pub count: u32,
    /// Whether the slot takes the item.
    pub may_place: bool,
    /// How many the slot holds, 1 to 99 (the item's own stack size still caps it).
    pub max_stack: u32,
}

/// A shift-click on a slot of the menu (Minecraft: `quickMoveStack`).
pub struct QuickMove<'a> {
    /// The block entity's position.
    pub pos: BlockPos,
    /// The menu's slot index: below `container_slots` the block's own slot, then the player's 27 main slots, then the hotbar.
    pub slot: usize,
    /// How many slots the block's container has.
    pub container_slots: usize,
    /// A copy of what the slot holds; take what you move out of it.
    pub stack: &'a mut ItemSlot,
    /// The block entity's saved data, which the callback may change.
    pub data: &'a mut NbtCompound,
    /// The block entity's item slots, which the callback may change.
    pub items: &'a mut Vec<Option<ItemSlot>>,
}

/// A hopper, dropper or other block asks whether the container takes an item (NeoForge: `IItemHandler.isItemValid`).
pub struct ContainerAccept<'a> {
    /// The block entity's position.
    pub pos: BlockPos,
    /// The slot it would go into.
    pub slot: usize,
    /// The item.
    pub item: &'a str,
    /// How many.
    pub count: u32,
    /// The side it comes in from, if any.
    pub face: Option<Direction>,
}

type Draw = Arc<dyn Fn(&mut ScreenDraw<'_>) + Send + Sync>;
type Click = Arc<dyn Fn(&ScreenClick<'_>) -> Option<i32> + Send + Sync>;
type Tooltip = Arc<dyn Fn(&mut ScreenTooltip<'_>) + Send + Sync>;
type Button = Arc<dyn Fn(&mut MenuButton<'_>) + Send + Sync>;
type Rule = Arc<dyn Fn(&mut SlotRule<'_>) + Send + Sync>;
type Move = Arc<dyn Fn(&mut QuickMove<'_>) -> bool + Send + Sync>;
type Accept = Arc<dyn Fn(&ContainerAccept<'_>) -> bool + Send + Sync>;

/// The window of a block's menu.
#[derive(Clone)]
pub struct ScreenHandlerType {
    /// The block whose menu this window belongs to.
    pub block: Identifier,
    /// The window's layout.
    pub layout: ScreenLayout,
    draw: Option<Draw>,
    click: Option<Click>,
    tooltip: Option<Tooltip>,
    button: Option<Button>,
    rule: Option<Rule>,
    quick_move: Option<Move>,
    accepts: Option<Accept>,
}

impl ScreenHandlerType {
    /// A window for `block`'s menu with the given layout and nothing drawn beyond the game's panel.
    pub fn new(block: Identifier, layout: ScreenLayout) -> Self {
        Self { block, layout, draw: None, click: None, tooltip: None, button: None, rule: None, quick_move: None, accepts: None }
    }

    /// The same window, `draw` drawing it each frame over the panel and the slot frames and under the items (Fabric: `HandledScreen.drawBackground`).
    pub fn on_draw(mut self, draw: impl Fn(&mut ScreenDraw<'_>) + Send + Sync + 'static) -> Self {
        self.draw = Some(Arc::new(draw));
        self
    }

    /// The same window, `click` deciding which menu button a press over no slot is on: answer `Some(id)` and the press is sent to the
    /// server as button `id`, `None` leaves it to the game.
    pub fn on_click(mut self, click: impl Fn(&ScreenClick<'_>) -> Option<i32> + Send + Sync + 'static) -> Self {
        self.click = Some(Arc::new(click));
        self
    }

    /// The same window, `tooltip` adding lines when the mouse is over something of the screen's own and no item.
    pub fn on_tooltip(mut self, tooltip: impl Fn(&mut ScreenTooltip<'_>) + Send + Sync + 'static) -> Self {
        self.tooltip = Some(Arc::new(tooltip));
        self
    }

    /// The same window, `button` running on the server when a player presses a menu button (Minecraft: `clickMenuButton`).
    pub fn on_button(mut self, button: impl Fn(&mut MenuButton<'_>) + Send + Sync + 'static) -> Self {
        self.button = Some(Arc::new(button));
        self
    }
}

impl ScreenHandlerType {
    /// The same window, `rule` deciding what each slot of the menu takes (Minecraft: `Slot.mayPlace`, `Slot.getMaxStackSize`). It runs inside a
    /// click, a drag or a shift-click, several times for one, with no world or player at hand: answer from the stack, and fast. A slot that only
    /// gives (a result slot) answers `may_place = false`.
    pub fn on_slot_rule(mut self, rule: impl Fn(&mut SlotRule<'_>) + Send + Sync + 'static) -> Self {
        self.rule = Some(Arc::new(rule));
        self
    }

    /// The same window, `quick_move` saying where a shift-clicked stack goes (Minecraft: `quickMoveStack`). Answer `true` when you handled it:
    /// the slot then holds what is left of `stack` and the game's own routing is skipped. Answer `false` and everything you changed is undone.
    pub fn on_quick_move(mut self, quick_move: impl Fn(&mut QuickMove<'_>) -> bool + Send + Sync + 'static) -> Self {
        self.quick_move = Some(Arc::new(quick_move));
        self
    }

    /// The same window, `accepts` saying whether the container takes an item that a hopper, a dropper or another block puts in (NeoForge:
    /// `IItemHandler.isItemValid`). Players' clicks are the slot rules' instead.
    pub fn on_accepts(mut self, accepts: impl Fn(&ContainerAccept<'_>) -> bool + Send + Sync + 'static) -> Self {
        self.accepts = Some(Arc::new(accepts));
        self
    }
}

static HANDLERS: Mutex<Option<HashMap<String, ScreenHandlerType>>> = Mutex::new(None);

/// Registers windows (Fabric: `HandledScreens.register`).
pub struct ScreenHandlerRegistry;

impl ScreenHandlerRegistry {
    /// Gives `handler.block`'s menu its window. Registering the same block again replaces the earlier window.
    pub fn register(handler: ScreenHandlerType) {
        if let Ok(mut all) = HANDLERS.lock() {
            all.get_or_insert_with(HashMap::new).insert(handler.block.to_string(), handler);
        }
    }

    /// The window registered for `block` (an id such as `example:oven`), if any.
    pub fn get(block: &str) -> Option<ScreenHandlerType> {
        HANDLERS.lock().ok()?.as_ref()?.get(block).cloned()
    }
}

/// The layout for a block, if it has a window.
pub(crate) fn layout(block: &str) -> Option<ScreenLayout> {
    ScreenHandlerRegistry::get(block).map(|h| h.layout)
}

/// Draws the block's window; false if it has none or draws nothing.
pub(crate) fn draw(block: &str, origin: [f32; 2], data: &[i32], canvas: &mut dyn ScreenCanvas) -> bool {
    match ScreenHandlerRegistry::get(block).and_then(|h| h.draw) {
        Some(draw) => {
            draw(&mut ScreenDraw { origin, data, canvas });
            true
        }
        None => false,
    }
}

/// The menu button a press at `at` is on, if the window says so.
pub(crate) fn click(block: &str, at: [f32; 2], right: bool, data: &[i32]) -> Option<i32> {
    ScreenHandlerRegistry::get(block).and_then(|h| h.click).and_then(|click| click(&ScreenClick { at, right, data }))
}

/// Adds the window's tooltip lines for `at`.
pub(crate) fn tooltip(block: &str, at: [f32; 2], data: &[i32], lines: &mut Vec<String>) {
    if let Some(tooltip) = ScreenHandlerRegistry::get(block).and_then(|h| h.tooltip) {
        tooltip(&mut ScreenTooltip { at, data, lines });
    }
}

/// Runs the button callback of the window of the block entity type whose block is `block`.
pub(crate) fn press(block: &str, pos: BlockPos, id: i32, data: &mut NbtCompound, items: &mut Vec<Option<ItemSlot>>, carried: &mut Option<ItemSlot>) -> bool {
    match ScreenHandlerRegistry::get(block).and_then(|h| h.button) {
        Some(button) => {
            button(&mut MenuButton { pos, id, data, items, carried });
            true
        }
        None => false,
    }
}

/// Asks the window's slot rule; leaves `may_place` and `max_stack` as they are if it has none.
pub(crate) fn slot_rules(block: &str, slot: usize, item: &str, count: u32, may_place: &mut bool, max_stack: &mut u32) {
    if let Some(rule) = ScreenHandlerRegistry::get(block).and_then(|h| h.rule) {
        let mut asked = SlotRule { slot, item, count, may_place: *may_place, max_stack: *max_stack };
        rule(&mut asked);
        *may_place = asked.may_place;
        *max_stack = asked.max_stack.clamp(1, 99);
    }
}

/// Runs the window's shift-click callback; true if it handled the click.
pub(crate) fn quick_move(block: &str, pos: BlockPos, slot: usize, container_slots: usize, stack: &mut ItemSlot, data: &mut NbtCompound, items: &mut Vec<Option<ItemSlot>>) -> bool {
    match ScreenHandlerRegistry::get(block).and_then(|h| h.quick_move) {
        Some(quick_move) => quick_move(&mut QuickMove { pos, slot, container_slots, stack, data, items }),
        None => false,
    }
}

/// Whether the container takes the item; true if the window has no opinion.
pub(crate) fn accepts(block: &str, pos: BlockPos, slot: usize, item: &str, count: u32, face: Option<Direction>) -> bool {
    match ScreenHandlerRegistry::get(block).and_then(|h| h.accepts) {
        Some(accepts) => accepts(&ContainerAccept { pos, slot, item, count, face }),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::base::{Nbt, NbtCompoundExt};

    #[derive(Default)]
    struct Recorder {
        fills: Vec<([f32; 4], u32)>,
        texts: Vec<String>,
    }

    impl ScreenCanvas for Recorder {
        fn fill(&mut self, rect: [f32; 4], argb: u32) {
            self.fills.push((rect, argb));
        }
        fn sprite(&mut self, _: &str, _: [f32; 4], _: [f32; 4]) -> bool {
            true
        }
        fn text(&mut self, text: &str, _: [f32; 2], _: u32, _: bool) {
            self.texts.push(text.to_owned());
        }
        fn text_width(&self, text: &str) -> f32 {
            text.len() as f32 * 6.0
        }
    }

    #[test]
    fn a_window_has_its_layout_draws_answers_clicks_adds_tooltips_and_handles_the_server_button() {
        ScreenHandlerRegistry::register(
            ScreenHandlerType::new(Identifier::parse("test:oven").unwrap(), ScreenLayout::new(176.0, 166.0).slot(56.0, 17.0).hidden_slot())
                .on_draw(|d| {
                    let heat = d.data.first().copied().unwrap_or(0) as f32;
                    d.canvas.fill([d.origin[0] + 10.0, d.origin[1] + 10.0, heat, 4.0], 0xFFFF0000);
                    d.canvas.text("Oven", [d.origin[0], d.origin[1]], 0x404040, false);
                })
                .on_click(|c| (c.at[0] > 100.0 && !c.right).then_some(7))
                .on_tooltip(|t| {
                    if t.at[0] < 20.0 {
                        t.lines.push(format!("heat {}", t.data[0]));
                    }
                })
                .on_button(|b| {
                    b.data.insert("pressed".into(), Nbt::Int(b.id));
                    b.items.push(None);
                    *b.carried = None;
                })
                .on_slot_rule(|r| {
                    if r.slot == 2 {
                        r.may_place = false; // a result slot only gives
                    }
                    if r.item == "test:fuel" {
                        r.max_stack = 16;
                    }
                })
                .on_quick_move(|q| {
                    // fuel goes to slot 1; the rest is left to the game
                    if q.stack.item.to_string() == "test:fuel" {
                        q.items.resize(2, None);
                        q.items[1] = Some(q.stack.clone());
                        q.stack.count = 0;
                        true
                    } else {
                        false
                    }
                })
                .on_accepts(|a| a.item != "test:junk"),
        );
        let layout = layout("test:oven").unwrap();
        assert_eq!(layout.slots, vec![Some([56.0, 17.0]), None]);
        assert_eq!((layout.inventory_x, layout.inventory_y), (8.0, 84.0));
        assert!(super::layout("test:none").is_none());

        let mut canvas = Recorder::default();
        assert!(draw("test:oven", [100.0, 50.0], &[30], &mut canvas));
        assert_eq!(canvas.fills, vec![([110.0, 60.0, 30.0, 4.0], 0xFFFF0000)]);
        assert_eq!(canvas.texts, ["Oven"]);
        assert!(!draw("test:none", [0.0; 2], &[], &mut canvas));

        assert_eq!(click("test:oven", [120.0, 10.0], false, &[]), Some(7));
        assert_eq!(click("test:oven", [120.0, 10.0], true, &[]), None);
        assert_eq!(click("test:oven", [5.0, 10.0], false, &[]), None);

        let mut lines = Vec::new();
        tooltip("test:oven", [5.0, 5.0], &[42], &mut lines);
        assert_eq!(lines, ["heat 42"]);

        let mut data = NbtCompound::new();
        let mut items = Vec::new();
        let mut carried = Some(ItemSlot { item: Identifier::parse("test:fuel").unwrap(), count: 3 });
        assert!(press("test:oven", [1, 2, 3], 7, &mut data, &mut items, &mut carried));
        assert_eq!(data.get_int("pressed", 0), 7);
        assert_eq!(items.len(), 1);
        assert_eq!(carried, None, "the button took what was on the cursor");
        assert!(!press("test:none", [0; 3], 1, &mut data, &mut items, &mut carried));

        let (mut may, mut max) = (true, 99);
        slot_rules("test:oven", 2, "test:coal", 1, &mut may, &mut max);
        assert!(!may, "slot 2 only gives");
        let (mut may, mut max) = (true, 99);
        slot_rules("test:oven", 0, "test:fuel", 1, &mut may, &mut max);
        assert!(may);
        assert_eq!(max, 16);
        let (mut may, mut max) = (false, 5);
        slot_rules("test:none", 0, "test:fuel", 1, &mut may, &mut max);
        assert_eq!((may, max), (false, 5), "no window: the game's answer stays");

        let mut fuel = ItemSlot { item: Identifier::parse("test:fuel").unwrap(), count: 8 };
        let mut items = Vec::new();
        assert!(quick_move("test:oven", [0; 3], 40, 2, &mut fuel, &mut data, &mut items));
        assert_eq!((fuel.count, items[1].as_ref().map(|s| s.count)), (0, Some(8)));
        let mut stone = ItemSlot { item: Identifier::parse("test:stone").unwrap(), count: 8 };
        assert!(!quick_move("test:oven", [0; 3], 40, 2, &mut stone, &mut data, &mut items));

        assert!(accepts("test:oven", [0; 3], 0, "test:coal", 1, None));
        assert!(!accepts("test:oven", [0; 3], 0, "test:junk", 1, Some(Direction::Up)));
        assert!(accepts("test:none", [0; 3], 0, "test:junk", 1, None));
    }
}
