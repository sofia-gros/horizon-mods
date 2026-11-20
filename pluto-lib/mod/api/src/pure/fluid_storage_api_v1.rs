//! The fluid storage of registered block entities, followed through forwarding blocks: what a host adapter answers a neighbour's fluid query with
//! (Fabric: `FluidStorage.SIDED` lookups; NeoForge: the `IFluidHandler` capability), written over the hooks of
//! [`BlockEntityType`](super::block_entity_api_v1::BlockEntityType) and a [`BlockEntityStore`] that holds the saved data. A block with a
//! `fluid_forward` hook (a drain in a smeltery wall) answers with the storage of the block it points at, narrowed by its filter; a block with
//! `fluid_reader`, `fluid_filler` and `fluid_drainer` hooks answers by them; any other block answers from its declared tanks.
use super::{
    base::Identifier,
    block_entity_api_v1::{read_fluid, write_fluid, BlockEntityTypeRegistry, BlockPos, FluidFilter, FluidStack, ItemSlot},
    neighbour_api_v1::NeighbourTank,
};
use crate::pure::base::NbtCompound;
use std::collections::BTreeMap;

/// The saved data of the block entities of a world, as the host holds them.
pub trait BlockEntityStore {
    /// The block id at `pos` (`tconstruct:seared_drain`), `None` where the chunk is not loaded.
    fn block_at(&mut self, pos: BlockPos) -> Option<String>;
    /// The saved data and item slots of the block entity at `pos`.
    fn read(&mut self, pos: BlockPos) -> Option<(NbtCompound, Vec<Option<ItemSlot>>)>;
    /// Stores changed data and slots back.
    fn write(&mut self, pos: BlockPos, data: NbtCompound, items: Vec<Option<ItemSlot>>);
}

/// A world of block entities for tests.
#[derive(Default)]
pub struct MemoryBlockEntities {
    /// Block ids by position.
    pub blocks: BTreeMap<BlockPos, String>,
    /// Saved data and slots by position.
    pub entities: BTreeMap<BlockPos, (NbtCompound, Vec<Option<ItemSlot>>)>,
}

impl BlockEntityStore for MemoryBlockEntities {
    fn block_at(&mut self, pos: BlockPos) -> Option<String> {
        self.blocks.get(&pos).cloned()
    }
    fn read(&mut self, pos: BlockPos) -> Option<(NbtCompound, Vec<Option<ItemSlot>>)> {
        self.entities.get(&pos).cloned()
    }
    fn write(&mut self, pos: BlockPos, data: NbtCompound, items: Vec<Option<ItemSlot>>) {
        self.entities.insert(pos, (data, items));
    }
}

/// Fluid queries over a [`BlockEntityStore`].
pub struct FluidStorage<'a> {
    store: &'a mut dyn BlockEntityStore,
}

/// Forwards followed before giving up (a forward cycle answers nothing).
const MAX_HOPS: usize = 4;

struct Resolved {
    pos: BlockPos,
    block: String,
    data: NbtCompound,
    items: Vec<Option<ItemSlot>>,
    filter: FluidFilter,
}

impl<'a> FluidStorage<'a> {
    /// Queries over `store`.
    pub fn new(store: &'a mut dyn BlockEntityStore) -> Self {
        Self { store }
    }

    /// Follows forwards from `pos` to the block entity that holds the storage, with the filter that applies on the way.
    fn resolve(&mut self, mut pos: BlockPos) -> Option<Resolved> {
        let mut filter = FluidFilter::Any;
        for _ in 0..=MAX_HOPS {
            let block = self.store.block_at(pos)?;
            let (data, items) = self.store.read(pos)?;
            match BlockEntityTypeRegistry::forward_of(&block, &data, &items) {
                Some(forward) => {
                    if filter == FluidFilter::Any {
                        filter = forward.filter;
                    }
                    pos = forward.target;
                }
                None if BlockEntityTypeRegistry::forwards(&block) => return None,
                None => return Some(Resolved { pos, block, data, items, filter }),
            }
        }
        None
    }

    /// The tanks the block at `pos` shows, as its filter lists them (`DuctTankWrapper.getTankMapping`); `None` where there is no storage.
    pub fn tanks(&mut self, pos: BlockPos) -> Option<Vec<NeighbourTank>> {
        let found = self.resolve(pos)?;
        let tanks = BlockEntityTypeRegistry::read_tanks(&found.block, &found.data)?;
        Some(match &found.filter {
            FluidFilter::Any => tanks,
            FluidFilter::Only(filter) => tanks.into_iter().filter(|tank| tank.fluid.as_ref().is_none_or(|held| held.fluid == *filter)).collect(),
            FluidFilter::Closed => match tanks.last() {
                Some(last) if last.fluid.is_none() => vec![last.clone()],
                _ => Vec::new(),
            },
        })
    }

    /// How much of `stack` the block at `pos` takes; with `execute` false it only asks.
    pub fn fill(&mut self, pos: BlockPos, stack: &FluidStack, execute: bool) -> u32 {
        if stack.amount == 0 {
            return 0;
        }
        let Some(mut found) = self.resolve(pos) else { return 0 };
        match &found.filter {
            FluidFilter::Closed => return 0,
            FluidFilter::Only(filter) if *filter != stack.fluid => return 0,
            _ => {}
        }
        let taken = match BlockEntityTypeRegistry::fill_fluid(&found.block, found.pos, &mut found.data, &mut found.items, stack.clone(), None, execute) {
            Some(taken) => taken,
            None => {
                let Some(tanks) = BlockEntityTypeRegistry::read_tanks(&found.block, &found.data) else { return 0 };
                let mut taken = 0;
                for (index, tank) in tanks.iter().enumerate() {
                    let room = match &tank.fluid {
                        Some(held) if held.fluid != stack.fluid => continue,
                        Some(held) => tank.capacity.saturating_sub(held.amount),
                        None => tank.capacity,
                    };
                    let part = room.min(stack.amount);
                    if part == 0 {
                        continue;
                    }
                    if execute {
                        let held = read_fluid(&found.data, index).map_or(0, |held| held.amount);
                        write_fluid(&mut found.data, index, Some(FluidStack { fluid: stack.fluid.clone(), amount: held + part }));
                    }
                    taken = part;
                    break;
                }
                taken
            }
        };
        if execute && taken > 0 {
            self.store.write(found.pos, found.data, found.items);
        }
        taken
    }

    /// Takes up to `amount` millibuckets out of the block at `pos`: of `wanted` when named (`drain(FluidStack)`), otherwise of what the storage
    /// gives first (`drain(maxDrain)`). A filter that names a fluid takes that fluid only.
    pub fn drain(&mut self, pos: BlockPos, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack> {
        let mut found = self.resolve(pos)?;
        let wanted = match &found.filter {
            FluidFilter::Closed => return None,
            FluidFilter::Only(filter) => {
                if wanted.is_some_and(|wanted| wanted != filter) {
                    return None;
                }
                Some(filter.clone())
            }
            FluidFilter::Any => wanted.cloned(),
        };
        let drained = BlockEntityTypeRegistry::drain_fluid(&found.block, found.pos, &mut found.data, &mut found.items, wanted, amount, execute)??;
        if execute {
            self.store.write(found.pos, found.data, found.items);
        }
        Some(drained)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::base::{Nbt, NbtCompoundExt};
    use crate::pure::block_entity_api_v1::{BlockEntityType, FluidForward};

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    fn fluid(name: &str, amount: u32) -> FluidStack {
        FluidStack { fluid: id(name), amount }
    }

    /// A pool block with two declared tanks, a gate that forwards to it (position in its data) letting `filter` through.
    fn world() -> MemoryBlockEntities {
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("fs:pool")).fluid_tanks([1000, 1000]));
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("fs:gate")).item_slots(1).fluid_forward(|data, items| {
            let target = match data.get("to") {
                Some(Nbt::IntArray(at)) if at.len() == 3 => [at[0], at[1], at[2]],
                _ => return None,
            };
            let filter = match items.first().and_then(Option::as_ref) {
                Some(stack) if stack.item.path() == "lava_bucket" => FluidFilter::Only(id("minecraft:lava")),
                Some(_) => FluidFilter::Closed,
                None => FluidFilter::Any,
            };
            Some(FluidForward { target, filter })
        }));
        let mut world = MemoryBlockEntities::default();
        world.blocks.insert([0, 0, 0], "fs:pool".into());
        let mut pool = NbtCompound::new();
        write_fluid(&mut pool, 0, Some(fluid("minecraft:water", 400)));
        write_fluid(&mut pool, 1, Some(fluid("minecraft:lava", 300)));
        world.entities.insert([0, 0, 0], (pool, Vec::new()));
        world.blocks.insert([1, 0, 0], "fs:gate".into());
        let mut gate = NbtCompound::new();
        gate.insert("to".into(), Nbt::IntArray(vec![0, 0, 0]));
        world.entities.insert([1, 0, 0], (gate, vec![None]));
        world
    }

    #[test]
    fn a_gate_answers_with_the_storage_it_points_at_and_an_unattached_gate_answers_nothing() {
        let mut world = world();
        let mut storage = FluidStorage::new(&mut world);
        assert_eq!(storage.tanks([1, 0, 0]).unwrap().len(), 2);
        assert_eq!(storage.drain([1, 0, 0], None, 50, true), Some(fluid("minecraft:water", 50)), "the first fluid");
        assert_eq!(storage.drain([1, 0, 0], Some(&id("minecraft:lava")), 500, false), Some(fluid("minecraft:lava", 300)), "a named fluid; asking changes nothing");
        assert_eq!(storage.fill([1, 0, 0], &fluid("minecraft:water", 900), true), 650, "the water tank had room for 650");
        assert_eq!(read_fluid(&world.entities[&[0, 0, 0]].0, 0).unwrap().amount, 1000);
        world.entities.get_mut(&[1, 0, 0]).unwrap().0.clear();
        assert_eq!(FluidStorage::new(&mut world).tanks([1, 0, 0]), None, "a forward to nowhere is not the gate's own storage");
        assert_eq!(FluidStorage::new(&mut world).fill([1, 0, 0], &fluid("minecraft:water", 5), true), 0);
    }

    #[test]
    fn a_filter_lets_one_fluid_through_or_none_and_lists_what_the_original_duct_listed() {
        let mut world = world();
        world.entities.get_mut(&[1, 0, 0]).unwrap().1[0] = Some(ItemSlot::new(id("minecraft:lava_bucket"), 1));
        let mut storage = FluidStorage::new(&mut world);
        let tanks = storage.tanks([1, 0, 0]).unwrap();
        assert_eq!(tanks.len(), 1, "only the tank that holds lava is listed");
        assert_eq!(storage.fill([1, 0, 0], &fluid("minecraft:water", 10), true), 0);
        assert_eq!(storage.drain([1, 0, 0], Some(&id("minecraft:water")), 10, true), None);
        assert_eq!(storage.drain([1, 0, 0], None, 100, true), Some(fluid("minecraft:lava", 100)), "an unnamed drain takes the filter's fluid");
        assert_eq!(storage.fill([1, 0, 0], &fluid("minecraft:lava", 50), true), 50);
        // a closed duct (an item that holds no fluid it knows) lists the last tank only while it is empty, and moves nothing
        world.entities.get_mut(&[1, 0, 0]).unwrap().1[0] = Some(ItemSlot::new(id("minecraft:stick"), 1));
        let mut storage = FluidStorage::new(&mut world);
        assert_eq!(storage.tanks([1, 0, 0]).unwrap().len(), 0, "the last tank holds lava");
        assert_eq!(storage.fill([1, 0, 0], &fluid("minecraft:lava", 1), true), 0);
        assert_eq!(storage.drain([1, 0, 0], None, 1, true), None);
        let mut data = world.entities[&[0, 0, 0]].0.clone();
        write_fluid(&mut data, 1, None);
        world.entities.get_mut(&[0, 0, 0]).unwrap().0 = data;
        assert_eq!(FluidStorage::new(&mut world).tanks([1, 0, 0]).unwrap().len(), 1);
    }

    #[test]
    fn readers_fillers_and_drainers_replace_the_declared_tanks_and_a_forward_cycle_answers_nothing() {
        // one shared tank of 1000 over every fluid, saved under "tank"
        BlockEntityTypeRegistry::register(
            BlockEntityType::new(id("fs:vat"))
                .fluid_reader(|data| vec![NeighbourTank { fluid: data.get_compound("tank").map(|tag| FluidStack { fluid: id(tag.get_string("fluid").unwrap()), amount: tag.get_int("amount", 0) as u32 }), capacity: 1000 }])
                .fluid_filler(|fill| {
                    let held = fill.data.get_compound("tank").map_or(0, |tag| tag.get_int("amount", 0) as u32);
                    let taken = fill.fluid.amount.min(1000 - held);
                    if taken > 0 {
                        let mut tag = NbtCompound::new();
                        tag.insert("fluid".into(), Nbt::String(fill.fluid.fluid.to_string()));
                        tag.insert("amount".into(), Nbt::Int((held + taken) as i32));
                        fill.data.insert("tank".into(), Nbt::Compound(tag));
                    }
                    taken
                })
                .fluid_drainer(|drain| {
                    let tag = drain.data.get_compound("tank")?.clone();
                    let held = tag.get_int("amount", 0) as u32;
                    let taken = held.min(drain.amount);
                    let mut left = tag.clone();
                    left.insert("amount".into(), Nbt::Int((held - taken) as i32));
                    drain.data.insert("tank".into(), Nbt::Compound(left));
                    Some(FluidStack { fluid: id(tag.get_string("fluid")?), amount: taken })
                }),
        );
        BlockEntityTypeRegistry::register(BlockEntityType::new(id("fs:loop")).fluid_forward(|_, _| Some(FluidForward { target: [9, 9, 9], filter: FluidFilter::Any })));
        let mut world = MemoryBlockEntities::default();
        world.blocks.insert([0, 0, 0], "fs:vat".into());
        world.entities.insert([0, 0, 0], (NbtCompound::new(), Vec::new()));
        world.blocks.insert([9, 9, 9], "fs:loop".into());
        world.entities.insert([9, 9, 9], (NbtCompound::new(), Vec::new()));
        let mut storage = FluidStorage::new(&mut world);
        assert_eq!(storage.fill([0, 0, 0], &fluid("minecraft:water", 1500), false), 1000);
        assert_eq!(storage.fill([0, 0, 0], &fluid("minecraft:water", 700), true), 700);
        assert_eq!(storage.fill([0, 0, 0], &fluid("minecraft:lava", 700), true), 300, "the hook decides the capacity");
        assert_eq!(storage.drain([0, 0, 0], None, 250, true), Some(fluid("minecraft:lava", 250)), "the hook overwrote the fluid");
        assert_eq!(storage.tanks([0, 0, 0]).unwrap()[0].fluid.as_ref().unwrap().amount, 750);
        assert_eq!(storage.tanks([9, 9, 9]), None, "a block that forwards to itself or to nothing has no storage");
        assert_eq!(storage.tanks([5, 5, 5]), None);
    }
}
