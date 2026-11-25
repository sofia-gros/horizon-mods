//! Block entities for a mod's blocks: data that belongs to a block position, saved with the chunk, ticked every game tick on the server
//! and optionally sent to clients (Minecraft: `BlockEntity`, `BlockEntityType`, `BlockEntityTicker`; Fabric:
//! `FabricBlockEntityTypeBuilder`).
//!
//! The block itself comes from the mod's content (its blocks and models). Here a block gets its block entity type: the data it starts
//! with, what it does each tick, and what clients are told of it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::base::{Identifier, Nbt, NbtCompound};
use super::block_api_v1::Direction;
use super::player_api::ServerPlayer;

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
    /// The stack's own enchantments as enchantment id to level (such as `minecraft:fortune` to 3), when it has any:
    /// the `minecraft:enchantments` component, or `minecraft:stored_enchantments` for an enchanted book, as vanilla keeps them.
    /// Set it to give this stack, not every stack of the item, those enchantments; an empty map removes them; `None` leaves the stack's.
    pub enchantments: Option<std::collections::BTreeMap<String, i32>>,
    /// The stack's own `minecraft:custom_model_data` component, which item models select on (see `CustomModelData`), when it has one.
    /// Set it to give this stack, not every stack of the item, that model selection; `None` leaves the stack's.
    pub custom_model_data: Option<crate::pure::player_api::CustomModelData>,
    /// The stack's own `minecraft:profile` component (a player head's owner: `name`, `id` as the uuid's four ints, and
    /// `properties`), when it has one. Set it to give this stack, not every stack of the item, that owner — a head a
    /// beheading drops keeps the player's skin through it; `None` leaves the stack's.
    pub profile: Option<NbtCompound>,
    /// The stack's own `minecraft:map_id` component (a filled map's id), when it has one. Set it
    /// to give this stack, not every stack of the item, that map id — a map a minimap inventory
    /// stores keeps its explored data through it; `None` leaves the stack's.
    pub map_id: Option<i32>,
    snapshot: Option<StackSnapshot>,
}

impl ItemSlot {
    /// A plain stack of `count` of `item`, with no damage and no data.
    pub fn new(item: Identifier, count: u32) -> Self {
        Self { item, count, damage: 0, max_damage: None, custom_data: NbtCompound::new(), tool: None, attribute_modifiers: None, enchantments: None, custom_model_data: None, profile: None, map_id: None, snapshot: None }
    }

    pub(crate) fn capture<T: std::any::Any + Send + Sync + PartialEq>(&mut self, value: T) {
        self.snapshot = Some(StackSnapshot::new(value));
    }

    pub(crate) fn captured<T: std::any::Any>(&self) -> Option<&T> {
        self.snapshot.as_ref()?.value.downcast_ref()
    }
}

/// Fluid in a tank: which fluid, how many millibuckets (a bucket is a thousand) and the fluid's own
/// tag when it has one (Forge's `FluidStack.tag`): potion contents are the usual case, where the
/// fluid id alone does not say what the fluid is. Two stacks hold the same fluid only when the id
/// and the tag both match, so equality is tag-aware; a plain fluid has no tag.
#[derive(Clone, Debug, PartialEq)]
pub struct FluidStack {
    /// The fluid, such as `minecraft:water` or a mod's own.
    pub fluid: Identifier,
    /// Millibuckets.
    pub amount: u32,
    /// The fluid's own tag, or `None` for a plain fluid.
    pub tag: Option<NbtCompound>,
}

impl FluidStack {
    /// A plain stack of `amount` millibuckets of `fluid`, with no tag.
    pub fn new(fluid: Identifier, amount: u32) -> Self {
        Self { fluid, amount, tag: None }
    }

    /// Whether both stacks hold the same fluid with the same tag (the amounts may differ).
    pub fn same_fluid(&self, other: &FluidStack) -> bool {
        self.fluid == other.fluid && self.tag == other.tag
    }
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
pub(crate) fn read_fluid(data: &NbtCompound, index: usize) -> Option<FluidStack> {
    let tank = data.get("fluids")?.as_list()?.get(index)?.as_compound()?;
    let fluid = tank.get("Fluid")?.as_str()?;
    let amount = tank.get("Amount")?.as_i64()?;
    if fluid == "minecraft:empty" || amount <= 0 {
        return None;
    }
    // The tag is optional and absent in data written before stacks carried one: plain fluid then.
    let tag = tank.get("Tag").and_then(|tag| tag.as_compound().cloned());
    Some(FluidStack { fluid: Identifier::parse(fluid).ok()?, amount: amount as u32, tag })
}

/// Puts `stack` in tank `index` of saved data, or empties the tank with `None`; the tanks before it are kept (empty if there are none yet).
pub(crate) fn write_fluid(data: &mut NbtCompound, index: usize, stack: Option<FluidStack>) {
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
            if let Some(tag) = stack.tag {
                tank.insert("Tag".to_owned(), Nbt::Compound(tag));
            }
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

    /// Puts `stack` in tank `index`, or empties the tank with `None` (a held bucket filling or emptying the block's tank).
    pub fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>) {
        write_fluid(self.data, index, stack);
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

/// A block entity was just placed by a player: what [`BlockEntityType::on_place`] receives (Minecraft: the placed stack's data
/// reaching the new block entity, as `BlockItem.place` and `BlockEntity.load` do for a tank item's stored fluid; Fabric:
/// `BlockEntity` creation with the stack at hand). The hook runs once, right after the block entity exists, over its starting
/// data and slots, so it can seed itself from the stack that was placed: a tank item's fluid, a name the stack carries, a
/// retexture the stack names. A block placed by the game itself (a piston, world generation) is not heard.
pub struct BlockEntityPlace<'a> {
    /// The block entity's position.
    pub pos: BlockPos,
    /// The stack that was placed, with all its components; the game's own count is already settled (the stack in the hand has
    /// not shrunk yet, so its count is the hand's, not the placed block's).
    pub stack: ItemSlot,
    /// The player who placed it; None when the game names none.
    pub player: Option<ServerPlayer>,
    /// The saved data the block entity starts with (its type's initial data); changes are saved with the chunk.
    pub data: &'a mut NbtCompound,
    /// The item slots; changes are saved with the chunk.
    pub items: &'a mut Vec<Option<ItemSlot>>,
}

impl BlockEntityPlace<'_> {
    /// The fluid in tank `index`, if it holds any.
    pub fn fluid(&self, index: usize) -> Option<FluidStack> {
        read_fluid(self.data, index)
    }

    /// Puts `stack` in tank `index`, or empties the tank with `None` (a placed tank item's fluid seeding the tank).
    pub fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>) {
        write_fluid(self.data, index, stack);
    }
}

/// The redstone signal reaching a block entity changed: what [`BlockEntityType::on_signal`] receives. Heard from the block
/// entity's own server tick, so only a type that also ticks hears it; the first tick after the block entity starts reports the
/// strength it finds (not a change), so a hook that acts on an edge compares `strength` with the last one it saved in its data,
/// as the game's own `handleNeighbourUpdate`-style logic does. The signal is the strongest reaching the block from its six
/// neighbours (`SignalGetter.getBestNeighborSignal`, what a dispenser or a piston reads).
pub struct BlockEntitySignal<'a> {
    /// The block entity's position.
    pub pos: BlockPos,
    /// The saved data; changes are saved with the chunk.
    pub data: &'a mut NbtCompound,
    /// The item slots; changes are saved with the chunk.
    pub items: &'a mut Vec<Option<ItemSlot>>,
    /// The signal strength now, 0 to 15.
    pub strength: u8,
    /// Whether any signal reaches the block (`strength` above 0).
    pub powered: bool,
}

impl BlockEntitySignal<'_> {
    /// The fluid in tank `index`, if it holds any.
    pub fn fluid(&self, index: usize) -> Option<FluidStack> {
        read_fluid(self.data, index)
    }

    /// Puts `stack` in tank `index`, or empties the tank with `None`.
    pub fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>) {
        write_fluid(self.data, index, stack);
    }
}

/// Which items a block that forwards its item storage lets through (see [`BlockEntityType::item_forward`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemFilter {
    /// Every item the target holds or takes.
    Any,
    /// Only this item: inserts of another are refused, and only slots holding it (or empty ones, for listing) are listed;
    /// extractions of another item are refused.
    Only(Identifier),
    /// Nothing moves and no slots are listed.
    Closed,
}

/// Where a forwarding block's item storage really is, and what passes through it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemForward {
    /// The block entity whose item slots answer for this block.
    pub target: BlockPos,
    /// What passes.
    pub filter: ItemFilter,
}

/// The slots of a container one face of the block opens to automation (Minecraft: `WorldlyContainer.getSlotsForFace` with
/// `canPlaceItemThroughFace` and `canTakeItemThroughFace`): which slots a hopper or another block may put into and take out
/// of through that face. A face the type does not cover answers with two empty lists when a hook is registered at all.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SidedAccess {
    /// The slots that take items through the face.
    pub insert: Vec<usize>,
    /// The slots that give items through the face.
    pub take: Vec<usize>,
}

/// Which way an item transfer moves, for [`BlockEntityTransfer`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferMode {
    /// Into the block entity (a hopper above, another block pushing).
    Insert,
    /// Out of the block entity (a hopper below, another block pulling).
    Take,
}

/// An item transfer automation wants to make with a block entity: what [`BlockEntityType::item_transfer_rule`] is asked
/// (NeoForge: `IItemHandler.isItemValid` for an insert, with the block entity's own state at hand; a take asks the same way,
/// which the original's sided containers answer through `canTakeItemThroughFace`). The rule answers for one slot and one
/// stack; the slots a face opens at all are [`SidedAccess`]'s business, and a stack the rule refuses does not move.
pub struct BlockEntityTransfer<'a> {
    /// The block entity's position.
    pub pos: BlockPos,
    /// The saved data, read-only: the rule decides with it (a casting table that is mid-cast refuses its cast slot).
    pub data: &'a NbtCompound,
    /// The item slots, read-only.
    pub items: &'a [Option<ItemSlot>],
    /// The slot the stack would move through.
    pub slot: usize,
    /// The face the transfer comes through; None when the caller names none (the block's own machinery asking).
    pub face: Option<Direction>,
    /// Which way the stack moves.
    pub mode: TransferMode,
    /// The stack that would move: the incoming one for an insert, the stored one for a take.
    pub stack: ItemSlot,
}


/// Which fluids a block that forwards its fluid storage lets through (see [`BlockEntityType::fluid_forward`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FluidFilter {
    /// Everything the target holds or takes.
    Any,
    /// Only this fluid: fills and drains of another fluid are refused, and only tanks that are empty or hold it are listed.
    Only(Identifier),
    /// Nothing moves; only the last tank is listed, and only while it is empty.
    Closed,
}

/// Where a forwarding block's fluid storage really is, and what passes through it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FluidForward {
    /// The block entity whose storage answers for this block.
    pub target: BlockPos,
    /// What passes.
    pub filter: FluidFilter,
}

/// A drain asked of a block entity's own fluid rule: what [`BlockEntityType::fluid_drainer`] receives.
pub struct BlockEntityDrain<'a> {
    /// The block entity's position.
    pub pos: BlockPos,
    /// The saved data; changes are kept only when `execute` is true.
    pub data: &'a mut NbtCompound,
    /// The item slots; changes are kept only when `execute` is true.
    pub items: &'a mut Vec<Option<ItemSlot>>,
    /// The fluid asked for, or `None` for whatever the block gives first.
    pub wanted: Option<Identifier>,
    /// The most millibuckets to take.
    pub amount: u32,
    /// False when the caller only asks what would come out.
    pub execute: bool,
}

impl BlockEntityDrain<'_> {
    /// The fluid in tank `index` of the data being drained.
    pub fn fluid_in(&self, index: usize) -> Option<FluidStack> {
        read_fluid(self.data, index)
    }

    /// Puts `stack` in tank `index`, or empties it with `None`.
    pub fn set_fluid(&mut self, index: usize, stack: Option<FluidStack>) {
        write_fluid(self.data, index, stack);
    }
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
type Forwarder = Arc<dyn Fn(&NbtCompound, &[Option<ItemSlot>]) -> Option<FluidForward> + Send + Sync>;
type Reader = Arc<dyn Fn(&NbtCompound) -> Vec<super::neighbour_api_v1::NeighbourTank> + Send + Sync>;
type Drainer = Arc<dyn Fn(&mut BlockEntityDrain<'_>) -> Option<FluidStack> + Send + Sync>;
type UpdateTag = Arc<dyn Fn(&NbtCompound) -> NbtCompound + Send + Sync>;
type Placer = Arc<dyn Fn(&mut BlockEntityPlace<'_>) + Send + Sync>;
type Signaller = Arc<dyn Fn(&mut BlockEntitySignal<'_>) + Send + Sync>;
type ItemForwarder = Arc<dyn Fn(&NbtCompound, &[Option<ItemSlot>]) -> Option<ItemForward> + Send + Sync>;
type Sided = Arc<dyn Fn(Direction) -> SidedAccess + Send + Sync>;
type TransferRule = Arc<dyn Fn(&BlockEntityTransfer<'_>) -> bool + Send + Sync>;

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
    forward: Option<Forwarder>,
    reader: Option<Reader>,
    drainer: Option<Drainer>,
    placer: Option<Placer>,
    signaller: Option<Signaller>,
    item_forward: Option<ItemForwarder>,
    sided: Option<Sided>,
    transfer_rule: Option<TransferRule>,
}

impl BlockEntityType {
    /// A block entity type for `block`, with no data, no ticker and nothing sent to clients.
    pub fn new(block: Identifier) -> Self {
        Self { id: block.clone(), block, initial_data: NbtCompound::new(), slots: 0, menu: None, tanks: Vec::new(), ticker: None, client_ticker: None, update_tag: None, filler: None, user: None, forward: None, reader: None, drainer: None, placer: None, signaller: None, item_forward: None, sided: None, transfer_rule: None }
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

    /// The same type, whose fluid storage is another block entity's (a drain in a wall answers for the controller behind it; Fabric: a
    /// `FluidStorage.SIDED` lookup that returns the storage of another position). `forward` is asked with the saved data and item slots; `None`
    /// means this block has no storage of its own either. The [`FluidFilter`] narrows what passes.
    pub fn fluid_forward(mut self, forward: impl Fn(&NbtCompound, &[Option<ItemSlot>]) -> Option<FluidForward> + Send + Sync + 'static) -> Self {
        self.forward = Some(Arc::new(forward));
        self
    }

    /// The same type, with its own way of listing its tanks (`Storage.iterator`, `IFluidHandler.getFluidInTank`): `reader` makes the list from the
    /// saved data. Without one the declared tanks are listed.
    pub fn fluid_reader(mut self, reader: impl Fn(&NbtCompound) -> Vec<super::neighbour_api_v1::NeighbourTank> + Send + Sync + 'static) -> Self {
        self.reader = Some(Arc::new(reader));
        self
    }

    /// The same type, with its own rule for fluid taken out of it (`Storage.extract`, `IFluidHandler.drain`): `drainer` answers what comes out and may
    /// change the saved data. Without one the first declared tank that holds fluid (or the one named) is drained.
    pub fn fluid_drainer(mut self, drainer: impl Fn(&mut BlockEntityDrain<'_>) -> Option<FluidStack> + Send + Sync + 'static) -> Self {
        self.drainer = Some(Arc::new(drainer));
        self
    }

    /// The same type, with `user` run when a player uses the block (right click): it may exchange items with the hand and the slots. Its
    /// answer is the use's answer; `Pass` leaves it to the game.
    pub fn on_use(mut self, user: impl Fn(&mut BlockEntityUse<'_>) -> crate::pure::base::action_result::ActionResult + Send + Sync + 'static) -> Self {
        self.user = Some(Arc::new(user));
        self
    }

    /// The same type, with `placer` run once when a player places the block (see [`BlockEntityPlace`]): it seeds the new block
    /// entity from the placed stack (a tank item's fluid, a custom name, a retexture). Without one a placed block entity starts
    /// with the type's initial data alone, and what the stack carried is lost.
    pub fn on_place(mut self, placer: impl Fn(&mut BlockEntityPlace<'_>) + Send + Sync + 'static) -> Self {
        self.placer = Some(Arc::new(placer));
        self
    }

    /// The same type, with `signaller` run when the redstone signal reaching the block changes (see [`BlockEntitySignal`]).
    /// The type must also have a ticker: the change is noticed on the block entity's own tick.
    pub fn on_signal(mut self, signaller: impl Fn(&mut BlockEntitySignal<'_>) + Send + Sync + 'static) -> Self {
        self.signaller = Some(Arc::new(signaller));
        self
    }

    /// The same type, whose item storage is another block entity's (a chute in a smeltery wall answers with the controller's
    /// inventory; the item counterpart of [`BlockEntityType::fluid_forward`], Fabric: an `ItemStorage.SIDED` lookup that returns
    /// the storage of another position). `forward` is asked with the saved data and item slots; `None` means this block has no
    /// storage of its own either. The [`ItemFilter`] narrows what passes.
    pub fn item_forward(mut self, forward: impl Fn(&NbtCompound, &[Option<ItemSlot>]) -> Option<ItemForward> + Send + Sync + 'static) -> Self {
        self.item_forward = Some(Arc::new(forward));
        self
    }

    /// The same type, with the slots each face of the block opens to automation (see [`SidedAccess`]): `access` answers for a
    /// face which slots take items and which give them through it. Without one every slot takes and gives through every face.
    pub fn sided_access(mut self, access: impl Fn(Direction) -> SidedAccess + Send + Sync + 'static) -> Self {
        self.sided = Some(Arc::new(access));
        self
    }

    /// The same type, with its own rule for the item transfers automation makes with it (see [`BlockEntityTransfer`]): `rule`
    /// is asked for each stack a hopper or another block would move in or out, with the saved data and slots at hand, and a
    /// stack it refuses does not move. Without one every transfer the faces open is allowed.
    pub fn item_transfer_rule(mut self, rule: impl Fn(&BlockEntityTransfer<'_>) -> bool + Send + Sync + 'static) -> Self {
        self.transfer_rule = Some(Arc::new(rule));
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

    /// Where the fluid storage of the block entity of `block` really is, from its [`BlockEntityType::fluid_forward`] hook; `None` when the block
    /// has no hook or the hook says it forwards nowhere.
    pub fn forward_of(block: &str, data: &NbtCompound, items: &[Option<ItemSlot>]) -> Option<FluidForward> {
        (Self::get(block)?.forward?)(data, items)
    }

    /// Whether the block has a forwarding hook (its fluid storage may be another block's).
    pub fn forwards(block: &str) -> bool {
        Self::get(block).is_some_and(|t| t.forward.is_some())
    }

    /// The tanks of the block entity of `block` as its [`BlockEntityType::fluid_reader`] lists them, or the declared tanks (with what the saved data
    /// holds) when it has none; `None` when the block has neither.
    pub fn read_tanks(block: &str, data: &NbtCompound) -> Option<Vec<super::neighbour_api_v1::NeighbourTank>> {
        let entity_type = Self::get(block)?;
        if let Some(reader) = entity_type.reader {
            return Some(reader(data));
        }
        if entity_type.tanks.is_empty() {
            return None;
        }
        Some(entity_type.tanks.iter().enumerate().map(|(index, capacity)| super::neighbour_api_v1::NeighbourTank { fluid: read_fluid(data, index), capacity: *capacity }).collect())
    }

    /// Takes up to `amount` millibuckets out of the block entity of `block` over the given data and slots: through its
    /// [`BlockEntityType::fluid_drainer`], or, without one, from the first declared tank that holds fluid (or holds `wanted`). `None` when the block has no
    /// tanks; `Some(None)` when nothing came out. With `execute` false the data and slots are left unchanged.
    pub fn drain_fluid(block: &str, pos: BlockPos, data: &mut NbtCompound, items: &mut Vec<Option<ItemSlot>>, wanted: Option<Identifier>, amount: u32, execute: bool) -> Option<Option<FluidStack>> {
        let entity_type = Self::get(block)?;
        let run = |data: &mut NbtCompound, items: &mut Vec<Option<ItemSlot>>| -> Option<Option<FluidStack>> {
            if let Some(drainer) = &entity_type.drainer {
                return Some(drainer(&mut BlockEntityDrain { pos, data, items, wanted: wanted.clone(), amount, execute }));
            }
            if entity_type.tanks.is_empty() {
                return None;
            }
            for index in 0..entity_type.tanks.len() {
                let Some(held) = read_fluid(data, index) else { continue };
                if wanted.as_ref().is_some_and(|wanted| *wanted != held.fluid) {
                    continue;
                }
                let taken = held.amount.min(amount);
                if taken == 0 {
                    return Some(None);
                }
                let left = held.amount - taken;
                write_fluid(data, index, (left > 0).then(|| FluidStack { fluid: held.fluid.clone(), amount: left, tag: held.tag.clone() }));
                return Some(Some(FluidStack { fluid: held.fluid, amount: taken, tag: held.tag }));
            }
            Some(None)
        };
        if execute {
            return run(data, items);
        }
        let (mut data, mut items) = (data.clone(), items.clone());
        run(&mut data, &mut items)
    }

    /// Runs the [`BlockEntityType::on_use`] hook of `block` for a player holding `held` (`None`: an empty hand); `None` when the block has no hook.
    pub fn use_block(block: &str, pos: BlockPos, data: &mut NbtCompound, items: &mut Vec<Option<ItemSlot>>, held: Option<ItemSlot>, sneaking: bool) -> Option<BlockUseOutcome> {
        let user = Self::get(block)?.user?;
        let mut used = BlockEntityUse { pos, data, items, held, sneaking, give: Vec::new() };
        let result = user(&mut used);
        Some(BlockUseOutcome { result, held: used.held, give: used.give })
    }

    /// Where the item storage of the block entity of `block` really is, from its [`BlockEntityType::item_forward`] hook; `None`
    /// when the block has no hook or the hook says it forwards nowhere.
    pub fn item_forward_of(block: &str, data: &NbtCompound, items: &[Option<ItemSlot>]) -> Option<ItemForward> {
        (Self::get(block)?.item_forward?)(data, items)
    }

    /// Whether the block has an item forwarding hook (its item storage may be another block's).
    pub fn item_forwards(block: &str) -> bool {
        Self::get(block).is_some_and(|t| t.item_forward.is_some())
    }

    /// The slots `face` of the block entity of `block` opens to automation, from its [`BlockEntityType::sided_access`] hook;
    /// `None` when the block has no hook (every slot opens both ways).
    pub fn sided_access(block: &str, face: Direction) -> Option<SidedAccess> {
        Some((Self::get(block)?.sided?)(face))
    }

    /// Whether the [`BlockEntityType::item_transfer_rule`] of `block` allows the transfer in the arguments; true when the
    /// block has no rule.
    pub fn item_transfer_allowed(block: &str, pos: BlockPos, data: &NbtCompound, items: &[Option<ItemSlot>], slot: usize, face: Option<Direction>, mode: TransferMode, stack: ItemSlot) -> bool {
        let Some(rule) = Self::get(block).and_then(|t| t.transfer_rule) else { return true };
        rule(&BlockEntityTransfer { pos, data, items, slot, face, mode, stack })
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

/// Whether the block's fluid storage is its saved data read through its type's own hooks (a fluid reader, a fluid
/// drainer or a forwarding hook), not tanks the host holds: the host's own tank calls cannot answer for those (a
/// forwarding block has no tanks of its own, and the host lists a block with hooks as not drainable), so the host's
/// neighbour adapter answers them through [`FluidStorage`](super::fluid_storage_api_v1::FluidStorage) instead.
pub(crate) fn fluid_served_from_data(block: &str) -> bool {
    BlockEntityTypeRegistry::get(block).is_some_and(|t| t.reader.is_some() || t.drainer.is_some() || t.forward.is_some())
}

/// Whether the block's block entity has an [`BlockEntityType::on_use`] hook (the host asks it on a right click).
pub(crate) fn has_use_hook(block: &str) -> bool {
    BlockEntityTypeRegistry::get(block).is_some_and(|t| t.user.is_some())
}

/// Whether the block's block entity has an [`BlockEntityType::on_place`] hook (the host asks it when a player places the block).
pub(crate) fn has_place_hook(block: &str) -> bool {
    BlockEntityTypeRegistry::get(block).is_some_and(|t| t.placer.is_some())
}

/// Runs the [`BlockEntityType::on_place`] hook of `block` over a fresh block entity's data and slots; true if a hook ran.
pub(crate) fn place(block: &str, pos: BlockPos, stack: ItemSlot, player: Option<ServerPlayer>, data: &mut NbtCompound, items: &mut Vec<Option<ItemSlot>>) -> bool {
    let Some(placer) = BlockEntityTypeRegistry::get(block).and_then(|t| t.placer) else { return false };
    placer(&mut BlockEntityPlace { pos, stack, player, data, items });
    true
}

/// Whether the block's block entity has an [`BlockEntityType::on_signal`] hook (the host watches its signal on its tick).
pub(crate) fn has_signal_hook(block: &str) -> bool {
    BlockEntityTypeRegistry::get(block).is_some_and(|t| t.signaller.is_some())
}

/// Runs the [`BlockEntityType::on_signal`] hook of the block entity type `entity_id` on `data`; true if a hook ran.
pub(crate) fn signal(entity_id: &str, pos: BlockPos, data: &mut NbtCompound, items: &mut Vec<Option<ItemSlot>>, strength: u8) -> bool {
    let Some(signaller) = by_entity_id(entity_id).and_then(|t| t.signaller) else { return false };
    signaller(&mut BlockEntitySignal { pos, data, items, strength, powered: strength > 0 });
    true
}

/// Whether the block's item storage is answered through its type's own hooks (an item forward, a sided access or a transfer
/// rule), not by the host's own container machinery: the host's neighbour adapter answers those through
/// [`ItemStorage`](super::item_storage_api_v1::ItemStorage) over the saved data instead, as it does for fluids.
pub(crate) fn item_served_from_data(block: &str) -> bool {
    BlockEntityTypeRegistry::get(block).is_some_and(|t| t.item_forward.is_some() || t.sided.is_some() || t.transfer_rule.is_some())
}

/// The container declaration of `block` for the host: for each of the six faces (in [`Direction::ALL`] order) the slots that
/// face opens (the union of its insert and take lists), then every slot any face takes into, then every slot any face gives
/// out of; `None` when the block declares no sided access (the host then opens every slot every way, as it does today).
pub(crate) fn container_access(block: &str) -> Option<(Vec<Vec<usize>>, Vec<usize>, Vec<usize>)> {
    let entity_type = BlockEntityTypeRegistry::get(block)?;
    let sided = entity_type.sided.as_ref()?;
    let mut faces = Vec::with_capacity(6);
    let mut inputs: Vec<usize> = Vec::new();
    let mut outputs: Vec<usize> = Vec::new();
    for face in Direction::ALL {
        let access = sided(face);
        let mut opened: Vec<usize> = access.insert.iter().chain(access.take.iter()).copied().collect();
        opened.sort_unstable();
        opened.dedup();
        faces.push(opened);
        inputs.extend(access.insert.iter().copied());
        outputs.extend(access.take.iter().copied());
    }
    inputs.sort_unstable();
    inputs.dedup();
    outputs.sort_unstable();
    outputs.dedup();
    Some((faces, inputs, outputs))
}

/// The tanks of the block's block entity as saved: each one's content and capacity; `None` if the block declares no tanks.
pub(crate) fn tank_contents(block: &str, data: &NbtCompound) -> Option<Vec<(Option<FluidStack>, u32)>> {
    let capacities = fluid_tanks(block)?;
    Some(capacities.iter().enumerate().map(|(index, &capacity)| (read_fluid(data, index), capacity)).collect())
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
        tick.set_fluid(1, Some(FluidStack { fluid: id("minecraft:lava"), amount: 250, tag: None }));
        assert_eq!(tick.fluid(0), None, "the tank before is kept, empty");
        assert_eq!(tick.fluid(1), Some(FluidStack { fluid: id("minecraft:lava"), amount: 250, tag: None }));
        tick.set_fluid(1, None);
        assert_eq!(tick.fluid(1), None);
        assert_eq!(tick.items[1].as_ref().unwrap().count, 2);
    }

    #[test]
    fn a_fluid_tag_round_trips_through_the_saved_data_and_untagged_data_reads_as_a_plain_fluid() {
        let mut tag = NbtCompound::new();
        tag.insert("Potion".to_owned(), Nbt::String("minecraft:healing".to_owned()));
        let mut data = NbtCompound::new();
        write_fluid(&mut data, 0, Some(FluidStack { fluid: id("minecraft:water"), amount: 250, tag: Some(tag.clone()) }));
        assert_eq!(read_fluid(&data, 0), Some(FluidStack { fluid: id("minecraft:water"), amount: 250, tag: Some(tag) }));
        // data written before stacks carried a tag has no Tag entry and reads back plain
        let mut old = NbtCompound::new();
        write_fluid(&mut old, 0, Some(FluidStack::new(id("minecraft:water"), 250)));
        let tanks = old.get("fluids").unwrap().as_list().unwrap();
        assert!(tanks[0].as_compound().unwrap().get("Tag").is_none());
        assert_eq!(read_fluid(&old, 0), Some(FluidStack::new(id("minecraft:water"), 250)));
        // a tagged and an untagged stack of one fluid are not the same fluid
        assert!(!FluidStack::new(id("minecraft:water"), 1).same_fluid(&FluidStack { fluid: id("minecraft:water"), amount: 1, tag: Some(NbtCompound::new()) }));
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
                if taken > 0 { fill.set_fluid(0, Some(FluidStack { fluid: fill.fluid.fluid.clone(), amount: held + taken, tag: fill.fluid.tag.clone() })); }
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
        let lava = |amount| FluidStack { fluid: id("minecraft:lava"), amount, tag: None };
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

    #[test]
    fn placement_seeds_a_block_entity_from_the_placed_stack() {
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:placed_tank")).item_slots(1).fluid_tanks([1000])
            .on_place(|placed| {
                // A tank item keeps its fluid in its custom data; the placed tank starts with it.
                let amount = placed.stack.custom_data.get_int("amount", 0);
                if amount > 0 {
                    if let Some(fluid) = placed.stack.custom_data.get("fluid").and_then(|fluid| fluid.as_str()).and_then(|fluid| Identifier::parse(fluid).ok()) {
                        placed.set_fluid(0, Some(FluidStack { fluid, amount: amount as u32, tag: None }));
                    }
                }
                placed.data.insert("placed_by".into(), Nbt::String(placed.player.as_ref().map_or("?".into(), |player| player.name.clone())));
            }));
        assert!(has_place_hook("test:placed_tank"));
        let mut stack = ItemSlot::new(id("test:placed_tank"), 1);
        stack.custom_data.insert("fluid".into(), Nbt::String("minecraft:lava".into()));
        stack.custom_data.insert("amount".into(), Nbt::Int(500));
        let (mut data, mut items) = (NbtCompound::new(), vec![None]);
        let player = ServerPlayer { uuid: 4, name: "Placer".into() };
        assert!(place("test:placed_tank", [1, 2, 3], stack, Some(player), &mut data, &mut items));
        let held = read_fluid(&data, 0).expect("the tank was seeded");
        assert_eq!((held.fluid.to_string(), held.amount), ("minecraft:lava".to_owned(), 500));
        assert_eq!(data["placed_by"], Nbt::String("Placer".into()));
        assert!(!place("test:unregistered", [0; 3], ItemSlot::new(id("minecraft:stone"), 1), None, &mut data, &mut items), "no hook, no placement");
    }

    #[test]
    fn a_signal_hook_sees_the_strength_and_whether_it_powers() {
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:swapping_tank")).item_slots(2)
            .on_signal(|signal| {
                signal.data.insert("last_signal".into(), Nbt::Int(i32::from(signal.strength)));
                if signal.powered && signal.items[0].is_some() && signal.items[1].is_none() {
                    signal.items[1] = signal.items[0].take();
                }
            }));
        assert!(has_signal_hook("test:swapping_tank"));
        let (mut data, mut items) = (NbtCompound::new(), vec![Some(ItemSlot::new(id("minecraft:bucket"), 1)), None]);
        assert!(signal("test:swapping_tank", [0; 3], &mut data, &mut items, 15));
        assert_eq!(data.get_int("last_signal", -1), 15);
        assert!(items[0].is_none() && items[1].is_some(), "powered: the slots swapped");
        items[0] = items[1].take();
        assert!(signal("test:swapping_tank", [0; 3], &mut data, &mut items, 0));
        assert!(items[0].is_some() && items[1].is_none(), "unpowered: no swap");
        assert!(!signal("test:unregistered", [0; 3], &mut data, &mut items, 15));
    }

    #[test]
    fn sided_access_and_the_transfer_rule_describe_a_containers_automation() {
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("test:casting_table")).item_slots(2)
            .sided_access(|face| match face {
                Direction::Up => SidedAccess { insert: vec![0], take: vec![] },
                Direction::Down => SidedAccess { insert: vec![], take: vec![1] },
                _ => SidedAccess::default(),
            })
            .item_transfer_rule(|transfer| {
                // Nothing goes in while a cast is running (the data says so), and only fluid containers go in at all.
                if transfer.mode == TransferMode::Insert && transfer.data.contains_key("casting") { return false; }
                transfer.mode == TransferMode::Take || transfer.stack.item.to_string().ends_with("bucket")
            }));
        assert!(item_served_from_data("test:casting_table"));
        let (faces, inputs, outputs) = container_access("test:casting_table").expect("sided access declared");
        assert_eq!(faces[Direction::Up as usize], vec![0]);
        assert_eq!(faces[Direction::Down as usize], vec![1]);
        assert_eq!(faces[Direction::North as usize], Vec::<usize>::new());
        assert_eq!((inputs, outputs), (vec![0], vec![1]));
        let bucket = ItemSlot::new(id("minecraft:bucket"), 1);
        let stick = ItemSlot::new(id("minecraft:stick"), 1);
        let items: Vec<Option<ItemSlot>> = vec![None, Some(stick.clone())];
        let mut data = NbtCompound::new();
        assert!(BlockEntityTypeRegistry::item_transfer_allowed("test:casting_table", [0; 3], &data, &items, 0, Some(Direction::Up), TransferMode::Insert, bucket.clone()));
        assert!(!BlockEntityTypeRegistry::item_transfer_allowed("test:casting_table", [0; 3], &data, &items, 0, Some(Direction::Up), TransferMode::Insert, stick.clone()), "only containers go in");
        data.insert("casting".into(), Nbt::Byte(1));
        assert!(!BlockEntityTypeRegistry::item_transfer_allowed("test:casting_table", [0; 3], &data, &items, 0, Some(Direction::Up), TransferMode::Insert, bucket), "not while casting");
        assert!(BlockEntityTypeRegistry::item_transfer_allowed("test:casting_table", [0; 3], &data, &items, 1, Some(Direction::Down), TransferMode::Take, stick), "a take is not the insert rule's business");
        assert!(BlockEntityTypeRegistry::item_transfer_allowed("test:unregistered", [0; 3], &data, &items, 0, None, TransferMode::Insert, ItemSlot::new(id("minecraft:stone"), 1)), "no rule allows everything");
    }
}
