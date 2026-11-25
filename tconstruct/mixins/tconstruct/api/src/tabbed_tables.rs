//! The two things every tabbed table's window shares (TabbedContainerMenu and BaseTabbedScreen,
//! SlimeKnights, MIT; see LICENSE): the strip of tabs above the window, one per neighbouring
//! tabbed table, whose click switches the open menu to that table; and — on the crafting
//! station, the part builder and the modifier worktable — the side inventory module, which
//! shows the item inventory of a neighbouring container beside the window.
//!
//! The neighbouring tables are found by the original's search: a flood fill from the opened
//! block over tabbed blocks only, at most six, ordered by the sum of their coordinates, then
//! by height, then by x, then by z. The side inventory is the first container found scanning
//! the four horizontal neighbours in order, skipping the tables themselves; it is shown
//! through the menu's linked slots (its menu-links answer names the container, and the host
//! fills the linked range from it), in columns of nine rows, three to six columns wide.
//!
//! What does not carry over, and why:
//! - The original only accepts a neighbour whose block entity type is in the
//!   `tconstruct:side_inventories` tag (unless the config turns the whitelist off); a block
//!   entity type's tags are not among the snapshots a window's ticker can read, so any
//!   container the neighbours' reach exposes is accepted.
//! - A side inventory taller than the window scrolls behind a slider in the original; here
//!   every slot is positioned, so a tall one (a double chest is nine rows at three columns)
//!   draws past the window's lower edge.
//! - A menu switch is carried out for the game's own player, and for a LAN guest too: a guest's
//!   tab press runs through the same button dispatch, and the guest switch flush (on the guest's
//!   own station-button path, see the pluto-menu-switch module) opens the table in the host's
//!   guest state.
use pluto_lib::{
    BlockEntityTick, BlockPos, Direction, I18n, Identifier, ItemSlot, Nbt, Neighbours, ScreenCanvas,
};

/// The linked slots a side-inventory window declares: a double chest's worth, of which the
/// dynamic callback positions only as many as the named container holds.
pub(crate) const SIDE_SLOTS: usize = 54;
/// The tab strip's cap (TabbedContainerMenu's station-block limit).
pub(crate) const MAX_TABS: usize = 6;
/// The menu button id of the first table tab; the rest follow in strip order.
pub(crate) const TAB_BUTTON_BASE: i32 = 100;

/// The tabbed tables, in declaration order (the strip names them by index into this list).
pub(crate) const TABBED_BLOCKS: &[&str] = &[
    "tconstruct:crafting_station",
    "tconstruct:tinker_station",
    "tconstruct:tinkers_anvil",
    "tconstruct:scorched_anvil",
    "tconstruct:part_builder",
    "tconstruct:modifier_worktable",
    "tconstruct:tinker_chest",
];

/// The menu-data words the tabbed wiring appends at a caller-chosen base: the side
/// inventory's slot count (0 when there is none), its column count, its title code (see
/// [`title_code`]), the tab count, the open table's own index in the strip, then per tab its
/// block's index in [`TABBED_BLOCKS`] and its position relative to the opened block.
pub(crate) const DATA_SIDE_COUNT: usize = 0;
pub(crate) const DATA_SIDE_COLUMNS: usize = 1;
pub(crate) const DATA_SIDE_TITLE: usize = 2;
pub(crate) const DATA_TAB_COUNT: usize = 3;
pub(crate) const DATA_TAB_SELF: usize = 4;
pub(crate) const DATA_TABS: usize = 5;
/// How many menu-data words the wiring appends at its base.
pub(crate) const DATA_LEN: usize = DATA_TABS + MAX_TABS * 4;

/// The tab strip's geometry, relative to the window's origin: it sits above the window,
/// starting four pixels in; each tab is 26 wide and 30 tall, one pixel apart (TinkerTabsWidget).
const TAB_X: f32 = 4.0;
const TAB_Y: f32 = -30.0;
const TAB_W: f32 = 26.0;
const TAB_H: f32 = 30.0;
const TAB_STEP: f32 = 27.0;
/// The icons sheet's tab element (TAB_ELEMENT) and its active centre variant.
const TAB_SPRITE: [f32; 4] = [0.0, 18.0, 26.0, 30.0];
const TAB_SPRITE_ACTIVE: [f32; 4] = [52.0, 18.0, 26.0, 30.0];

/// The horizontal scan order for the side inventory (Direction.Plane.HORIZONTAL).
const HORIZONTAL: [Direction; 4] = [Direction::North, Direction::South, Direction::West, Direction::East];
/// All six directions, for the table flood fill.
const DIRECTIONS: [Direction; 6] = [
    Direction::North, Direction::South, Direction::West, Direction::East, Direction::Up, Direction::Down,
];

/// The block at a position, through the neighbours' reach.
fn block_at(pos: BlockPos) -> Option<Identifier> {
    Neighbours::block_state(pos).map(|state| state.block)
}

fn is_tabbed(block: &Identifier) -> bool {
    TABBED_BLOCKS.iter().any(|id| *id == block.to_string())
}

fn block_code(block: &Identifier) -> i32 {
    TABBED_BLOCKS.iter().position(|id| *id == block.to_string()).map(|index| index as i32).unwrap_or(0)
}

/// The neighbouring tabbed tables of the block at `pos`, itself first among equals after the
/// original's sort: flood fill over tabbed blocks, at most [`MAX_TABS`], ordered by coordinate
/// sum, then y, then x, then z (TabbedContainerMenu's stationBlocks).
pub(crate) fn station_blocks(pos: BlockPos) -> Vec<BlockPos> {
    let mut found: Vec<BlockPos> = Vec::new();
    let mut queue: Vec<BlockPos> = vec![pos];
    while let Some(at) = queue.first().copied() {
        queue.remove(0);
        if found.len() >= MAX_TABS || found.contains(&at) { continue; }
        let Some(block) = block_at(at) else { continue; };
        if !is_tabbed(&block) { continue; }
        found.push(at);
        for side in DIRECTIONS {
            let (next, _) = Neighbours::beside(at, side);
            if !found.contains(&next) && !queue.contains(&next) { queue.push(next); }
        }
    }
    found.sort_by_key(|at| (at[0] + at[1] + at[2], at[1], at[0], at[2]));
    found
}

/// The side inventory beside the block at `pos`: the first horizontal neighbour holding an
/// item inventory, skipping the tabbed tables themselves. Returns its position and how many
/// slots it holds (TabbedContainerMenu.addChestSideInventory's search).
pub(crate) fn side_inventory(pos: BlockPos, tables: &[BlockPos]) -> Option<(BlockPos, usize)> {
    for side in HORIZONTAL {
        let (at, back) = Neighbours::beside(pos, side);
        if tables.contains(&at) { continue; }
        if let Some(slots) = Neighbours::item_slots(at, back) {
            if !slots.is_empty() { return Some((at, slots.len())); }
        }
    }
    None
}

/// The containers a side-inventory window's linked slots show: its side inventory, if any.
pub(crate) fn links_answer(pos: BlockPos) -> Vec<BlockPos> {
    let tables = station_blocks(pos);
    match side_inventory(pos, &tables) {
        Some((at, _)) => vec![at],
        None => Vec::new(),
    }
}

/// The title a side inventory shows, as a code: the container names the original's screen
/// draws (a vanilla container's menu title, a Tinkers chest's block name). 0 draws no title.
fn title_code(block: &Identifier) -> i32 {
    match block.to_string().as_str() {
        "minecraft:chest" | "minecraft:trapped_chest" => 1,
        "minecraft:barrel" => 2,
        "minecraft:hopper" => 3,
        "minecraft:dispenser" => 4,
        "minecraft:dropper" => 5,
        "minecraft:furnace" => 6,
        "minecraft:blast_furnace" => 7,
        "minecraft:smoker" => 8,
        "minecraft:brewing_stand" => 9,
        "minecraft:shulker_box" => 10,
        "tconstruct:tinkers_chest" => 11,
        "tconstruct:part_chest" => 12,
        "tconstruct:cast_chest" => 13,
        _ => 0,
    }
}

fn title_key(code: i32) -> Option<&'static str> {
    Some(match code {
        1 => "container.chest",
        2 => "container.barrel",
        3 => "container.hopper",
        4 => "container.dispenser",
        5 => "container.dropper",
        6 => "container.furnace",
        7 => "container.blast_furnace",
        8 => "container.smoker",
        9 => "container.brewing_stand",
        10 => "container.shulkerBox",
        11 => "block.tconstruct.tinkers_chest",
        12 => "block.tconstruct.part_chest",
        13 => "block.tconstruct.cast_chest",
        _ => return None,
    })
}

/// The side inventory's column count for a slot count: a column per nine slots, at least
/// three, at most six (SideInventoryContainer's clamping).
fn side_columns(slots: usize) -> usize {
    ((slots.saturating_sub(1)) / 9 + 1).clamp(3, 6)
}

/// The side panel's left edge for a column count, relative to the window's origin: the panel
/// ends six pixels left of the window, each column is 18 wide with a seven-pixel border.
fn side_left(columns: usize) -> f32 {
    -(columns as f32 * 18.0 + 14.0) - 6.0
}

/// Where side slot `index` sits, from the published side words; `None` past the container's
/// slots (the linked range declares more than any one container holds).
pub(crate) fn side_slot_position(data: &[i32], base: usize, index: usize) -> Option<[f32; 2]> {
    let count = data.get(base + DATA_SIDE_COUNT).copied().unwrap_or(0).max(0) as usize;
    if index >= count { return None; }
    let columns = data.get(base + DATA_SIDE_COLUMNS).copied().unwrap_or(3).max(1) as usize;
    Some([
        side_left(columns) + 7.0 + (index % columns) as f32 * 18.0,
        17.0 + (index / columns) as f32 * 18.0,
    ])
}

/// The positions of a window's [`SIDE_SLOTS`] linked slots, for its dynamic-slot callback.
pub(crate) fn side_positions(data: &[i32], base: usize) -> Vec<Option<[f32; 2]>> {
    (0..SIDE_SLOTS).map(|index| side_slot_position(data, base, index)).collect()
}

/// Publishes the tabbed words at `base` for the window open at the ticking block: the tab
/// strip (always) and, when `side` holds, the side inventory's words. Also leaves the tabs'
/// absolute positions in the block entity's data, where the button callback reads the one a
/// pressed tab names. Runs inside the neighbours' scope, as every ticker does.
pub(crate) fn publish(tick: &mut BlockEntityTick, base: usize, side: bool) {
    let tables = station_blocks(tick.pos);
    let found = if side { side_inventory(tick.pos, &tables) } else { None };
    let (side_count, side_title) = match &found {
        Some((at, count)) => {
            let code = block_at(*at).map(|block| title_code(&block)).unwrap_or(0);
            (*count as i32, code)
        }
        None => (0, 0),
    };
    tick.set_menu_data(base + DATA_SIDE_COUNT, side_count);
    tick.set_menu_data(base + DATA_SIDE_COLUMNS, if side_count > 0 { side_columns(side_count as usize) as i32 } else { 0 });
    tick.set_menu_data(base + DATA_SIDE_TITLE, side_title);
    tick.set_menu_data(base + DATA_TAB_COUNT, tables.len() as i32);
    tick.set_menu_data(base + DATA_TAB_SELF, tables.iter().position(|at| *at == tick.pos).map(|index| index as i32).unwrap_or(-1));
    for (index, at) in tables.iter().enumerate() {
        let block = block_at(*at);
        let word = base + DATA_TABS + index * 4;
        tick.set_menu_data(word, block.map(|id| block_code(&id)).unwrap_or(0));
        tick.set_menu_data(word + 1, at[0] - tick.pos[0]);
        tick.set_menu_data(word + 2, at[1] - tick.pos[1]);
        tick.set_menu_data(word + 3, at[2] - tick.pos[2]);
        tick.data.insert(format!("tabbed_tab_{index}"), Nbt::IntArray(at.to_vec()));
    }
}

/// Draws the tab strip above the window: one tab per neighbouring table, its block's item as
/// the icon, the open table's tab in its active form. `icons` is the icons sheet's texture
/// name, as each window's own drawing names it.
pub(crate) fn draw_tabs(canvas: &mut dyn ScreenCanvas, origin: [f32; 2], data: &[i32], base: usize, icons: &str) {
    let count = data.get(base + DATA_TAB_COUNT).copied().unwrap_or(0).max(0) as usize;
    let own = data.get(base + DATA_TAB_SELF).copied().unwrap_or(-1);
    for index in 0..count.min(MAX_TABS) {
        let x = origin[0] + TAB_X + index as f32 * TAB_STEP;
        let y = origin[1] + TAB_Y;
        let sprite = if index as i32 == own { TAB_SPRITE_ACTIVE } else { TAB_SPRITE };
        canvas.sprite(icons, sprite, [x, y, TAB_W, TAB_H]);
        let code = data.get(base + DATA_TABS + index * 4).copied().unwrap_or(0);
        if let Some(id) = TABBED_BLOCKS.get(code.max(0) as usize) {
            if let Ok(item) = Identifier::parse(id) {
                canvas.item(&ItemSlot::new(item, 1), [x + 5.0, y + 7.0], 16.0);
            }
        }
    }
}

/// Draws the side inventory's module: its panel, its title, and a frame per shown slot (the
/// host's own frames sit under this draw, so the module draws its own).
pub(crate) fn draw_side_panel(canvas: &mut dyn ScreenCanvas, origin: [f32; 2], data: &[i32], base: usize) {
    let count = data.get(base + DATA_SIDE_COUNT).copied().unwrap_or(0).max(0) as usize;
    if count == 0 { return; }
    let columns = data.get(base + DATA_SIDE_COLUMNS).copied().unwrap_or(3).max(1) as usize;
    let rows = count.div_ceil(columns);
    let left = origin[0] + side_left(columns);
    let width = columns as f32 * 18.0 + 14.0;
    let height = rows as f32 * 18.0 + 24.0;
    canvas.fill([left, origin[1], width, height], 0xFFC6_C6C6);
    canvas.fill([left, origin[1], width, 1.0], 0xFFFF_FFFF);
    canvas.fill([left, origin[1], 1.0, height], 0xFFFF_FFFF);
    canvas.fill([left, origin[1] + height - 1.0, width, 1.0], 0xFF55_5555);
    canvas.fill([left + width - 1.0, origin[1], 1.0, height], 0xFF55_5555);
    if let Some(key) = title_key(data.get(base + DATA_SIDE_TITLE).copied().unwrap_or(0)) {
        canvas.text(&I18n::translate(key), [left + 7.0, origin[1] + 5.0], 0xFF40_4040, false);
    }
    for index in 0..count {
        let Some(at) = side_slot_position(data, base, index) else { continue; };
        let (x, y) = (origin[0] + at[0] - 1.0, origin[1] + at[1] - 1.0);
        canvas.fill([x, y, 18.0, 18.0], 0xFF8B_8B8B);
        canvas.fill([x, y, 18.0, 1.0], 0xFF37_3737);
        canvas.fill([x, y, 1.0, 18.0], 0xFF37_3737);
        canvas.fill([x, y + 17.0, 18.0, 1.0], 0xFFFF_FFFF);
        canvas.fill([x + 17.0, y, 1.0, 18.0], 0xFFFF_FFFF);
    }
}

/// The tab a window-relative point falls on, if any.
pub(crate) fn tab_at(at: [f32; 2], data: &[i32], base: usize) -> Option<usize> {
    if at[1] < TAB_Y || at[1] >= TAB_Y + TAB_H { return None; }
    let count = data.get(base + DATA_TAB_COUNT).copied().unwrap_or(0).max(0) as usize;
    (0..count.min(MAX_TABS)).find(|index| {
        let x = TAB_X + *index as f32 * TAB_STEP;
        at[0] >= x && at[0] < x + TAB_W
    })
}

/// The menu button a click at a window-relative point asks for: a tab's button, if the point
/// is on a tab.
pub(crate) fn tab_click(at: [f32; 2], data: &[i32], base: usize) -> Option<i32> {
    tab_at(at, data, base).map(|index| TAB_BUTTON_BASE + index as i32)
}

/// The name a hovered tab shows, if the point is on one: the table's block name.
pub(crate) fn tab_tooltip(at: [f32; 2], data: &[i32], base: usize) -> Option<String> {
    let index = tab_at(at, data, base)?;
    let code = data.get(base + DATA_TABS + index * 4).copied().unwrap_or(0);
    let id = TABBED_BLOCKS.get(code.max(0) as usize)?;
    Some(I18n::translate(&format!("block.{}", id.replace(':', "."))))
}

/// Answers a pressed tab's button: switches the pressing player's menu to that tab's table.
/// Returns whether the press was a tab's.
pub(crate) fn press_tab(press: &mut pluto_lib::MenuButton<'_>) -> bool {
    let index = press.id - TAB_BUTTON_BASE;
    if !(0..MAX_TABS as i32).contains(&index) { return false; }
    let Some(Nbt::IntArray(at)) = press.data.get(&format!("tabbed_tab_{index}")) else { return false; };
    let Ok(at) = <[i32; 3]>::try_from(at.as_slice()) else { return false; };
    press.open_menu(at);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_columns_clamp_between_three_and_six() {
        assert_eq!(side_columns(1), 3);
        assert_eq!(side_columns(27), 3, "a single chest: three columns of nine");
        assert_eq!(side_columns(28), 4);
        assert_eq!(side_columns(54), 6, "a double chest: six columns of nine");
        assert_eq!(side_columns(256), 6);
    }

    #[test]
    fn side_slots_fill_the_panel_in_columns_and_hide_past_the_count() {
        let mut data = vec![0; DATA_LEN];
        data[DATA_SIDE_COUNT] = 27;
        data[DATA_SIDE_COLUMNS] = 3;
        assert_eq!(side_slot_position(&data, 0, 0), Some([side_left(3) + 7.0, 17.0]));
        assert_eq!(side_slot_position(&data, 0, 3), Some([side_left(3) + 7.0, 35.0]), "the fourth slot starts the second row of a three-column panel");
        assert_eq!(side_slot_position(&data, 0, 26), Some([side_left(3) + 7.0 + 36.0, 17.0 + 8.0 * 18.0]));
        assert_eq!(side_slot_position(&data, 0, 27), None, "past the container's slots");
        assert_eq!(side_positions(&data, 0).len(), SIDE_SLOTS);
    }

    #[test]
    fn tabs_hit_test_the_strip_above_the_window() {
        let mut data = vec![0; DATA_LEN];
        data[DATA_TAB_COUNT] = 2;
        assert_eq!(tab_at([4.0, -30.0], &data, 0), Some(0));
        assert_eq!(tab_at([4.0 + 27.0 + 25.0, -1.0], &data, 0), Some(1));
        assert_eq!(tab_at([4.0 + 54.0, -15.0], &data, 0), None, "past the second tab");
        assert_eq!(tab_at([10.0, 0.0], &data, 0), None, "the strip is above the window");
        assert_eq!(tab_click([10.0, -15.0], &data, 0), Some(TAB_BUTTON_BASE));
    }
}
