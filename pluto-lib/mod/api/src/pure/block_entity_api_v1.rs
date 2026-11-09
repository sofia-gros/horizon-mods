//! Block entities for a mod's blocks: data that belongs to a block position, saved with the chunk, ticked every game tick on the server
//! and optionally sent to clients (Minecraft: `BlockEntity`, `BlockEntityType`, `BlockEntityTicker`; Fabric:
//! `FabricBlockEntityTypeBuilder`).
//!
//! The block itself comes from the mod's content (its blocks and models). Here a block gets its block entity type: the data it starts
//! with, what it does each tick, and what clients are told of it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::base::{Identifier, Nbt, NbtCompound};

/// A block position `[x, y, z]`.
pub type BlockPos = [i32; 3];

/// One stack in an item slot: the item, the count, and what a mod keeps on the stack. A stack whose item is not changed keeps all of its
/// other data when only the count is changed.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemSlot {
    /// The item.
    pub item: Identifier,
    /// How many.
    pub count: u32,
    /// Damage taken, 0 for a new item.
    pub damage: u32,
    /// The most damage the stack takes before it breaks, when it differs from the item's (the `minecraft:max_damage` component).
    pub max_damage: Option<i32>,
    /// The stack's `minecraft:custom_data`, empty for none.
    pub custom_data: NbtCompound,
}

impl ItemSlot {
    /// A plain stack of `count` of `item`, with no damage and no data.
    pub fn new(item: Identifier, count: u32) -> Self {
        Self { item, count, damage: 0, max_damage: None, custom_data: NbtCompound::new() }
    }
}

/// Fluid in a tank: which fluid and how many millibuckets (a bucket is a thousand).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FluidStack {
    /// The fluid, such as `minecraft:water` or a mod's own.
    pub fluid: Identifier,
    /// Millibuckets.
    pub amount: u32,
}

/// What a ticker receives: where the block entity is, its saved data and its item slots, which the ticker may change.
pub struct BlockEntityTick<'a> {
    /// The block entity's position.
    pub pos: BlockPos,
    /// The block entity's saved data, without the position and the id; changes are saved with the chunk.
    pub data: &'a mut NbtCompound,
    /// The item slots, as many as the type declared (none if it declared none); an empty slot is `None`.
    pub items: &'a mut Vec<Option<ItemSlot>>,
}

/// Stores one menu data slot in saved data; Horizon synchronizes it to viewers of the content menu.
pub(crate) fn set_menu_data(data: &mut NbtCompound, index: usize, value: i32) {
    if index >= 256 { return; }
    let slots = data.entry("pluto:menu_data".to_owned()).or_insert_with(|| Nbt::IntArray(Vec::new()));
    if let Nbt::IntArray(slots) = slots {
        slots.resize(slots.len().max(index + 1), 0);
        slots[index] = value.clamp(i16::MIN as i32, i16::MAX as i32);
    }
}

impl BlockEntityTick<'_> {
    /// Sets menu data slot `index` (0..256); the content menu must declare it. Values are clamped to the signed 16-bit wire range.
    pub fn set_menu_data(&mut self, index: usize, value: i32) {
        set_menu_data(self.data, index, value);
    }

    /// The fluid in tank `index`, if it holds any (tanks are declared with [`BlockEntityType::fluid_tanks`]).
    pub fn fluid(&self, index: usize) -> Option<FluidStack> {
        let tank = self.data.get("fluids")?.as_list()?.get(index)?.as_compound()?;
        let fluid = tank.get("Fluid")?.as_str()?;
        let amount = tank.get("Amount")?.as_i64()?;
        if fluid == "minecraft:empty" || amount <= 0 {
            return None;
        }
        Some(FluidStack { fluid: Identifier::parse(fluid).ok()?, amount: amount as u32 })
    }

    /// Puts `stack` in tank `index`, or empties the tank with `None`. The tanks before `index` are kept (empty if there are none yet).
    pub fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>) {
        let mut tanks: Vec<Nbt> = match self.data.get("fluids") {
            Some(Nbt::List(list)) => list.clone(),
            _ => Vec::new(),
        };
        let empty = || {
            let mut tank = NbtCompound::new();
            tank.insert("Fluid".to_owned(), Nbt::String("minecraft:empty".to_owned()));
            tank.insert("Amount".to_owned(), Nbt::Int(0));
            Nbt::Compound(tank)
        };
        while tanks.len() <= index {
            tanks.push(empty());
        }
        tanks[index] = match stack {
            Some(stack) if stack.amount > 0 => {
                let mut tank = NbtCompound::new();
                tank.insert("Fluid".to_owned(), Nbt::String(stack.fluid.to_string()));
                tank.insert("Amount".to_owned(), Nbt::Int(stack.amount.min(i32::MAX as u32) as i32));
                Nbt::Compound(tank)
            }
            _ => empty(),
        };
        self.data.insert("fluids".to_owned(), Nbt::List(tanks));
    }
}

type Ticker = Arc<dyn Fn(&mut BlockEntityTick<'_>) + Send + Sync>;
type UpdateTag = Arc<dyn Fn(&NbtCompound) -> NbtCompound + Send + Sync>;

/// The kind of block entity a block has (Minecraft: `BlockEntityType`).
#[derive(Clone)]
pub struct BlockEntityType {
    /// The block this type belongs to.
    pub block: Identifier,
    /// The type's own id, saved with the block entity; the block's id unless changed.
    pub id: Identifier,
    /// The data a new block entity starts with.
    pub initial_data: NbtCompound,
    slots: u32,
    menu: Option<String>,
    tanks: Vec<u32>,
    ticker: Option<Ticker>,
    update_tag: Option<UpdateTag>,
}

impl BlockEntityType {
    /// A block entity type for `block`, with no data, no ticker and nothing sent to clients.
    pub fn new(block: Identifier) -> Self {
        Self { id: block.clone(), block, initial_data: NbtCompound::new(), slots: 0, menu: None, tanks: Vec::new(), ticker: None, update_tag: None }
    }

    /// The same type under another id.
    pub fn with_id(mut self, id: Identifier) -> Self {
        self.id = id;
        self
    }

    /// The same type, new block entities starting with `data`.
    pub fn initial_data(mut self, data: NbtCompound) -> Self {
        self.initial_data = data;
        self
    }

    /// The same type, with `count` item slots (at most 256) that hoppers and the block's menu can reach (Minecraft: `Container`).
    pub fn item_slots(mut self, count: u32) -> Self {
        self.slots = count.min(256);
        self
    }

    /// The same type, with the menu the game opens when the block is used (its item slots are the menu's): one of `minecraft:furnace`,
    /// `minecraft:blast_furnace`, `minecraft:smoker` (3 slots), `minecraft:hopper` (5), `minecraft:generic_3x3` (9),
    /// `minecraft:generic_9x1` to `minecraft:generic_9x6` (9 to 54), or a menu type of the mod's content; the slot count must match.
    /// The window around the slots is registered with `ScreenHandlerRegistry`.
    pub fn menu(mut self, menu_type: &str) -> Self {
        self.menu = Some(menu_type.to_owned());
        self
    }

    /// The same type, with one fluid tank for each capacity given, in millibuckets (at most 16 tanks, each 1 to 1 000 000). The
    /// contents are saved with the block entity; read and change them in the ticker with [`BlockEntityTick::fluid`] and
    /// [`BlockEntityTick::set_fluid`], and other blocks can fill and drain them (Fabric: `FluidStorage`).
    pub fn fluid_tanks(mut self, capacities: impl IntoIterator<Item = u32>) -> Self {
        self.tanks = capacities.into_iter().take(16).map(|c| c.clamp(1, 1_000_000)).collect();
        self
    }

    /// The same type, ticked on the server every game tick by `ticker` (Fabric: `BlockEntityTicker`).
    pub fn ticker(mut self, ticker: impl Fn(&mut BlockEntityTick<'_>) + Send + Sync + 'static) -> Self {
        self.ticker = Some(Arc::new(ticker));
        self
    }

    /// The same type, clients being sent what `update_tag` makes of the saved data whenever a chunk holding it is sent or its block
    /// changes (Minecraft: `BlockEntity.getUpdateTag`).
    pub fn update_tag(mut self, update_tag: impl Fn(&NbtCompound) -> NbtCompound + Send + Sync + 'static) -> Self {
        self.update_tag = Some(Arc::new(update_tag));
        self
    }

    /// The same type, clients being sent all of its saved data.
    pub fn sync_data(self) -> Self {
        self.update_tag(NbtCompound::clone)
    }
}

static TYPES: Mutex<Vec<BlockEntityType>> = Mutex::new(Vec::new());

/// Registers block entity types (Fabric: `Registry.register(Registries.BLOCK_ENTITY_TYPE, ...)`).
pub struct BlockEntityTypeRegistry;

impl BlockEntityTypeRegistry {
    /// Gives `entity_type.block` its block entity. Registering the same block again replaces the earlier type.
    pub fn register(entity_type: BlockEntityType) {
        if let Ok(mut all) = TYPES.lock() {
            all.retain(|t| t.block != entity_type.block);
            all.push(entity_type);
        }
    }

    /// The type registered for `block` (an id such as `tconstruct:seared_melter`), if any.
    pub fn get(block: &str) -> Option<BlockEntityType> {
        TYPES.lock().ok()?.iter().find(|t| t.block.to_string() == block).cloned()
    }
}

fn by_entity_id(id: &str) -> Option<BlockEntityType> {
    TYPES.lock().ok()?.iter().find(|t| t.id.to_string() == id).cloned()
}

/// A new block entity for `block`: its type id and starting data; None if the block has no registered type.
pub(crate) fn create(block: &str) -> Option<(String, NbtCompound)> {
    BlockEntityTypeRegistry::get(block).map(|t| (t.id.to_string(), t.initial_data))
}

/// The number of item slots the block's block entity has; None if it has no container.
pub(crate) fn item_slots(block: &str) -> Option<u32> {
    BlockEntityTypeRegistry::get(block).map(|t| t.slots).filter(|&n| n > 0)
}

/// The menu type of the block's container, if it has one.
pub(crate) fn menu(block: &str) -> Option<String> {
    BlockEntityTypeRegistry::get(block).and_then(|t| t.menu)
}

/// The block whose type opens the menu type `menu`.
pub(crate) fn block_of_menu(menu: &str) -> Option<String> {
    TYPES.lock().ok()?.iter().find(|t| t.menu.as_deref() == Some(menu)).map(|t| t.block.to_string())
}

/// The block whose type has the block entity id `entity_id`.
pub(crate) fn block_of(entity_id: &str) -> Option<String> {
    by_entity_id(entity_id).map(|t| t.block.to_string())
}

/// The capacity of each fluid tank of the block's block entity; None if it has none.
pub(crate) fn fluid_tanks(block: &str) -> Option<Vec<u32>> {
    BlockEntityTypeRegistry::get(block).map(|t| t.tanks).filter(|t| !t.is_empty())
}

/// Whether the block's block entity ticks.
pub(crate) fn ticks(block: &str) -> bool {
    BlockEntityTypeRegistry::get(block).is_some_and(|t| t.ticker.is_some())
}

/// Runs the ticker of the block entity type `entity_id` on `data`; true if a ticker ran.
pub(crate) fn tick(entity_id: &str, pos: BlockPos, data: &mut NbtCompound, items: &mut Vec<Option<ItemSlot>>) -> bool {
    match by_entity_id(entity_id).and_then(|t| t.ticker) {
        Some(ticker) => {
            ticker(&mut BlockEntityTick { pos, data, items });
            true
        }
        None => false,
    }
}

/// What clients are sent of the block entity type `entity_id` with `data`; None sends nothing.
pub(crate) fn update_tag(entity_id: &str, data: &NbtCompound) -> Option<NbtCompound> {
    by_entity_id(entity_id).and_then(|t| t.update_tag).map(|make| make(data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::base::{Nbt, NbtCompoundExt};

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    #[test]
    fn a_registered_block_gets_its_data_a_ticker_and_the_update_tag_it_asked_for() {
        let mut start = NbtCompound::new();
        start.insert("heat".into(), Nbt::Int(0));
        BlockEntityTypeRegistry::register(
            BlockEntityType::new(id("test:furnace"))
                .with_id(id("test:furnace_entity"))
                .initial_data(start)
                .ticker(|tick| {
                    let heat = tick.data.get_int("heat", 0) + 1;
                    tick.data.insert("heat".into(), Nbt::Int(heat as i32));
                    tick.data.insert("x".into(), Nbt::Int(tick.pos[0]));
                })
                .update_tag(|data| {
                    let mut tag = NbtCompound::new();
                    tag.insert("heat".into(), data["heat"].clone());
                    tag
                }),
        );
        let (entity, mut data) = create("test:furnace").unwrap();
        assert_eq!(entity, "test:furnace_entity");
        assert!(ticks("test:furnace"));
        let mut items = Vec::new();
        assert!(tick(&entity, [5, 64, 5], &mut data, &mut items));
        assert!(tick(&entity, [5, 64, 5], &mut data, &mut items));
        assert_eq!(data.get_int("heat", -1), 2);
        assert_eq!(data.get_int("x", -1), 5);
        let tag = update_tag(&entity, &data).unwrap();
        assert_eq!(tag.len(), 1, "only what the update tag chose is sent");
        assert_eq!(tag.get_int("heat", -1), 2);
        assert!(create("test:unknown").is_none());
        assert!(!ticks("test:unknown"));
        assert!(!tick("test:unknown_entity", [0; 3], &mut data, &mut items));
    }

    #[test]
    fn slots_and_tanks_are_declared_per_block_and_fluid_contents_round_trip_through_the_saved_data() {
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:tank")).item_slots(3).fluid_tanks([4000, 0, 5_000_000]));
        assert_eq!(item_slots("test:tank"), Some(3));
        assert_eq!(fluid_tanks("test:tank"), Some(vec![4000, 1, 1_000_000]), "capacities are clamped to 1..=1 000 000");
        assert_eq!(item_slots("test:nothing"), None);
        let mut data = NbtCompound::new();
        let mut items = vec![None, Some(ItemSlot::new(id("minecraft:stick"), 2)), None];
        let mut tick = BlockEntityTick { pos: [0; 3], data: &mut data, items: &mut items };
        assert_eq!(tick.fluid(1), None);
        tick.set_fluid(1, Some(FluidStack { fluid: id("minecraft:lava"), amount: 250 }));
        assert_eq!(tick.fluid(0), None, "the tank before is kept, empty");
        assert_eq!(tick.fluid(1), Some(FluidStack { fluid: id("minecraft:lava"), amount: 250 }));
        tick.set_fluid(1, None);
        assert_eq!(tick.fluid(1), None);
        assert_eq!(tick.items[1].as_ref().unwrap().count, 2);
    }

    #[test]
    fn a_type_without_a_ticker_does_not_tick_and_sync_data_sends_everything() {
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:quiet")).sync_data());
        assert!(!ticks("test:quiet"));
        let mut data = NbtCompound::new();
        data.insert("a".into(), Nbt::Byte(1));
        assert_eq!(update_tag("test:quiet", &data), Some(data.clone()));
        assert!(!tick("test:quiet", [0; 3], &mut data, &mut Vec::new()));
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:quiet")));
        assert!(update_tag("test:quiet", &data).is_none(), "registering again replaces the earlier type");
    }
}
