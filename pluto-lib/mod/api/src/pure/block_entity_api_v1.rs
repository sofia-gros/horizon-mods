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

// Host-owned value transported without exposing host types through the public API.
#[derive(Clone)]
struct StackSnapshot {
    value: Arc<dyn std::any::Any + Send + Sync>,
    equal: fn(&dyn std::any::Any, &dyn std::any::Any) -> bool,
}
impl StackSnapshot {
    fn new<T: std::any::Any + Send + Sync + PartialEq>(value: T) -> Self {
        Self { value: Arc::new(value), equal: |left, right| {
            left.downcast_ref::<T>().zip(right.downcast_ref::<T>()).is_some_and(|(a, b)| a == b)
        } }
    }
}
impl std::fmt::Debug for StackSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StackSnapshot(..)")
    }
}
impl PartialEq for StackSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.value, &other.value) || (self.equal)(self.value.as_ref(), other.value.as_ref())
    }
}

/// One stack in an item slot. Clone an existing slot to retain all native components when moving it.
/// A newly constructed slot is plain; changing its item identity discards the captured components.
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
    /// The stack's own `minecraft:tool` component (mining rules and speeds), when it has one. Set it
    /// to give this stack, not every stack of the item, its mining behavior; `None` leaves the item's.
    pub tool: Option<NbtCompound>,
    /// The stack's own `minecraft:attribute_modifiers` component, one compound per modifier (`type`, `id`, `amount`, `operation`, `slot`),
    /// when it has one. Set it to give this stack, not every stack of the item, its attribute modifiers: an empty list gives it none;
    /// `None` leaves the item's.
    pub attribute_modifiers: Option<Vec<NbtCompound>>,
    /// The stack's own `minecraft:enchantments` component as enchantment id to level (such as `minecraft:fortune` to 3), when it has one.
    /// Set it to give this stack, not every stack of the item, those enchantments; an empty map removes them; `None` leaves the stack's.
    pub enchantments: Option<std::collections::BTreeMap<String, i32>>,
    /// The stack's own `minecraft:custom_model_data` component, which item models select on (see `CustomModelData`), when it has one.
    /// Set it to give this stack, not every stack of the item, that model selection; `None` leaves the stack's.
    pub custom_model_data: Option<crate::pure::player_api::CustomModelData>,
    snapshot: Option<StackSnapshot>,
}

impl ItemSlot {
    /// A plain stack of `count` of `item`, with no damage and no data.
    pub fn new(item: Identifier, count: u32) -> Self {
        Self { item, count, damage: 0, max_damage: None, custom_data: NbtCompound::new(), tool: None, attribute_modifiers: None, enchantments: None, custom_model_data: None, snapshot: None }
    }

    pub(crate) fn capture<T: std::any::Any + Send + Sync + PartialEq>(&mut self, value: T) {
        self.snapshot = Some(StackSnapshot::new(value));
    }

    pub(crate) fn captured<T: std::any::Any>(&self) -> Option<&T> {
        self.snapshot.as_ref()?.value.downcast_ref()
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

/// The fluid in tank `index` of saved data, if it holds any.
fn read_fluid(data: &NbtCompound, index: usize) -> Option<FluidStack> {
    let tank = data.get("fluids")?.as_list()?.get(index)?.as_compound()?;
    let fluid = tank.get("Fluid")?.as_str()?;
    let amount = tank.get("Amount")?.as_i64()?;
    if fluid == "minecraft:empty" || amount <= 0 {
        return None;
    }
    Some(FluidStack { fluid: Identifier::parse(fluid).ok()?, amount: amount as u32 })
}

/// Puts `stack` in tank `index` of saved data, or empties the tank with `None`; the tanks before it are kept (empty if there are none yet).
fn write_fluid(data: &mut NbtCompound, index: usize, stack: Option<FluidStack>) {
    let mut tanks: Vec<Nbt> = match data.get("fluids") {
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
    data.insert("fluids".to_owned(), Nbt::List(tanks));
}

impl BlockEntityTick<'_> {
    /// Sets menu data slot `index` (0..256); the content menu must declare it. Values are clamped to the signed 16-bit wire range.
    pub fn set_menu_data(&mut self, index: usize, value: i32) {
        set_menu_data(self.data, index, value);
    }

    /// The fluid in tank `index`, if it holds any (tanks are declared with [`BlockEntityType::fluid_tanks`]).
    pub fn fluid(&self, index: usize) -> Option<FluidStack> {
        read_fluid(self.data, index)
    }

    /// Puts `stack` in tank `index`, or empties the tank with `None`. The tanks before `index` are kept (empty if there are none yet).
    pub fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>) {
        write_fluid(self.data, index, stack);
    }
}

/// A fluid offered to a block entity's tanks: what [`BlockEntityType::fluid_filler`] receives. The hook decides how much of `fluid` the block
/// takes, and may change the saved data and the item slots (a casting table starts its recipe here and fixes its tank to the recipe's amount).
pub struct BlockEntityFill<'a> {
    /// The block entity's position.
    pub pos: BlockPos,
    /// The saved data; changes are kept only when `execute` is true.
    pub data: &'a mut NbtCompound,
    /// The item slots; changes are kept only when `execute` is true.
    pub items: &'a mut Vec<Option<ItemSlot>>,
    /// What is offered.
    pub fluid: FluidStack,
    /// The face of this block the fluid comes through (`None` when the caller does not say, as the original's handler answers a null side).
    pub face: Option<super::block_api_v1::Direction>,
    /// False when the caller only asks how much would be taken: the changes are thrown away, so the hook may run the same code for both.
    pub execute: bool,
}

impl BlockEntityFill<'_> {
    /// The fluid in tank `index`, if it holds any.
    pub fn fluid_in(&self, index: usize) -> Option<FluidStack> {
        read_fluid(self.data, index)
    }

    /// Puts `stack` in tank `index`, or empties the tank with `None`.
    pub fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>) {
        write_fluid(self.data, index, stack);
    }
}

/// A player used the block with the block entity: what [`BlockEntityType::on_use`] receives (Minecraft: `BlockEntity` reached from
/// `Block.use`; Fabric: `UseBlockCallback` with the block entity at hand).
pub struct BlockEntityUse<'a> {
    /// The block entity's position.
    pub pos: BlockPos,
    /// The saved data.
    pub data: &'a mut NbtCompound,
    /// The item slots.
    pub items: &'a mut Vec<Option<ItemSlot>>,
    /// The stack in the hand that was used; `None` for an empty hand. The hook may change it (taking one off the stack) or empty it.
    pub held: Option<ItemSlot>,
    /// Whether the player was sneaking.
    pub sneaking: bool,
    /// Stacks the hook gives the player: into the inventory, dropped at the player's feet when it is full.
    pub give: Vec<ItemSlot>,
}

impl BlockEntityUse<'_> {
    /// The fluid in tank `index`, if it holds any.
    pub fn fluid(&self, index: usize) -> Option<FluidStack> {
        read_fluid(self.data, index)
    }
}

/// What a use of the block did: the answer, the hand afterwards and what the player is given.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockUseOutcome {
    /// `Pass` leaves the use to the game.
    pub result: crate::pure::base::action_result::ActionResult,
    /// The stack in the used hand afterwards; `None` for an empty hand.
    pub held: Option<ItemSlot>,
    /// Stacks to give the player.
    pub give: Vec<ItemSlot>,
}

/// A client-only block entity tick. Saved/synchronized data is read-only;
/// client_data belongs to this client entity instance, is never serialized or
/// sent to the server, and is discarded when the instance unloads or is replaced.
/// Native instance ownership and dispatch remain an explicit support-matrix stub.
pub struct BlockEntityClientTick<'a> {
    pub pos: BlockPos,
    pub data: &'a NbtCompound,
    pub client_data: &'a mut NbtCompound,
    pub world: &'a dyn super::block_api_v1::BlockView,
}

type ClientTicker = Arc<dyn Fn(&mut BlockEntityClientTick<'_>) + Send + Sync>;
type Ticker = Arc<dyn Fn(&mut BlockEntityTick<'_>) + Send + Sync>;
type Filler = Arc<dyn Fn(&mut BlockEntityFill<'_>) -> u32 + Send + Sync>;
type User = Arc<dyn Fn(&mut BlockEntityUse<'_>) -> crate::pure::base::action_result::ActionResult + Send + Sync>;
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
    client_ticker: Option<ClientTicker>,
    update_tag: Option<UpdateTag>,
    filler: Option<Filler>,
    user: Option<User>,
}

impl BlockEntityType {
    /// A block entity type for `block`, with no data, no ticker and nothing sent to clients.
    pub fn new(block: Identifier) -> Self {
        Self { id: block.clone(), block, initial_data: NbtCompound::new(), slots: 0, menu: None, tanks: Vec::new(), ticker: None, client_ticker: None, update_tag: None, filler: None, user: None }
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

    /// Registers the client-level ticker, independently of the server ticker.
    /// Callable and pure-testable now; native ticking/transient render transport
    /// are not wired yet, so registration alone produces no client animation.
    pub fn client_ticker(mut self, ticker: impl Fn(&mut BlockEntityClientTick<'_>) + Send + Sync + 'static) -> Self {
        self.client_ticker = Some(Arc::new(ticker));
        self
    }

    /// The same type, clients being sent what `update_tag` makes of the saved data whenever a chunk holding it is sent or its block
    /// changes (Minecraft: `BlockEntity.getUpdateTag`).
    pub fn update_tag(mut self, update_tag: impl Fn(&NbtCompound) -> NbtCompound + Send + Sync + 'static) -> Self {
        self.update_tag = Some(Arc::new(update_tag));
        self
    }

    /// The same type, with its own rule for fluid put into it: `filler` is asked how much of the offered fluid the block takes, and may change
    /// the saved data (Fabric: a custom `Storage<FluidVariant>.insert`; NeoForge: a custom `IFluidHandler.fill`). Without one the host fills
    /// the declared tanks up to their capacity. It lets a block take a capacity that depends on what it is offered.
    pub fn fluid_filler(mut self, filler: impl Fn(&mut BlockEntityFill<'_>) -> u32 + Send + Sync + 'static) -> Self {
        self.filler = Some(Arc::new(filler));
        self
    }

    /// The same type, with `user` run when a player uses the block (right click): it may exchange items with the hand and the slots. Its
    /// answer is the use's answer; `Pass` leaves it to the game.
    pub fn on_use(mut self, user: impl Fn(&mut BlockEntityUse<'_>) -> crate::pure::base::action_result::ActionResult + Send + Sync + 'static) -> Self {
        self.user = Some(Arc::new(user));
        self
    }

    /// The same type, clients being sent all of its saved data.
    pub fn sync_data(self) -> Self {
        self.update_tag(NbtCompound::clone)
    }
}

static TYPES: Mutex<Vec<BlockEntityType>> = Mutex::new(Vec::new());
pub(crate) fn registry_available() -> bool { !TYPES.is_poisoned() }

/// Registers block entity types (Fabric: `Registry.register(Registries.BLOCK_ENTITY_TYPE, ...)`).
pub struct BlockEntityTypeRegistry;

impl BlockEntityTypeRegistry {
    /// Gives `entity_type.block` its block entity. Registering the same block again replaces the earlier type.
    pub fn register(entity_type: BlockEntityType) {
        if let Ok(mut all) = TYPES.lock() {
            all.retain(|t| t.block != entity_type.block);
            all.push(entity_type);
            super::render_cache::changed();
        }
    }

    /// The type registered for `block` (an id such as `tconstruct:seared_melter`), if any.
    pub fn get(block: &str) -> Option<BlockEntityType> {
        TYPES.lock().ok()?.iter().find(|t| t.block.to_string() == block).cloned()
    }

    /// Offers `fluid` to the block entity of `block` with the saved `data` and `items`: how much its [`BlockEntityType::fluid_filler`] takes, or
    /// `None` when the block has no filler (the host fills the declared tanks). With `execute` false the changes the filler makes are thrown away.
    pub fn fill_fluid(block: &str, pos: BlockPos, data: &mut NbtCompound, items: &mut Vec<Option<ItemSlot>>, fluid: FluidStack, face: Option<super::block_api_v1::Direction>, execute: bool) -> Option<u32> {
        let filler = Self::get(block)?.filler?;
        if execute {
            return Some(filler(&mut BlockEntityFill { pos, data, items, fluid, face, execute }));
        }
        let (mut data, mut items) = (data.clone(), items.clone());
        Some(filler(&mut BlockEntityFill { pos, data: &mut data, items: &mut items, fluid, face, execute }))
    }

    /// Runs the [`BlockEntityType::on_use`] hook of `block` for a player holding `held` (`None`: an empty hand); `None` when the block has no hook.
    pub fn use_block(block: &str, pos: BlockPos, data: &mut NbtCompound, items: &mut Vec<Option<ItemSlot>>, held: Option<ItemSlot>, sneaking: bool) -> Option<BlockUseOutcome> {
        let user = Self::get(block)?.user?;
        let mut used = BlockEntityUse { pos, data, items, held, sneaking, give: Vec::new() };
        let result = user(&mut used);
        Some(BlockUseOutcome { result, held: used.held, give: used.give })
    }
}

fn by_entity_id(id: &str) -> Option<BlockEntityType> {
    TYPES.lock().ok()?.iter().find(|t| t.id.to_string() == id).cloned()
}

/// Predicate for the draft mod-only native factory-priority adapter. Keep a
/// poisoned registry distinct from an unregistered block so it cannot silently
/// become a vanilla skull/sign after registration was intended.
pub(crate) fn registered_type(block: &str) -> Result<bool, String> {
    Ok(TYPES.lock().map_err(|_| "block entity type registry poisoned")?
        .iter().any(|entity_type| entity_type.block.to_string() == block))
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

/// Whether the block's block entity has its own fluid rule (the host must route fills to it instead of the declared tanks).
pub(crate) fn has_fluid_filler(block: &str) -> bool {
    BlockEntityTypeRegistry::get(block).is_some_and(|t| t.filler.is_some())
}

/// Whether the block's block entity ticks.
pub(crate) fn ticks(block: &str) -> bool {
    BlockEntityTypeRegistry::get(block).is_some_and(|t| t.ticker.is_some())
}

/// A client ticker makes render geometry frame-dependent. None preserves a
/// poisoned registry as unavailable; cache policy must not treat that as static.
pub(crate) fn client_ticks(block: &str) -> Option<bool> {
    TYPES.lock().ok().map(|all| all.iter().any(|t| t.block.to_string() == block && t.client_ticker.is_some()))
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

/// Source dispatcher for a client copy. The future native adapter must supply
/// instance-owned transient data, not a global position map or saved entity NBT.
/// Resolve by actual block, not shared type ID: floor/wall variants may differ.
pub(crate) fn client_tick(
    block: &str, pos: BlockPos, data: &NbtCompound,
    client_data: &mut NbtCompound, world: &dyn super::block_api_v1::BlockView,
    client_side: bool,
) -> bool {
    if !client_side { return false; }
    let ticker = BlockEntityTypeRegistry::get(block).and_then(|t| t.client_ticker);
    let Some(ticker) = ticker else { return false };
    ticker(&mut BlockEntityClientTick { pos, data, client_data, world });
    true
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
    fn client_tickers_keep_transient_instances_separate_from_saved_data_and_server_ticks() {
        use crate::pure::block_api_v1::{BlockView, MemoryBlockView};
        let block = "test:client_clock";
        BlockEntityTypeRegistry::register(BlockEntityType::new(id(block)).sync_data().client_ticker(|tick| {
            assert!(BlockEntityTypeRegistry::get("test:client_clock").is_some()); // no lock held
            let Some(powered) = tick.world.has_neighbor_signal(tick.pos) else { return };
            if powered {
                let count = tick.client_data.get_int("frames", 0);
                tick.client_data.insert("frames".into(), Nbt::Int(count as i32 + 1));
            }
            assert_eq!(tick.data.get_int("server", -1), 7);
        }));
        let mut world = MemoryBlockView::default();
        world.neighbor_signals.insert([1, 2, 3], true);
        world.neighbor_signals.insert([4, 5, 6], false);
        let mut saved = NbtCompound::new();
        saved.insert("server".into(), Nbt::Int(7));
        let mut a = NbtCompound::new();
        let mut b = NbtCompound::new();
        assert!(!ticks(block));
        assert!(!tick(block, [1, 2, 3], &mut saved, &mut Vec::new()));
        assert!(!client_tick(block, [1, 2, 3], &saved, &mut a, &world, false));
        assert!(a.is_empty());
        assert!(client_tick(block, [1, 2, 3], &saved, &mut a, &world, true));
        assert!(client_tick(block, [1, 2, 3], &saved, &mut a, &world, true));
        assert!(client_tick(block, [4, 5, 6], &saved, &mut b, &world, true));
        assert_eq!(a.get_int("frames", 0), 2);
        assert!(b.is_empty());
        assert_eq!(update_tag(block, &saved), Some(saved.clone()));
        assert_eq!(saved.len(), 1); // transient data never entered the update tag
        world.unloaded.insert([1, 2, 3]);
        assert_eq!(world.has_neighbor_signal([1, 2, 3]), None);
        assert!(client_tick(block, [1, 2, 3], &saved, &mut a, &world, true));
        assert_eq!(a.get_int("frames", 0), 2); // unknown is not false or true
        assert!(!client_tick("test:missing_client_clock", [0; 3], &saved, &mut b, &world, true));
        BlockEntityTypeRegistry::register(BlockEntityType::new(id(block)));
        assert!(!client_tick(block, [1, 2, 3], &saved, &mut a, &world, true));
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

    #[test]
    fn a_fluid_filler_decides_what_a_block_takes_and_a_simulation_keeps_nothing_and_a_use_hook_trades_with_the_hand() {
        use crate::pure::base::action_result::ActionResult;
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:basin")).item_slots(1).fluid_tanks([1000])
            .fluid_filler(|fill| {
                // takes up to 90 of one fluid
                let held = fill.fluid_in(0).map_or(0, |held| held.amount);
                let taken = fill.fluid.amount.min(90 - held);
                if taken > 0 { fill.set_fluid(0, Some(FluidStack { fluid: fill.fluid.fluid.clone(), amount: held + taken })); }
                taken
            })
            .on_use(|used| {
                let Some(stack) = used.held.take() else { return ActionResult::Pass };
                used.give.push(stack.clone());
                used.items[0] = Some(stack);
                ActionResult::Success
            }));
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:plain_tank")).fluid_tanks([1000]));
        let (mut data, mut items) = (NbtCompound::new(), vec![None]);
        let lava = |amount| FluidStack { fluid: id("minecraft:lava"), amount };
        assert_eq!(BlockEntityTypeRegistry::fill_fluid("test:basin", [0; 3], &mut data, &mut items, lava(50), None, false), Some(50));
        assert!(data.is_empty(), "a simulation changes nothing");
        assert_eq!(BlockEntityTypeRegistry::fill_fluid("test:basin", [0; 3], &mut data, &mut items, lava(50), None, true), Some(50));
        assert_eq!(BlockEntityTypeRegistry::fill_fluid("test:basin", [0; 3], &mut data, &mut items, lava(50), None, true), Some(40), "90 is the most it takes");
        assert!(has_fluid_filler("test:basin") && !has_fluid_filler("test:plain_tank"));
        assert_eq!(BlockEntityTypeRegistry::fill_fluid("test:plain_tank", [0; 3], &mut data, &mut items, lava(5), None, true), None, "no filler: the host fills the tanks");
        let stick = ItemSlot::new(id("minecraft:stick"), 1);
        let out = BlockEntityTypeRegistry::use_block("test:basin", [0; 3], &mut data, &mut items, Some(stick.clone()), false).unwrap();
        assert_eq!((out.result, out.held, out.give, items[0].clone()), (ActionResult::Success, None, vec![stick.clone()], Some(stick)));
        assert_eq!(BlockEntityTypeRegistry::use_block("test:basin", [0; 3], &mut data, &mut items, None, false).unwrap().result, ActionResult::Pass);
        assert!(BlockEntityTypeRegistry::use_block("test:plain_tank", [0; 3], &mut data, &mut items, None, false).is_none());
    }
}
