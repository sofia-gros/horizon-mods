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
    /// Draws the part `src` (in texture pixels) of the GUI texture `name`, a sheet of `sheet` pixels, into `dst`, tinted by `argb`:
    /// `sprite` with a colour, for the translucent shades and covers of the original screens. False if there is no such texture.
    fn sprite_tinted(&mut self, name: &str, src: [f32; 4], sheet: [f32; 2], dst: [f32; 4], argb: u32) -> bool {
        let _ = (name, src, sheet, dst, argb);
        false
    }
    /// Draws the block/item atlas sprite `name` (such as `tconstruct:gui/tinker_pattern/pick_head`) stretched into `dst`, tinted
    /// by `argb`. The host gives no answer for a missing sprite: an unknown name draws nothing.
    fn atlas_sprite(&mut self, name: &str, dst: [f32; 4], argb: u32) {
        let _ = (name, dst, argb);
    }
    /// Draws `stack`'s item icon with its top left at `at`, `size` GUI pixels square, as the game draws it in a slot (model and
    /// decorations, without the count). False when nothing was drawn (an unknown item or an empty stack).
    fn item(&mut self, stack: &ItemSlot, at: [f32; 2], size: f32) -> bool {
        let _ = (stack, at, size);
        false
    }
    /// Draws `text` with its top left at `at`, in colour `rgb`, with a shadow if asked.
    fn text(&mut self, text: &str, at: [f32; 2], rgb: u32, shadow: bool);
    /// The width of `text` in GUI pixels.
    fn text_width(&self, text: &str) -> f32;
}

/// Where things are in a window, in GUI pixels from its top left (Minecraft: `Slot.x`, `Slot.y`).
///
/// A container slot is in the menu in one of two ways: at the position `slots` gives it, or — when
/// the matching entry of `dynamic` is set — at the position the window's dynamic-slot callback
/// (`ScreenHandlerType::on_dynamic_slots`) gives it for the menu's current state. A slot that is
/// neither is hidden: it is not in the menu at all, on the server either, so no click reaches it and
/// shift-clicks skip it.
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenLayout {
    /// The window's width; vanilla container windows are 176.
    pub width: f32,
    /// The window's height.
    pub height: f32,
    /// Where each container slot is; `None` leaves the slot's place to `dynamic` (or hides the slot).
    pub slots: Vec<Option<[f32; 2]>>,
    /// Which slots the dynamic-slot callback positions: such a slot is in the menu though the static
    /// layout has no position for it. Parallel to `slots`, one entry per slot.
    pub dynamic: Vec<bool>,
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
        Self { width, height, slots: Vec::new(), dynamic: Vec::new(), inventory_x: 8.0, inventory_y: 84.0, own_labels: false, own_background: false }
    }

    /// The same layout with one more container slot at `(x, y)`.
    pub fn slot(mut self, x: f32, y: f32) -> Self {
        self.slots.push(Some([x, y]));
        self.dynamic.push(false);
        self
    }

    /// The same layout with one more container slot that is hidden: it is not in the menu at all, so
    /// the server refuses clicks on it and its item is never shown. For a slot whose place depends
    /// on the menu's state, use `dynamic_slot` instead.
    pub fn hidden_slot(mut self) -> Self {
        self.slots.push(None);
        self.dynamic.push(false);
        self
    }

    /// The same layout with one more container slot the dynamic-slot callback positions
    /// (`ScreenHandlerType::on_dynamic_slots`): the slot is in the menu — the server takes clicks on
    /// it and its item syncs like any other's — though the static layout has no position for it. Use
    /// this for slots whose place follows the menu's data or items (a tool station's inputs, a paged
    /// chest's grid).
    pub fn dynamic_slot(mut self) -> Self {
        self.slots.push(None);
        self.dynamic.push(true);
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

/// The state a dynamic screen-slot layout can inspect. `items` are the menu's container slots only, in slot order; `data` are its menu data slots.
pub struct ScreenLayoutContext<'a> {
    /// The menu's data slots.
    pub data: &'a [i32],
    /// The block entity's current item slots, in menu order.
    pub items: &'a [Option<ItemSlot>],
}

/// What the draw callback receives.
pub struct ScreenDraw<'a> {
    /// The window's top left in GUI pixels.
    pub origin: [f32; 2],
    /// The menu's data slots (what the block entity set); empty until the server's first values arrive, so read it with `get`.
    pub data: &'a [i32],
    /// The cursor's position in GUI pixels from the window's top left (outside it too: a sidebar
    /// left of the window reads negative x). The original's screens draw hover faces and fluid
    /// highlights from it.
    pub mouse: [f32; 2],
    /// The menu's container slots as the client last saw them synced, in slot order (the block
    /// entity's item slots only, not the player's). Empty until the first sync arrives, and on a
    /// screen whose menu never synced items; read it with `get`. The window's own server-side
    /// state stays in the menu data — this is the client's view for drawing (side info panels
    /// that read the tool in the slots, say), never an authority.
    pub items: &'a [Option<ItemSlot>],
    /// Where to draw.
    pub canvas: &'a mut dyn ScreenCanvas,
}

/// What the scroll callback receives: the mouse wheel turned over the window. Its `at` is where
/// the cursor is, its `delta` the wheel's movement, and its `data` the menu's data slots. Scroll
/// state is the window's own (client-side) business — no scroll reaches the server — so a window
/// whose list scrolls keeps the offset itself, fed by its draw callback's view of the menu data.
pub struct ScreenScroll<'a> {
    /// Where the cursor is, in GUI pixels from the window's top left.
    pub at: [f32; 2],
    /// The wheel's movement, in notches: a positive y is a roll up (toward the user at the top
    /// of the wheel, Minecraft's `mouseScrolled` sign flipped to "up is positive").
    pub delta: [f32; 2],
    /// The menu's data slots.
    pub data: &'a [i32],
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

impl MenuButton<'_> {
    /// Sets menu data slot `index` (0..256); the content menu must declare it. Values are clamped to the signed 16-bit wire range.
    pub fn set_data(&mut self, index: usize, value: i32) {
        super::block_entity_api_v1::set_menu_data(self.data, index, value);
    }

    /// The fluid in the block entity's tank `index`, if it holds any (tanks are declared with
    /// `BlockEntityType::fluid_tanks`): a tank a player clicked with a held container is filled
    /// or emptied here, as the original's `clickMenuButton` does.
    pub fn fluid(&self, index: usize) -> Option<super::block_entity_api_v1::FluidStack> {
        super::block_entity_api_v1::read_fluid(self.data, index)
    }

    /// Puts `stack` in the block entity's tank `index`, or empties the tank with `None`.
    pub fn set_fluid(&mut self, index: usize, stack: Option<super::block_entity_api_v1::FluidStack>) {
        super::block_entity_api_v1::write_fluid(self.data, index, stack);
    }
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

/// A slot operation performed inside the menu engine, before its state is published.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotAction { Pickup, QuickMove, Swap, Throw }

/// Access to the current menu's staged state. Transfers obey the host's slot
/// acceptance/component/stack-size rules; they do not write the live inventory.
pub trait MenuAccess {
    fn slot_count(&self) -> usize;
    fn stack(&self, slot: usize) -> Option<ItemSlot>;
    /// False for an out-of-range slot. A zero-count stack is empty.
    fn set_stack(&mut self, slot: usize, stack: Option<ItemSlot>) -> bool;
    /// The swap target: hotbar 0..=8 or offhand 40, not a menu-slot index.
    fn swap_stack(&self, button: i32) -> Option<ItemSlot>;
    /// False for any other button. A zero-count stack clears the target.
    fn set_swap_stack(&mut self, button: i32, stack: Option<ItemSlot>) -> bool;
    fn carried(&self) -> Option<ItemSlot>;
    fn set_carried(&mut self, stack: Option<ItemSlot>);
    /// Native item-and-component equality, ignoring count; false for empty stacks.
    /// This does not check remaining capacity or slot acceptance.
    fn same_item_and_components(&self, left: &ItemSlot, right: &ItemSlot) -> bool;
    /// Native stack component/item limit, or zero for an empty stack.
    /// A slot may impose a lower limit; this is the item stack's own limit.
    fn max_stack_count(&self, stack: &ItemSlot) -> u32;
    /// Move into [start, end), leaving the remainder in `stack`.
    fn move_stack(&mut self, stack: &mut ItemSlot, start: usize, end: usize, reverse: bool) -> bool;
    /// The native item's crafting remainder (not its use/consumption remainder).
    /// None for an empty/unknown item or an item with no crafting remainder.
    fn crafting_remainder(&self, stack: &ItemSlot) -> Option<ItemSlot>;
    /// Return an owned stack through native player-inventory insertion, queuing
    /// overflow as a drop. Both inventory and drops remain staged until commit.
    fn give_stack(&mut self, stack: ItemSlot);
    /// Stage a drop; the host publishes it only through its normal menu commit.
    fn drop_stack(&mut self, stack: &ItemSlot);
}

/// A registered slot operation owns this click. Unregistered slots retain the
/// native behavior. No live player/world mutation scope is installed here.
pub struct SlotClick<'a> {
    pub pos: BlockPos,
    pub player: &'a super::player_api::ServerPlayer,
    /// False during client prediction; never use prediction for server-only effects.
    pub authoritative: bool,
    pub slot: usize,
    pub button: i32,
    pub action: SlotAction,
    pub data: &'a mut NbtCompound,
    pub menu: &'a mut dyn MenuAccess,
}

pub(crate) type SlotClickHandler = Arc<dyn Fn(&mut SlotClick<'_>) + Send + Sync>;

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
type DynamicSlots = Arc<dyn Fn(&ScreenLayoutContext<'_>) -> Option<Vec<Option<[f32; 2]>>> + Send + Sync>;
type Click = Arc<dyn Fn(&ScreenClick<'_>) -> Option<i32> + Send + Sync>;
type Scroll = Arc<dyn Fn(&mut ScreenScroll<'_>) -> bool + Send + Sync>;
type Char = Arc<dyn Fn(char) -> bool + Send + Sync>;
type Key = Arc<dyn Fn(&str) -> bool + Send + Sync>;
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
    dynamic_slots: Option<DynamicSlots>,
    click: Option<Click>,
    scroll: Option<Scroll>,
    char_typed: Option<Char>,
    key_pressed: Option<Key>,
    tooltip: Option<Tooltip>,
    button: Option<Button>,
    rule: Option<Rule>,
    quick_move: Option<Move>,
    accepts: Option<Accept>,
    slot_clicks: HashMap<(usize, SlotAction), SlotClickHandler>,
    pick_all_exclusions: std::collections::HashSet<usize>,
}

impl ScreenHandlerType {
    /// A window for `block`'s menu with the given layout and nothing drawn beyond the game's panel.
    pub fn new(block: Identifier, layout: ScreenLayout) -> Self {
        Self { block, layout, draw: None, dynamic_slots: None, click: None, scroll: None, char_typed: None, key_pressed: None, tooltip: None, button: None, rule: None, quick_move: None, accepts: None, slot_clicks: HashMap::new(), pick_all_exclusions: std::collections::HashSet::new() }
    }

    /// The same window, `draw` drawing it each frame over the panel and the slot frames and under the items (Fabric: `HandledScreen.drawBackground`).
    pub fn on_draw(mut self, draw: impl Fn(&mut ScreenDraw<'_>) + Send + Sync + 'static) -> Self {
        self.draw = Some(Arc::new(draw));
        self
    }

    /// The same window, `dynamic_slots` choosing the container-slot positions from the current menu data and item slots. Return `None` to
    /// keep the static positions; return exactly one position per container slot to update them.
    pub fn on_dynamic_slots(mut self, dynamic_slots: impl Fn(&ScreenLayoutContext<'_>) -> Option<Vec<Option<[f32; 2]>>> + Send + Sync + 'static) -> Self {
        self.dynamic_slots = Some(Arc::new(dynamic_slots));
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

    /// The same window, `scroll` hearing the mouse wheel over the window (client-side, while this
    /// window is the open one). Answer `true` to consume the roll (the game's own scrolling —
    /// hotbar selection with no screen, recipe book paging — is then skipped). Nothing a scroll
    /// does reaches the server: a window that scrolls a server-published list keeps the offset
    /// itself and windows the list in its draw, click and tooltip callbacks.
    pub fn on_scroll(mut self, scroll: impl Fn(&mut ScreenScroll<'_>) -> bool + Send + Sync + 'static) -> Self {
        self.scroll = Some(Arc::new(scroll));
        self
    }

    /// The same window, `char_typed` hearing each character typed while this window is the open
    /// one (a text field's input). Answer `true` to consume the character. Key presses that are
    /// not characters (backspace, arrows, enter, escape) arrive through [`Self::on_key`]. The
    /// text a field collects is the window's own client-side state; the surface carries input,
    /// not a channel to the server.
    pub fn on_char(mut self, char_typed: impl Fn(char) -> bool + Send + Sync + 'static) -> Self {
        self.char_typed = Some(Arc::new(char_typed));
        self
    }

    /// The same window, `key_pressed` hearing each named key press while this window is the open
    /// one (the game's own key names, lower-cased: `backspace`, `enter`, `escape`, `left`, ...).
    /// Answer `true` to consume the press. See [`Self::on_char`] for character input.
    pub fn on_key(mut self, key_pressed: impl Fn(&str) -> bool + Send + Sync + 'static) -> Self {
        self.key_pressed = Some(Arc::new(key_pressed));
        self
    }

    /// The same window, `button` running on the server when a player presses a menu button (Minecraft: `clickMenuButton`).
    pub fn on_button(mut self, button: impl Fn(&mut MenuButton<'_>) + Send + Sync + 'static) -> Self {
        self.button = Some(Arc::new(button));
        self
    }
}

impl ScreenHandlerType {
    /// Exclude a slot from double-click collection and shift-double-click scanning.
    /// Use for lazy recipe previews: collecting them must never bypass crafting.
    /// This only restricts the native decision; it cannot enable a refused slot.
    pub fn exclude_from_pick_all(mut self, slot: usize) -> Self {
        self.pick_all_exclusions.insert(slot);
        self
    }

    /// Handle one operation on one menu slot; unregistered operations remain native.
    /// The callback owns that operation and must implement its recipe semantics.
    /// Drag, clone and pick-all remain native. This is not by itself a lazy
    /// crafting-result implementation. Native integration is under R2 validation.
    pub fn on_slot_click(mut self, slot: usize, action: SlotAction, handler: impl Fn(&mut SlotClick<'_>) + Send + Sync + 'static) -> Self {
        self.slot_clicks.insert((slot, action), Arc::new(handler));
        self
    }

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

/// Whether a registered screen has opted into dynamic container-slot positions.
pub(crate) fn has_dynamic_slots(block: &str) -> bool {
    ScreenHandlerRegistry::get(block).is_some_and(|handler| handler.dynamic_slots.is_some())
}

/// Resolves dynamic container-slot positions for the current menu state. Invalid slot counts leave the registered static layout intact.
pub(crate) fn dynamic_slots(block: &str, data: &[i32], items: &[Option<ItemSlot>]) -> Option<Vec<Option<[f32; 2]>>> {
    let handler = ScreenHandlerRegistry::get(block)?;
    let resolve = handler.dynamic_slots?;
    let positions = resolve(&ScreenLayoutContext { data, items });
    Some(positions.filter(|slots| slots.len() == handler.layout.slots.len() && slots.iter().flatten().all(|p| p.iter().all(|v| v.is_finite())))
        .unwrap_or(handler.layout.slots))
}

/// The layout for a block, if it has a window.
pub(crate) fn layout(block: &str) -> Option<ScreenLayout> {
    ScreenHandlerRegistry::get(block).map(|h| h.layout)
}

/// The state the client last saw synced for each block's open menu: its menu data and its
/// container items, in slot order. The dynamic-slot refresh (which runs on the client whenever
/// a menu's slots or data change) records it; the draw and scroll callbacks read it back.
/// Client-side state only: the server never sees this map, and a menu that never refreshed
/// simply draws with no items.
static MENU_STATE: Mutex<Option<HashMap<String, (Vec<i32>, Vec<Option<ItemSlot>>)>>> = Mutex::new(None);

/// Records the state of the menu of `block` the client just saw (see [`MENU_STATE`]).
pub(crate) fn note_menu_state(block: &str, data: &[i32], items: &[Option<ItemSlot>]) {
    if let Ok(mut all) = MENU_STATE.lock() {
        all.get_or_insert_with(HashMap::new).insert(block.to_owned(), (data.to_vec(), items.to_vec()));
    }
}

/// Forgets the recorded state of `block`'s menu (its screen closed).
pub(crate) fn forget_menu_state(block: &str) {
    if let Ok(mut all) = MENU_STATE.lock() {
        if let Some(all) = all.as_mut() {
            all.remove(block);
        }
    }
}

/// The recorded state of `block`'s open menu, if the client has seen a sync of it.
fn menu_state(block: &str) -> Option<(Vec<i32>, Vec<Option<ItemSlot>>)> {
    MENU_STATE.lock().ok().and_then(|all| all.as_ref().and_then(|all| all.get(block).cloned()))
}

/// Draws the block's window; false if it has none or draws nothing. `mouse` is the cursor in GUI
/// pixels (absolute); the callback receives it from the window's top left.
pub(crate) fn draw(block: &str, origin: [f32; 2], data: &[i32], mouse: [f32; 2], canvas: &mut dyn ScreenCanvas) -> bool {
    match ScreenHandlerRegistry::get(block).and_then(|h| h.draw) {
        Some(draw) => {
            let items = menu_state(block).map_or_else(Vec::new, |(_, items)| items);
            draw(&mut ScreenDraw { origin, data, mouse: [mouse[0] - origin[0], mouse[1] - origin[1]], items: &items, canvas });
            true
        }
        None => false,
    }
}

/// Whether the block's window hears the mouse wheel.
pub(crate) fn has_scroll(block: &str) -> bool {
    ScreenHandlerRegistry::get(block).is_some_and(|h| h.scroll.is_some())
}

/// Offers a wheel roll over the block's window; true when the window consumed it. The menu
/// data the callback reads is the state the client last saw synced (see [`MENU_STATE`]).
pub(crate) fn scroll(block: &str, at: [f32; 2], delta: [f32; 2]) -> bool {
    match ScreenHandlerRegistry::get(block).and_then(|h| h.scroll) {
        Some(scroll) => {
            let data = menu_state(block).map_or_else(Vec::new, |(data, _)| data);
            scroll(&mut ScreenScroll { at, delta, data: &data })
        }
        None => false,
    }
}

/// Whether the block's window hears typed characters or key presses.
pub(crate) fn has_text_input(block: &str) -> bool {
    ScreenHandlerRegistry::get(block).is_some_and(|h| h.char_typed.is_some() || h.key_pressed.is_some())
}

/// Offers a typed character to the block's window; true when the window consumed it.
pub(crate) fn char_typed(block: &str, c: char) -> bool {
    match ScreenHandlerRegistry::get(block).and_then(|h| h.char_typed) {
        Some(char_typed) => char_typed(c),
        None => false,
    }
}

/// Offers a key press to the block's window; true when the window consumed it.
pub(crate) fn key_pressed(block: &str, key: &str) -> bool {
    match ScreenHandlerRegistry::get(block).and_then(|h| h.key_pressed) {
        Some(key_pressed) => key_pressed(key),
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

pub(crate) fn excluded_from_pick_all(block: &str, slot: usize) -> bool {
    ScreenHandlerRegistry::get(block).is_some_and(|handler| handler.pick_all_exclusions.contains(&slot))
}

pub(crate) fn slot_click(block: &str, slot: usize, action: SlotAction) -> Option<SlotClickHandler> {
    ScreenHandlerRegistry::get(block)?.slot_clicks.get(&(slot, action)).cloned()
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
    fn pick_all_exclusions_apply_only_to_declared_slots_and_handlers() {
        let block = Identifier::parse("test:pick_all_result").unwrap();
        ScreenHandlerRegistry::register(
            ScreenHandlerType::new(block, ScreenLayout::new(176.0, 166.0))
                .exclude_from_pick_all(1),
        );
        assert!(excluded_from_pick_all("test:pick_all_result", 1));
        assert!(!excluded_from_pick_all("test:pick_all_result", 0));
        assert!(!excluded_from_pick_all("test:pick_all_unregistered", 1));
        ScreenHandlerRegistry::register(ScreenHandlerType::new(
            Identifier::parse("test:pick_all_storage").unwrap(),
            ScreenLayout::new(176.0, 166.0),
        ));
        assert!(!excluded_from_pick_all("test:pick_all_storage", 1));
    }

    #[test]
    fn dynamic_slot_positions_follow_current_container_items_without_changing_the_static_layout() {
        let block = Identifier::parse("test:dynamic_chest").unwrap();
        ScreenHandlerRegistry::register(
            ScreenHandlerType::new(block, ScreenLayout::new(176.0, 166.0).slot(8.0, 8.0).slot(26.0, 8.0))
                .on_dynamic_slots(|state| {
                    match state.data.first() {
                        Some(1) => return None,
                        Some(2) => return Some(vec![]),
                        Some(3) => return Some(vec![Some([f32::NAN, 0.0]), None]),
                        _ => {}
                    }
                    let first = state.items.first().and_then(Option::as_ref).map(|stack| stack.item.to_string());
                    Some(if first.as_deref() == Some("test:wide_part") {
                        vec![Some([8.0, 44.0]), Some([44.0, 8.0])]
                    } else {
                        vec![Some([8.0, 8.0]), Some([26.0, 8.0])]
                    })
                }),
        );
        assert!(has_dynamic_slots("test:dynamic_chest"));
        let item = ItemSlot::new(Identifier::parse("test:wide_part").unwrap(), 1);
        assert_eq!(dynamic_slots("test:dynamic_chest", &[], &[Some(item)]), Some(vec![Some([8.0, 44.0]), Some([44.0, 8.0])]));
        assert_eq!(layout("test:dynamic_chest").unwrap().slots, vec![Some([8.0, 8.0]), Some([26.0, 8.0])]);
        assert_eq!(dynamic_slots("test:dynamic_chest", &[], &[None]), Some(vec![Some([8.0, 8.0]), Some([26.0, 8.0])]));
        for data in [1, 2, 3] {
            assert_eq!(dynamic_slots("test:dynamic_chest", &[data], &[]), Some(vec![Some([8.0, 8.0]), Some([26.0, 8.0])]));
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
                    b.set_data(0, b.id);
                    b.set_data(1, 100_000);
                    b.set_data(256, 4);
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
        assert!(draw("test:oven", [100.0, 50.0], &[30], [100.0, 50.0], &mut canvas));
        assert_eq!(canvas.fills, vec![([110.0, 60.0, 30.0, 4.0], 0xFFFF0000)]);
        assert_eq!(canvas.texts, ["Oven"]);
        assert!(!draw("test:none", [0.0; 2], &[], [0.0; 2], &mut canvas));

        assert_eq!(click("test:oven", [120.0, 10.0], false, &[]), Some(7));
        assert_eq!(click("test:oven", [120.0, 10.0], true, &[]), None);
        assert_eq!(click("test:oven", [5.0, 10.0], false, &[]), None);

        let mut lines = Vec::new();
        tooltip("test:oven", [5.0, 5.0], &[42], &mut lines);
        assert_eq!(lines, ["heat 42"]);

        let mut data = NbtCompound::new();
        let mut items = Vec::new();
        let mut carried = Some(ItemSlot::new(Identifier::parse("test:fuel").unwrap(), 3));
        assert!(press("test:oven", [1, 2, 3], 7, &mut data, &mut items, &mut carried));
        assert_eq!(data.get_int("pressed", 0), 7);
        assert_eq!(data.get("pluto:menu_data"), Some(&Nbt::IntArray(vec![7, 32767])));
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

        let mut fuel = ItemSlot::new(Identifier::parse("test:fuel").unwrap(), 8);
        let mut items = Vec::new();
        assert!(quick_move("test:oven", [0; 3], 40, 2, &mut fuel, &mut data, &mut items));
        assert_eq!((fuel.count, items[1].as_ref().map(|s| s.count)), (0, Some(8)));
        let mut stone = ItemSlot::new(Identifier::parse("test:stone").unwrap(), 8);
        assert!(!quick_move("test:oven", [0; 3], 40, 2, &mut stone, &mut data, &mut items));

        assert!(accepts("test:oven", [0; 3], 0, "test:coal", 1, None));
        assert!(!accepts("test:oven", [0; 3], 0, "test:junk", 1, Some(Direction::Up)));
        assert!(accepts("test:none", [0; 3], 0, "test:junk", 1, None));
    }

    #[test]
    fn draw_sees_the_cursor_and_the_synced_items_and_scroll_and_text_input_reach_the_window() {
        use std::sync::Mutex as StdMutex;
        let seen: Arc<StdMutex<Vec<String>>> = Arc::new(StdMutex::new(Vec::new()));
        let record = seen.clone();
        ScreenHandlerRegistry::register(
            ScreenHandlerType::new(Identifier::parse("test:panel").unwrap(), ScreenLayout::new(176.0, 166.0))
                .on_draw(move |d| {
                    record.lock().unwrap().push(format!("mouse {:?} items {}", d.mouse, d.items.len()));
                })
                .on_scroll(|s| s.delta[1] > 0.0)
                .on_char(|c| c == 'a')
                .on_key(|key| key == "backspace"),
        );
        let mut canvas = Recorder::default();
        // No sync seen yet: the draw runs with no items and the cursor relative to the window.
        assert!(draw("test:panel", [100.0, 50.0], &[], [130.0, 90.0], &mut canvas));
        assert_eq!(seen.lock().unwrap().as_slice(), ["mouse [30.0, 40.0] items 0"]);
        // After a sync the recorded items are what the draw reads; forgetting empties them again.
        let item = ItemSlot::new(Identifier::parse("test:ingot").unwrap(), 3);
        note_menu_state("test:panel", &[7], &[Some(item), None]);
        assert!(draw("test:panel", [100.0, 50.0], &[], [100.0, 50.0], &mut canvas));
        assert_eq!(seen.lock().unwrap().last().unwrap(), "mouse [0.0, 0.0] items 2");
        forget_menu_state("test:panel");
        assert!(draw("test:panel", [100.0, 50.0], &[], [100.0, 50.0], &mut canvas));
        assert_eq!(seen.lock().unwrap().last().unwrap(), "mouse [0.0, 0.0] items 0");

        assert!(has_scroll("test:panel"));
        assert!(scroll("test:panel", [0.0; 2], [0.0, 1.0]), "a roll up is consumed");
        assert!(!scroll("test:panel", [0.0; 2], [0.0, -1.0]), "a roll down is not");
        assert!(!has_scroll("test:none"));
        assert!(has_text_input("test:panel"));
        assert!(char_typed("test:panel", 'a'));
        assert!(!char_typed("test:panel", 'b'));
        assert!(key_pressed("test:panel", "backspace"));
        assert!(!key_pressed("test:panel", "enter"));
        assert!(!has_text_input("test:none"));
    }

    #[test]
    fn a_menu_button_reads_and_writes_the_block_entity_tanks() {
        ScreenHandlerRegistry::register(
            ScreenHandlerType::new(Identifier::parse("test:tank_button").unwrap(), ScreenLayout::new(176.0, 166.0))
                .on_button(|b| {
                    let held = b.fluid(0).map_or(0, |stack| stack.amount);
                    b.set_fluid(0, Some(crate::pure::block_entity_api_v1::FluidStack::new(Identifier::parse("minecraft:water").unwrap(), held + 250)));
                }),
        );
        let mut data = NbtCompound::new();
        let mut items = Vec::new();
        let mut carried = None;
        assert!(press("test:tank_button", [0; 3], 1, &mut data, &mut items, &mut carried));
        let tank = crate::pure::block_entity_api_v1::read_fluid(&data, 0).expect("the button filled the tank");
        assert_eq!((tank.fluid.to_string().as_str(), tank.amount), ("minecraft:water", 250));
    }

    #[test]
    fn dynamic_slots_are_in_the_menu_without_a_static_position_and_hidden_ones_are_not() {
        let declared = ScreenLayout::new(176.0, 166.0).slot(8.0, 8.0).dynamic_slot().hidden_slot();
        assert_eq!(declared.slots, vec![Some([8.0, 8.0]), None, None], "neither kind has a static position");
        assert_eq!(declared.dynamic, vec![false, true, false], "only the dynamic slot is in the menu without one");
        ScreenHandlerRegistry::register(ScreenHandlerType::new(Identifier::parse("test:dynamic_layout").unwrap(), declared));
        let layout = layout("test:dynamic_layout").unwrap();
        assert_eq!(layout.dynamic, vec![false, true, false], "the registry hands the declaration back whole");
    }
}
