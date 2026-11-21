//! Reaching the containers and fluid tanks of the blocks beside a ticking block entity (what a machine pulls fuel or
//! items from; NeoForge: capability lookup on a neighbour, Fabric: `ItemStorage.SIDED` / `FluidStorage.SIDED`).
//! Like [`World`](super::world_api_v1::World), the reach only exists while a host-installed scope is open; outside it
//! every query answers "nothing there", never a guess.
use super::{
    base::Identifier,
    block_api_v1::Direction,
    block_entity_api_v1::{BlockPos, FluidStack, ItemSlot},
};
use std::{cell::Cell, collections::BTreeMap, marker::PhantomData};

/// One slot of a neighbouring container as the queried side reaches it.
#[derive(Clone, Debug, PartialEq)]
pub struct NeighbourSlot {
    /// The slot's number, for [`Neighbours::extract_item`].
    pub slot: usize,
    pub stack: Option<ItemSlot>,
    /// Whether that stack can be taken through the queried side.
    pub can_take: bool,
}

/// One tank of a neighbouring block as the queried side reaches it.
#[derive(Clone, Debug, PartialEq)]
pub struct NeighbourTank {
    /// What it holds, `None` when empty.
    pub fluid: Option<FluidStack>,
    /// Millibuckets it can hold.
    pub capacity: u32,
}

/// What the game can do for neighbours. The host implements it; [`MemoryNeighbourOps`] is a small world for tests.
pub trait NeighbourOps {
    /// The slots of the container at `pos` that its side `face` reaches; `None` where there is no container (or its
    /// chunk is not loaded).
    fn item_slots(&mut self, pos: BlockPos, face: Direction) -> Option<Vec<NeighbourSlot>>;
    /// Takes up to `count` items out of `slot` through `face`; `None` when nothing could be taken.
    fn extract_item(&mut self, pos: BlockPos, face: Direction, slot: usize, count: u32) -> Option<ItemSlot>;
    /// Puts `stack` into the container through `face`; the part that did not fit comes back.
    fn insert_item(&mut self, pos: BlockPos, face: Direction, stack: ItemSlot) -> Option<ItemSlot>;
    /// The tanks of the block at `pos` that `face` reaches; `None` where there are none.
    fn fluid_tanks(&mut self, pos: BlockPos, face: Direction) -> Option<Vec<NeighbourTank>>;
    /// Drains up to `amount` millibuckets of one fluid (the first that is there) through `face`.
    fn drain_fluid(&mut self, pos: BlockPos, face: Direction, amount: u32) -> Option<FluidStack>;
    /// Puts up to `stack.amount` millibuckets of the fluid into the block at `pos` through `face`; the amount taken (0 where nothing answers).
    /// With `execute` false it only reports what would be taken. A block with its own fluid rule (`BlockEntityType::fluid_filler`) decides itself.
    fn fill_fluid(&mut self, _pos: BlockPos, _face: Direction, _stack: &FluidStack, _execute: bool) -> u32 {
        0
    }
    /// The state of the block at `pos` as text (`minecraft:stone` or `ns:name[key=value]`), `None` where it is not loaded. The ticking block's
    /// own position answers too (a faucet reads its own facing).
    fn block_state(&mut self, _pos: BlockPos) -> Option<String> {
        None
    }
}

thread_local! {
    static OPS: Cell<Option<*mut dyn NeighbourOps>> = const { Cell::new(None) };
}

struct Scope<'a> {
    previous: Option<*mut dyn NeighbourOps>,
    _borrow: PhantomData<&'a mut dyn NeighbourOps>,
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        OPS.with(|slot| slot.set(self.previous));
    }
}

/// Runs `f` with `ops` answering every [`Neighbours`] query made (on this thread) during `f`.
pub fn with_neighbours<R>(ops: &mut dyn NeighbourOps, f: impl FnOnce() -> R) -> R {
    // SAFETY: the pointer is only dereferenced while `f` runs, inside the lifetime of `ops`; `Scope` clears it on every exit.
    let raw: *mut dyn NeighbourOps = unsafe { std::mem::transmute::<&mut dyn NeighbourOps, *mut dyn NeighbourOps>(ops) };
    let previous = OPS.with(|slot| slot.replace(Some(raw)));
    let _scope = Scope { previous, _borrow: PhantomData };
    f()
}

fn with_current<R>(default: R, f: impl FnOnce(&mut dyn NeighbourOps) -> R) -> R {
    match OPS.with(Cell::get) {
        // SAFETY: see `with_neighbours`: the pointer is valid for as long as it is stored.
        Some(raw) => f(unsafe { &mut *raw }),
        None => default,
    }
}

/// Queries on the blocks around a block entity. Every method answers `None` (or the whole stack back) outside a scope.
pub struct Neighbours;

impl Neighbours {
    /// The block next to `pos` on `side`, and the face of it that looks back at `pos`.
    pub fn beside(pos: BlockPos, side: Direction) -> (BlockPos, Direction) {
        let (dx, dy, dz) = match side {
            Direction::Down => (0, -1, 0),
            Direction::Up => (0, 1, 0),
            Direction::North => (0, 0, -1),
            Direction::South => (0, 0, 1),
            Direction::West => (-1, 0, 0),
            Direction::East => (1, 0, 0),
        };
        let back = match side {
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
            Direction::East => Direction::West,
        };
        ([pos[0] + dx, pos[1] + dy, pos[2] + dz], back)
    }

    pub fn item_slots(pos: BlockPos, face: Direction) -> Option<Vec<NeighbourSlot>> {
        with_current(None, |ops| ops.item_slots(pos, face))
    }

    pub fn extract_item(pos: BlockPos, face: Direction, slot: usize, count: u32) -> Option<ItemSlot> {
        with_current(None, |ops| ops.extract_item(pos, face, slot, count))
    }

    /// The part of `stack` that did not fit; the whole stack when no container answers.
    pub fn insert_item(pos: BlockPos, face: Direction, stack: ItemSlot) -> Option<ItemSlot> {
        let kept = stack.clone();
        with_current(Some(kept), |ops| ops.insert_item(pos, face, stack))
    }

    pub fn fluid_tanks(pos: BlockPos, face: Direction) -> Option<Vec<NeighbourTank>> {
        with_current(None, |ops| ops.fluid_tanks(pos, face))
    }

    pub fn drain_fluid(pos: BlockPos, face: Direction, amount: u32) -> Option<FluidStack> {
        with_current(None, |ops| ops.drain_fluid(pos, face, amount))
    }

    /// How much of `stack` the block at `pos` takes through `face`: 0 outside a scope or where nothing answers. `execute` false only asks.
    pub fn fill_fluid(pos: BlockPos, face: Direction, stack: &FluidStack, execute: bool) -> u32 {
        with_current(0, |ops| ops.fill_fluid(pos, face, stack, execute))
    }

    /// The state of the block at `pos`, `None` outside a scope, where it is not loaded or when the host's text does not parse.
    pub fn block_state(pos: BlockPos) -> Option<super::world_api_v1::BlockState> {
        with_current(None, |ops| ops.block_state(pos)).and_then(|text| super::world_api_v1::BlockState::parse(&text).ok())
    }
}

/// A small world of containers and tanks, for tests: every face reaches every slot and tank.
#[derive(Default)]
pub struct MemoryNeighbourOps {
    pub containers: BTreeMap<BlockPos, Vec<Option<ItemSlot>>>,
    /// Tanks by position: contents and capacity.
    pub tanks: BTreeMap<BlockPos, Vec<(Option<FluidStack>, u32)>>,
    /// Block states by position, as text.
    pub states: BTreeMap<BlockPos, String>,
}

impl NeighbourOps for MemoryNeighbourOps {
    fn item_slots(&mut self, pos: BlockPos, _: Direction) -> Option<Vec<NeighbourSlot>> {
        let slots = self.containers.get(&pos)?;
        Some(slots.iter().enumerate().map(|(slot, stack)| NeighbourSlot { slot, stack: stack.clone(), can_take: stack.is_some() }).collect())
    }

    fn extract_item(&mut self, pos: BlockPos, _: Direction, slot: usize, count: u32) -> Option<ItemSlot> {
        let held = self.containers.get_mut(&pos)?.get_mut(slot)?;
        let stack = held.as_mut()?;
        let taken = count.min(stack.count);
        if taken == 0 {
            return None;
        }
        let mut out = stack.clone();
        out.count = taken;
        stack.count -= taken;
        if stack.count == 0 {
            *held = None;
        }
        Some(out)
    }

    fn insert_item(&mut self, pos: BlockPos, _: Direction, mut stack: ItemSlot) -> Option<ItemSlot> {
        let Some(slots) = self.containers.get_mut(&pos) else { return Some(stack) };
        for held in slots.iter_mut() {
            match held {
                Some(existing) if existing.item == stack.item && existing.custom_data == stack.custom_data && existing.damage == stack.damage => {
                    let room = 64u32.saturating_sub(existing.count).min(stack.count);
                    existing.count += room;
                    stack.count -= room;
                }
                None if stack.count > 0 => {
                    *held = Some(stack.clone());
                    stack.count = 0;
                }
                _ => {}
            }
            if stack.count == 0 {
                return None;
            }
        }
        Some(stack)
    }

    fn fluid_tanks(&mut self, pos: BlockPos, _: Direction) -> Option<Vec<NeighbourTank>> {
        Some(self.tanks.get(&pos)?.iter().map(|(fluid, capacity)| NeighbourTank { fluid: fluid.clone(), capacity: *capacity }).collect())
    }

    fn drain_fluid(&mut self, pos: BlockPos, _: Direction, amount: u32) -> Option<FluidStack> {
        let tanks = self.tanks.get_mut(&pos)?;
        let (fluid, _) = tanks.iter_mut().find(|(fluid, _)| fluid.is_some())?;
        let stack = fluid.as_mut()?;
        let taken = amount.min(stack.amount);
        if taken == 0 {
            return None;
        }
        let out = FluidStack { fluid: Identifier::parse(&stack.fluid.to_string()).ok()?, amount: taken, tag: stack.tag.clone() };
        stack.amount -= taken;
        if stack.amount == 0 {
            *fluid = None;
        }
        Some(out)
    }

    fn fill_fluid(&mut self, pos: BlockPos, _: Direction, stack: &FluidStack, execute: bool) -> u32 {
        let Some(tanks) = self.tanks.get_mut(&pos) else { return 0 };
        for (held, capacity) in tanks.iter_mut() {
            let room = match held {
                Some(own) if !own.same_fluid(stack) => continue,
                Some(own) => capacity.saturating_sub(own.amount),
                None => *capacity,
            };
            let taken = room.min(stack.amount);
            if taken == 0 { continue; }
            if execute {
                match held {
                    Some(own) => own.amount += taken,
                    None => *held = Some(FluidStack { fluid: stack.fluid.clone(), amount: taken, tag: stack.tag.clone() }),
                }
            }
            return taken;
        }
        0
    }

    fn block_state(&mut self, pos: BlockPos) -> Option<String> {
        self.states.get(&pos).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    #[test]
    fn queries_answer_nothing_outside_a_scope_and_reach_the_scoped_world_inside_it() {
        let coal = ItemSlot::new(id("minecraft:coal"), 3);
        assert_eq!(Neighbours::item_slots([0, 63, 0], Direction::Up), None);
        assert_eq!(Neighbours::insert_item([0, 63, 0], Direction::Up, coal.clone()), Some(coal.clone()), "nothing took it");
        let mut world = MemoryNeighbourOps::default();
        world.containers.insert([0, 63, 0], vec![None, Some(coal.clone())]);
        world.tanks.insert([0, 63, 0], vec![(Some(FluidStack { fluid: id("minecraft:lava"), amount: 100, tag: None }), 4000)]);
        with_neighbours(&mut world, || {
            let (below, face) = Neighbours::beside([0, 64, 0], Direction::Down);
            assert_eq!((below, face), ([0, 63, 0], Direction::Up));
            let slots = Neighbours::item_slots(below, face).unwrap();
            assert_eq!(slots.iter().map(|s| (s.slot, s.can_take)).collect::<Vec<_>>(), [(0, false), (1, true)]);
            assert_eq!(Neighbours::extract_item(below, face, 1, 1).unwrap().count, 1);
            assert_eq!(Neighbours::drain_fluid(below, face, 30).unwrap().amount, 30);
            assert_eq!(Neighbours::fluid_tanks(below, face).unwrap()[0].fluid.as_ref().unwrap().amount, 70);
            assert!(Neighbours::insert_item(below, face, ItemSlot::new(id("minecraft:coal"), 1)).is_none(), "it fit");
        });
        // one coal was taken from slot 1 (3 -> 2) and the inserted one went to the first empty slot
        assert_eq!(world.containers[&[0, 63, 0]][1].as_ref().unwrap().count, 2);
        assert_eq!(world.containers[&[0, 63, 0]][0].as_ref().unwrap().count, 1);
        assert_eq!(Neighbours::drain_fluid([0, 63, 0], Direction::Up, 1), None, "the scope is closed again");
    }

    #[test]
    fn filling_respects_the_tank_fluid_and_capacity_and_states_are_read_by_position() {
        let mut world = MemoryNeighbourOps::default();
        world.tanks.insert([0, 63, 0], vec![(None, 100), (Some(FluidStack { fluid: id("minecraft:water"), amount: 10, tag: None }), 50)]);
        world.states.insert([0, 64, 0], "test:faucet[facing=north]".to_owned());
        let lava = FluidStack { fluid: id("minecraft:lava"), amount: 130, tag: None };
        assert_eq!(Neighbours::fill_fluid([0, 63, 0], Direction::Up, &lava, true), 0, "no scope");
        with_neighbours(&mut world, || {
            assert_eq!(Neighbours::fill_fluid([0, 63, 0], Direction::Up, &lava, false), 100);
            assert_eq!(Neighbours::fill_fluid([0, 63, 0], Direction::Up, &lava, true), 100, "only the empty tank takes lava");
            assert_eq!(Neighbours::fill_fluid([0, 63, 0], Direction::Up, &FluidStack { fluid: id("minecraft:water"), amount: 100, tag: None }, true), 40, "the water tank has 40 left");
            assert_eq!(Neighbours::fill_fluid([9, 9, 9], Direction::Up, &lava, true), 0);
            assert_eq!(Neighbours::block_state([0, 64, 0]).unwrap().property("facing"), Some("north"));
            assert_eq!(Neighbours::block_state([1, 64, 0]), None);
        });
        assert_eq!(world.tanks[&[0, 63, 0]][0].0.as_ref().unwrap().amount, 100, "the simulation added nothing, the real fill 100");
    }

}
