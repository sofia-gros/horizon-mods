//! The fluid interface of the smeltery and foundry, from SlimeKnights SmelteryInputOutputBlockEntity, DrainBlockEntity, DuctBlockEntity,
//! DuctItemHandler, DuctTankWrapper and the controllers' `ISmelteryTankHandler` capability (MIT; see LICENSE): a drain in the wall
//! answers a faucet, channel or pipe with the controller's tank; a duct does the same for one fluid only, the fluid of the container in
//! its slot. The structure tells these blocks their master (`master` in their saved data, written when the structure forms and cleared
//! when a block leaves it), and a servant that already answers to another controller blocks a second structure (`isValidMaster`).
//! The controllers' own tank answers through PlutoLib's fluid hooks over the saved `tank`. Retexturing's data path is
//! ported (see `retextured`): the placement hook keeps a placed stack's texture in the block entity's data, and the
//! loot function restores it on the drop; only the picture is missing, as no model-data surface lets a placed block's
//! texture follow its block entity. The fluids the drain and the duct display are ported in the
//! port's form: the drain's ticker mirrors its master's first tank fluid into its own data under `display` (the controller
//! notifies its drains in the original), the duct's renderer reads the fluid of the container in its slot from its synced
//! items, and `rendering::render_io` draws both. A chute forwards its master's item inventory on every
//! face, as ChuteBlockEntity forwards the master's item handler capability (its facing is ignored), so an
//! item query on a chute is answered by the controller's inventory through `item_forward`. A neighbour's fluid query on these blocks is answered by
//! PlutoLib's host adapter through `FluidStorage` over the loaded block entities, because their storage is their saved data read through
//! these hooks, which the host's own tank calls cannot serve.
use crate::{
    smeltery_tank::SmelteryTank,
    tank::{Fluid, Tank},
};
use pluto_lib::{
    ActionResult, BlockEntityDrain, BlockEntityFill, BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, BlockEntityUse, BlockPos, Direction, FluidFilter, FluidForward,
    FluidItems, FluidStack, FluidTarget, Identifier, ItemFilter, ItemForward, ItemSlot, Nbt, NbtCompound, NbtCompoundExt, NeighbourTank, Neighbours, ScreenHandlerRegistry,
    ScreenHandlerType, ScreenLayout, TransferDirection,
};

const DUCT_BACKGROUND: &str = "tconstruct:gui/duct";

pub(crate) const DRAINS: [&str; 2] = ["seared_drain", "scorched_drain"];
pub(crate) const DUCTS: [&str; 2] = ["seared_duct", "scorched_duct"];
pub(crate) const CHUTES: [&str; 2] = ["seared_chute", "scorched_chute"];
/// The saved position of the controller a servant answers to; empty (or missing) for none.
pub(crate) const TAG_MASTER: &str = "master";

/// Whether a block is a servant that keeps its master in its block entity (the structure blocks with no block entity of their own only
/// carry `in_structure`).
pub(crate) fn keeps_master(block: &str) -> bool {
    DRAINS.contains(&block) || DUCTS.contains(&block) || CHUTES.contains(&block) || crate::tank::capacity_of(block).is_some()
}

/// The master a servant was told (`ServantTileEntity.getMasterPos`).
pub(crate) fn master_of(data: &NbtCompound) -> Option<BlockPos> {
    match data.get(TAG_MASTER) {
        Some(Nbt::IntArray(at)) if at.len() == 3 => Some([at[0], at[1], at[2]]),
        _ => None,
    }
}

/// `setMaster`: the data to merge into a servant's saved data; `None` frees it.
pub(crate) fn master_data(master: Option<BlockPos>) -> NbtCompound {
    let mut data = NbtCompound::new();
    data.insert(TAG_MASTER.to_owned(), Nbt::IntArray(master.map_or_else(Vec::new, |at| at.to_vec())));
    data
}

/// `IServantLogic.isValidMaster`: a servant that has a master other than `candidate` is foreign to it, unless that master is gone
/// (`master_alive` false).
pub(crate) fn foreign_to(data: &NbtCompound, candidate: BlockPos, master_alive: impl FnOnce(BlockPos) -> bool) -> bool {
    match master_of(data) {
        Some(master) if master != candidate => master_alive(master),
        _ => false,
    }
}

// --- the controllers' tank

/// `getTanks`, `getFluidInTank` and `getTankCapacity` of the smeltery tank: a tank per fluid and the empty space.
pub(crate) fn read_controller(data: &NbtCompound) -> Vec<NeighbourTank> {
    let mut tank = SmelteryTank::default();
    if let Some(tag) = data.get_compound("tank") { tank.read(tag); }
    let mut tanks: Vec<NeighbourTank> = tank.fluids().iter().enumerate()
        .map(|(index, fluid)| NeighbourTank { fluid: Some(FluidStack { fluid: fluid.fluid.clone(), amount: fluid.amount.max(0) as u32, tag: None }), capacity: tank.tank_capacity(index).max(0) as u32 })
        .collect();
    if tank.contained() < tank.capacity() { tanks.push(NeighbourTank { fluid: None, capacity: (tank.capacity() - tank.contained()) as u32 }); }
    tanks
}

pub(crate) fn fill_controller(fill: &mut BlockEntityFill<'_>) -> u32 {
    let Some(tag) = fill.data.get_compound("tank").cloned() else { return 0 };
    let mut tank = SmelteryTank::default();
    tank.read(&tag);
    let filled = tank.fill(&fill.fluid.fluid, fill.fluid.amount.min(i32::MAX as u32) as i32, true);
    if filled > 0 { fill.data.insert("tank".to_owned(), Nbt::Compound(tank.write())); }
    filled.max(0) as u32
}

pub(crate) fn drain_controller(drain: &mut BlockEntityDrain<'_>) -> Option<FluidStack> {
    let tag = drain.data.get_compound("tank")?.clone();
    let mut tank = SmelteryTank::default();
    tank.read(&tag);
    let amount = drain.amount.min(i32::MAX as u32) as i32;
    let out = match &drain.wanted { Some(fluid) => tank.drain_fluid(fluid, amount, true), None => tank.drain(amount, true) }?;
    drain.data.insert("tank".to_owned(), Nbt::Compound(tank.write()));
    Some(FluidStack { fluid: out.fluid, amount: out.amount.max(0) as u32, tag: None })
}

// --- the duct's filter

/// What the original's `DUCT_CONTAINERS` item tag lists (`tconstruct:duct_containers`), and the filled buckets whose remainder is in it.
const CONTAINERS: [&str; 4] = ["minecraft:bucket", "tconstruct:copper_can", "tconstruct:seared_lantern", "tconstruct:scorched_lantern"];
const BUCKETS: [(&str, &str); 2] = [("minecraft:water_bucket", "minecraft:water"), ("minecraft:lava_bucket", "minecraft:lava")];

/// The fluid in a container item (`FluidUtil.getFluidHandler(stack).getFluidInTank(0)`): the `tank` of an item of this port, otherwise
/// a vanilla bucket's. The host cannot give a fluid-carrying item's contents yet (see the module notes), so other containers are empty.
pub(crate) fn container_fluid(stack: &ItemSlot) -> Option<Fluid> {
    let tank = Tank::read(&stack.custom_data, i32::MAX);
    if tank.fluid.is_some() { return tank.fluid; }
    let item = stack.item.to_string();
    BUCKETS.iter().find(|(bucket, _)| *bucket == item).map(|(_, fluid)| Fluid { fluid: Identifier::parse(fluid).expect("valid fluid id"), amount: 1000 })
}

/// `DuctItemHandler.isItemValid`: the item or its container is in the tag, and it holds fluid.
pub(crate) fn duct_accepts(stack: &ItemSlot) -> bool {
    let item = stack.item.to_string();
    let remainder = BUCKETS.iter().any(|(bucket, _)| *bucket == item).then_some("minecraft:bucket");
    (CONTAINERS.contains(&item.as_str()) || remainder.is_some_and(|remainder| CONTAINERS.contains(&remainder))) && container_fluid(stack).is_some()
}

/// What a slot of the duct's window takes by the item alone (the window cannot see the stack's data): the tag's containers and the
/// buckets that hold fluid, not the empty bucket.
pub(crate) fn duct_slot_takes(item: &str) -> bool {
    item != "minecraft:bucket" && (CONTAINERS.contains(&item) || BUCKETS.iter().any(|(bucket, _)| *bucket == item))
}

/// `DuctItemHandler.getFluid` as a filter: the container's fluid lets only that fluid through, an empty duct nothing.
pub(crate) fn duct_filter(items: &[Option<ItemSlot>]) -> FluidFilter {
    match items.first().and_then(Option::as_ref).and_then(container_fluid) {
        Some(fluid) => FluidFilter::Only(fluid.fluid),
        None => FluidFilter::Closed,
    }
}

/// The saved display fluid of a drain: the master's first tank fluid, as a `FluidName`/`Amount` compound.
pub(crate) const TAG_DISPLAY: &str = "display";

/// The fluid a drain shows, read from its saved display compound.
pub(crate) fn display_of(data: &NbtCompound) -> Option<Fluid> {
    let tag = data.get_compound(TAG_DISPLAY)?;
    let name = tag.get_string("FluidName")?;
    let amount = tag.get_int("Amount", 0) as i32;
    if amount <= 0 || name == "minecraft:empty" { return None; }
    Some(Fluid { fluid: Identifier::parse(name).ok()?, amount })
}

/// Keeps a drain's `display` at the given fluid (none clears it); answers whether anything changed.
/// `IDisplayFluidListener.notifyDisplayFluidUpdated` arrives as a notification in the original — the
/// controller calls its drains when the tank's order changes — so this is the mirror it would write.
pub(crate) fn set_display(data: &mut NbtCompound, fluid: Option<FluidStack>) -> bool {
    let wanted = fluid.filter(|fluid| fluid.amount > 0).map(|fluid| {
        let mut tag = NbtCompound::new();
        tag.insert("FluidName".to_owned(), Nbt::String(fluid.fluid.to_string()));
        tag.insert("Amount".to_owned(), Nbt::Int(fluid.amount.min(i32::MAX as u32) as i32));
        tag
    });
    let changed = data.get_compound(TAG_DISPLAY) != wanted.as_ref();
    if changed {
        match wanted {
            Some(tag) => { data.insert(TAG_DISPLAY.to_owned(), Nbt::Compound(tag)); }
            None => { data.remove(TAG_DISPLAY); }
        }
    }
    changed
}

/// A drain's ticker: its display follows its master's tank, read through the neighbour queries like the
/// smeltery window reads its fuel tanks (the master's first fluid is its display fluid).
fn tick_drain(tick: &mut BlockEntityTick<'_>) {
    let first = master_of(tick.data)
        .and_then(|master| Neighbours::fluid_tanks(master, Direction::Up))
        .and_then(|tanks| tanks.into_iter().next())
        .and_then(|tank| tank.fluid);
    set_display(tick.data, first);
}

/// The storage a drain's use trades with: its master's tank, reached through the drain's own fluid
/// forwarding, as the bucket trade on a clicked block reaches its storage (the neighbour drain takes
/// the first fluid it finds and cannot be asked first, so a drain that names its fluid reads the
/// tanks for its answer and an executed drain of a different fluid is put straight back).
struct DrainTarget {
    pos: BlockPos,
    face: Direction,
}

impl FluidTarget for DrainTarget {
    fn fill(&mut self, stack: &FluidStack, execute: bool) -> u32 {
        Neighbours::fill_fluid(self.pos, self.face, stack, execute)
    }

    fn drain(&mut self, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack> {
        if !execute {
            let tanks = Neighbours::fluid_tanks(self.pos, self.face)?;
            return tanks.into_iter().find_map(|tank| {
                let held = tank.fluid?;
                wanted.is_none_or(|wanted| *wanted == held.fluid).then(|| FluidStack { amount: held.amount.min(amount), ..held })
            });
        }
        let drained = Neighbours::drain_fluid(self.pos, self.face, amount)?;
        if wanted.is_some_and(|wanted| *wanted != drained.fluid) {
            Neighbours::fill_fluid(self.pos, self.face, &drained, true);
            return None;
        }
        Some(drained)
    }
}

/// `SearedDrainBlock.use`: the held container trades with the tank the drain forwards to
/// (`FluidTransferHelper.interactWithTank` at the drain's position, the handler on the side its
/// facing turns away from); `Success` when a transfer happened, `Pass` when none did, so a click
/// holding anything else — a faucet to place against the drain, above all — goes on to the item's
/// own handling. A drain with no master forwards nowhere, so its clicks all pass.
fn use_drain(used: &mut BlockEntityUse<'_>) -> ActionResult {
    let facing = Neighbours::block_state(used.pos).and_then(|state| state.property("facing").and_then(Direction::parse)).unwrap_or(Direction::North);
    let mut target = DrainTarget { pos: used.pos, face: facing.opposite() };
    let Some(done) = FluidItems::interact(used.held.as_ref(), &mut target, TransferDirection::Auto) else { return ActionResult::Pass };
    used.held = done.held;
    used.give.extend(done.give);
    ActionResult::Success
}

pub(crate) fn register() {
    for name in DRAINS {
        let block = Identifier::new("tconstruct", name).expect("valid drain id");
        // a drain answers with the whole tank of its master, and shows its first fluid
        BlockEntityTypeRegistry::register(BlockEntityType::new(block).sync_data().on_place(crate::retextured::keep_texture).ticker(tick_drain).fluid_forward(|data, _| master_of(data).map(|target| FluidForward { target, filter: FluidFilter::Any })).on_use(use_drain));
    }
    for name in CHUTES {
        let block = Identifier::new("tconstruct", name).expect("valid chute id");
        // ChuteBlockEntity: the master's item handler answers on every face, the facing ignored.
        BlockEntityTypeRegistry::register(BlockEntityType::new(block).sync_data().on_place(crate::retextured::keep_texture).item_forward(|data, _| master_of(data).map(|target| ItemForward { target, filter: ItemFilter::Any })));
    }
    for name in DUCTS {
        let block = Identifier::new("tconstruct", name).expect("valid duct id");
        // SingleItemContainerMenu: one slot at (80, 20) over a 133 high window, the inventory at 51
        let layout = ScreenLayout::new(176.0, 133.0).inventory_at(8.0, 51.0).with_own_background().slot(80.0, 20.0);
        ScreenHandlerRegistry::register(
            ScreenHandlerType::new(block.clone(), layout)
                .on_slot_rule(|rule| { rule.may_place = duct_slot_takes(rule.item); rule.max_stack = 1; })
                .on_accepts(|put| duct_slot_takes(put.item))
                .on_draw(|draw| { draw.canvas.sprite(DUCT_BACKGROUND, [0.0, 0.0, 176.0, 133.0], [draw.origin[0], draw.origin[1], 176.0, 133.0]); }),
        );
        BlockEntityTypeRegistry::register(
            BlockEntityType::new(block).item_slots(1).menu(&format!("tconstruct:{name}")).sync_data().on_place(crate::retextured::keep_texture)
                .fluid_forward(|data, items| master_of(data).map(|target| FluidForward { target, filter: duct_filter(items) })),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faucet::{self, Plumbing};
    use pluto_lib::{BlockEntityStore, Direction, FluidStorage, MemoryBlockEntities, NeighbourOps, NeighbourSlot};

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn fluid(name: &str, amount: u32) -> FluidStack { FluidStack { fluid: id(name), amount, tag: None } }

    /// The controller at [0, 1, 0] with 3 fluids of 1000 in a tank of 4000, a drain and a duct beside it, and a pool of 90 under a faucet.
    fn world() -> MemoryBlockEntities {
        crate::smeltery_menu::register();
        register();
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("tconstruct:test_pool")).fluid_tanks([90]));
        let mut tank = SmelteryTank::default();
        tank.set_capacity(4000);
        tank.fill(&id("tconstruct:molten_iron"), 1000, true);
        tank.fill(&id("tconstruct:molten_gold"), 1000, true);
        let mut world = MemoryBlockEntities::default();
        let mut controller = NbtCompound::new();
        controller.insert("tank".into(), Nbt::Compound(tank.write()));
        world.blocks.insert([0, 1, 0], "tconstruct:smeltery_controller".into());
        world.entities.insert([0, 1, 0], (controller, vec![None; 256]));
        world.blocks.insert([1, 1, 0], "tconstruct:seared_drain".into());
        world.entities.insert([1, 1, 0], (master_data(Some([0, 1, 0])), Vec::new()));
        world.blocks.insert([0, 1, 1], "tconstruct:seared_duct".into());
        world.entities.insert([0, 1, 1], (master_data(Some([0, 1, 0])), vec![None]));
        world.blocks.insert([5, 0, 0], "tconstruct:test_pool".into());
        world.entities.insert([5, 0, 0], (NbtCompound::new(), Vec::new()));
        world
    }

    #[test]
    fn a_drain_shows_and_gives_the_controllers_tank_and_a_free_drain_shows_nothing() {
        let mut world = world();
        let mut storage = FluidStorage::new(&mut world);
        let tanks = storage.tanks([1, 1, 0]).unwrap();
        assert_eq!(tanks.len(), 3, "two fluids and the empty space");
        assert_eq!((tanks[0].capacity, tanks[2].fluid.is_none(), tanks[2].capacity), (3000, true, 2000), "a fluid's tank is its amount and the space");
        assert_eq!(storage.drain([1, 1, 0], None, 90, true), Some(fluid("tconstruct:molten_iron", 90)), "from the first fluid");
        assert_eq!(storage.drain([1, 1, 0], Some(&id("tconstruct:molten_gold")), 90, true), Some(fluid("tconstruct:molten_gold", 90)), "a named fluid from anywhere");
        assert_eq!(storage.fill([1, 1, 0], &fluid("tconstruct:molten_copper", 5000), true), 2180, "the shared capacity less what is inside");
        let (tag, _) = world.read([0, 1, 0]).unwrap();
        let mut tank = SmelteryTank::default();
        tank.read(tag.get_compound("tank").unwrap());
        assert_eq!((tank.contained(), tank.fluids().len()), (4000, 3));
        world.write([1, 1, 0], master_data(None), Vec::new());
        assert_eq!(FluidStorage::new(&mut world).tanks([1, 1, 0]), None, "a drain that was freed has no tank");
    }

    #[test]
    fn a_duct_passes_only_the_fluid_of_the_container_in_its_slot() {
        let mut world = world();
        assert_eq!(FluidStorage::new(&mut world).drain([0, 1, 1], None, 50, true), None, "an empty duct is closed");
        let mut bucket = ItemSlot::new(id("minecraft:lava_bucket"), 1);
        assert!(duct_accepts(&bucket) && !duct_accepts(&ItemSlot::new(id("minecraft:bucket"), 1)) && !duct_accepts(&ItemSlot::new(id("minecraft:stick"), 1)));
        assert!(duct_slot_takes("minecraft:water_bucket") && !duct_slot_takes("minecraft:bucket") && duct_slot_takes("tconstruct:copper_can"));
        world.entities.get_mut(&[0, 1, 1]).unwrap().1[0] = Some(bucket.clone());
        let mut storage = FluidStorage::new(&mut world);
        assert_eq!(storage.drain([0, 1, 1], None, 50, true), None, "the controller holds no lava");
        // a container of this port holds its fluid under `tank` (what the tank items do)
        bucket = ItemSlot::new(id("tconstruct:copper_can"), 1);
        Tank { fluid: Some(Fluid { fluid: id("tconstruct:molten_gold"), amount: 90 }), capacity: 90 }.write(&mut bucket.custom_data);
        assert_eq!(container_fluid(&bucket).unwrap().fluid, id("tconstruct:molten_gold"));
        world.entities.get_mut(&[0, 1, 1]).unwrap().1[0] = Some(bucket);
        let mut storage = FluidStorage::new(&mut world);
        assert_eq!(storage.tanks([0, 1, 1]).unwrap().len(), 2, "the gold tank and the empty space");
        assert_eq!(storage.drain([0, 1, 1], None, 20, true), Some(fluid("tconstruct:molten_gold", 20)), "an unnamed drain takes the filter's fluid");
        assert_eq!(storage.drain([0, 1, 1], Some(&id("tconstruct:molten_iron")), 20, true), None);
        assert_eq!(storage.fill([0, 1, 1], &fluid("tconstruct:molten_iron", 20), true), 0);
    }

    #[test]
    fn a_chute_is_registered_with_a_block_entity_of_its_own() {
        register();
        for name in CHUTES {
            let kind = BlockEntityTypeRegistry::get(&format!("tconstruct:{name}")).expect("chute block entity type");
            assert_eq!(kind.block.to_string(), format!("tconstruct:{name}"));
        }
    }

    #[test]
    fn a_drains_display_is_its_masters_first_fluid_and_follows_the_order() {
        let mut data = master_data(Some([0, 1, 0]));
        assert!(set_display(&mut data, Some(fluid("tconstruct:molten_iron", 1000))));
        assert_eq!(display_of(&data), Some(Fluid { fluid: id("tconstruct:molten_iron"), amount: 1000 }));
        assert!(!set_display(&mut data, Some(fluid("tconstruct:molten_iron", 1000))), "an unchanged display is not rewritten");
        assert!(set_display(&mut data, Some(fluid("tconstruct:molten_gold", 500))), "the tank's order changed: the new first fluid shows");
        assert_eq!(display_of(&data), Some(Fluid { fluid: id("tconstruct:molten_gold"), amount: 500 }));
        assert!(set_display(&mut data, None), "an empty tank shows nothing");
        assert_eq!(display_of(&data), None);
    }

    #[test]
    fn a_drain_tick_reads_its_master_through_the_neighbour_queries() {
        let mut world = world();
        let mut data = master_data(Some([0, 1, 0]));
        let mut items: Vec<Option<ItemSlot>> = Vec::new();
        {
            let mut reach = Reach(&mut world);
            pluto_lib::with_neighbours(&mut reach, || {
                tick_drain(&mut BlockEntityTick { pos: [1, 1, 0], data: &mut data, items: &mut items });
            });
        }
        assert_eq!(display_of(&data), Some(Fluid { fluid: id("tconstruct:molten_iron"), amount: 1000 }), "the controller's first fluid");
        // the first fluid drained away: the second becomes the display
        assert_eq!(FluidStorage::new(&mut world).drain([0, 1, 0], None, 1000, true), Some(fluid("tconstruct:molten_iron", 1000)));
        {
            let mut reach = Reach(&mut world);
            pluto_lib::with_neighbours(&mut reach, || {
                tick_drain(&mut BlockEntityTick { pos: [1, 1, 0], data: &mut data, items: &mut items });
            });
        }
        assert_eq!(display_of(&data), Some(Fluid { fluid: id("tconstruct:molten_gold"), amount: 1000 }));
        // a freed drain shows nothing
        let mut freed = master_data(None);
        freed.insert(TAG_DISPLAY.to_owned(), data.get(TAG_DISPLAY).unwrap().clone());
        {
            let mut reach = Reach(&mut world);
            pluto_lib::with_neighbours(&mut reach, || {
                tick_drain(&mut BlockEntityTick { pos: [1, 1, 0], data: &mut freed, items: &mut items });
            });
        }
        assert_eq!(display_of(&freed), None);
    }

    #[test]
    fn a_drain_use_trades_a_held_container_with_its_masters_tank_and_passes_any_other_click() {
        crate::fluids::register();
        crate::fluid_items::register();
        let mut world = world();
        let mut data = master_data(Some([0, 1, 0]));
        let mut items: Vec<Option<ItemSlot>> = Vec::new();
        let mut reach = Reach(&mut world);
        // a faucet in hand is no fluid container: nothing moves and the click passes, so the faucet
        // goes on to be placed against the drain instead of being swallowed (#370)
        let passed = pluto_lib::with_neighbours(&mut reach, || {
            let mut used = BlockEntityUse { pos: [1, 1, 0], data: &mut data, items: &mut items, held: Some(ItemSlot::new(id("tconstruct:seared_faucet"), 1)), sneaking: false, give: Vec::new() };
            (use_drain(&mut used), used.held.map(|held| held.item.to_string()))
        });
        assert_eq!(passed, (ActionResult::Pass, Some("tconstruct:seared_faucet".to_owned())));
        // an empty bucket fills with the controller's first fluid through the drain
        let filled = pluto_lib::with_neighbours(&mut reach, || {
            let mut used = BlockEntityUse { pos: [1, 1, 0], data: &mut data, items: &mut items, held: Some(ItemSlot::new(id("minecraft:bucket"), 1)), sneaking: false, give: Vec::new() };
            (use_drain(&mut used), used.held.map(|held| held.item.to_string()))
        });
        assert_eq!(filled, (ActionResult::Success, Some("tconstruct:molten_iron_bucket".to_owned())));
        // a filled bucket empties into the same tank through the drain
        let emptied = pluto_lib::with_neighbours(&mut reach, || {
            let mut used = BlockEntityUse { pos: [1, 1, 0], data: &mut data, items: &mut items, held: Some(ItemSlot::new(id("minecraft:lava_bucket"), 1)), sneaking: false, give: Vec::new() };
            (use_drain(&mut used), used.held.map(|held| held.item.to_string()))
        });
        assert_eq!(emptied, (ActionResult::Success, Some("minecraft:bucket".to_owned())));
        drop(reach);
        let (tag, _) = world.read([0, 1, 0]).unwrap();
        let mut tank = SmelteryTank::default();
        tank.read(tag.get_compound("tank").unwrap());
        let amounts: Vec<(String, i32)> = tank.fluids().iter().filter(|fluid| fluid.amount > 0).map(|fluid| (fluid.fluid.to_string(), fluid.amount)).collect();
        assert_eq!(amounts, vec![("tconstruct:molten_gold".to_owned(), 1000), ("minecraft:lava".to_owned(), 1000)], "the iron left in the bucket, the lava arrived, the gold is untouched");
        // a freed drain forwards nowhere: even a container click passes
        world.write([1, 1, 0], master_data(None), Vec::new());
        let mut freed = master_data(None);
        let mut reach = Reach(&mut world);
        let passed = pluto_lib::with_neighbours(&mut reach, || {
            let mut used = BlockEntityUse { pos: [1, 1, 0], data: &mut freed, items: &mut items, held: Some(ItemSlot::new(id("minecraft:bucket"), 1)), sneaking: false, give: Vec::new() };
            use_drain(&mut used)
        });
        assert_eq!(passed, ActionResult::Pass);
    }

    /// A neighbour reach that answers from the fluid storage of the world: what the host's adapter has to do.
    struct Reach<'a>(&'a mut MemoryBlockEntities);
    impl NeighbourOps for Reach<'_> {
        fn item_slots(&mut self, _: [i32; 3], _: Direction) -> Option<Vec<NeighbourSlot>> { None }
        fn extract_item(&mut self, _: [i32; 3], _: Direction, _: usize, _: u32) -> Option<ItemSlot> { None }
        fn insert_item(&mut self, _: [i32; 3], _: Direction, stack: ItemSlot) -> Option<ItemSlot> { Some(stack) }
        fn fluid_tanks(&mut self, pos: [i32; 3], _: Direction) -> Option<Vec<NeighbourTank>> { FluidStorage::new(self.0).tanks(pos) }
        fn drain_fluid(&mut self, pos: [i32; 3], _: Direction, amount: u32) -> Option<FluidStack> { FluidStorage::new(self.0).drain(pos, None, amount, true) }
        fn fill_fluid(&mut self, pos: [i32; 3], _: Direction, stack: &FluidStack, execute: bool) -> u32 { FluidStorage::new(self.0).fill(pos, stack, execute) }
    }

    #[test]
    fn a_faucet_beside_a_drain_pours_the_smeltery_into_the_block_below() {
        let mut world = world();
        // the drain is at [1, 1, 0]; a faucet at [2, 1, 0] facing east has its input to the west, and pours into [2, 0, 0], where the pool is
        world.blocks.insert([2, 0, 0], "tconstruct:test_pool".into());
        world.entities.insert([2, 0, 0], (NbtCompound::new(), Vec::new()));
        let mut faucet = faucet::State::default();
        let mut reach = Reach(&mut world);
        pluto_lib::with_neighbours(&mut reach, || {
            let mut around = faucet::Around::of([2, 1, 0], Direction::East);
            assert!(around.has_input() && around.has_output());
            faucet.activate(&mut around);
            for _ in 0..12 { faucet.tick(&mut around); }
        });
        let (pool, _) = world.read([2, 0, 0]).unwrap();
        let held = pluto_lib::BlockEntityTypeRegistry::read_tanks("tconstruct:test_pool", &pool).unwrap();
        assert_eq!(held[0].fluid, Some(fluid("tconstruct:molten_iron", 90)), "an ingot's worth of the first fluid, a nugget a tick, until the target is full");
        let (controller, _) = world.read([0, 1, 0]).unwrap();
        let mut tank = SmelteryTank::default();
        tank.read(controller.get_compound("tank").unwrap());
        assert_eq!(tank.fluids()[0].amount, 910);
    }

    #[test]
    fn a_structure_tells_its_servants_who_the_master_is_and_a_servant_of_another_controller_is_foreign() {
        let data = master_data(Some([0, 1, 0]));
        assert_eq!(master_of(&data), Some([0, 1, 0]));
        assert_eq!(master_of(&master_data(None)), None, "freed");
        assert_eq!(master_of(&NbtCompound::new()), None);
        assert!(!foreign_to(&data, [0, 1, 0], |_| true), "our own servant");
        assert!(foreign_to(&data, [9, 9, 9], |_| true), "answers to a controller that is still there");
        assert!(!foreign_to(&data, [9, 9, 9], |_| false), "its controller is gone");
        assert!(!foreign_to(&NbtCompound::new(), [9, 9, 9], |_| true), "no master yet");
        assert!(keeps_master("seared_drain") && keeps_master("seared_chute") && keeps_master("scorched_chute") && keeps_master("scorched_fuel_tank") && keeps_master("seared_lantern") && keeps_master("scorched_lantern") && !keeps_master("seared_stone"));
    }
}
