# Neighbouring containers and tanks

A machine that burns fuel from the block below it, or pulls items from the block beside it, asks the neighbour through `Neighbours`: its item slots, taking items out, putting items in, and draining fluid. Every method needs a scope opened by the host (`with_neighbours`); outside one the answers are "nothing there", so a machine simply finds no fuel. This page first walks through using it, then lists every item in the [Reference](#reference).

## Basic Example

A furnace-like block that burns one coal from the container below it:

```rust
use pluto_lib::{Direction, Identifier, ItemSlot, MemoryNeighbourOps, Neighbours, with_neighbours};

fn burn_one_coal(pos: [i32; 3]) -> bool {
    let (below, face) = Neighbours::beside(pos, Direction::Down);
    let Some(slots) = Neighbours::item_slots(below, face) else { return false };
    for slot in slots {
        if slot.can_take && slot.stack.as_ref().is_some_and(|stack| stack.item.path() == "coal") {
            return Neighbours::extract_item(below, face, slot.slot, 1).is_some();
        }
    }
    false
}

let mut world = MemoryNeighbourOps::default();
world.containers.insert([0, 63, 0], vec![Some(ItemSlot::new(Identifier::parse("minecraft:coal").unwrap(), 2))]);
assert!(!burn_one_coal([0, 64, 0]), "no scope, no fuel");
with_neighbours(&mut world, || assert!(burn_one_coal([0, 64, 0])));
assert_eq!(world.containers[&[0, 63, 0]][0].as_ref().unwrap().count, 1);
```

> **Use with Caution.** Host adapter written, not yet compiled or game-verified. `block_entity_tick` in `hooks.rs` opens this scope (with the world and ticking-dimension scopes) around every ticker over the host's `container_slots`, `extract_item`, `insert_item`, `fluid_tanks`, `fluid_extract`, `fluid_insert` and `level_block_state`; `container_slots` is the one host reach the pinned Horizon lacks as a `Host` method (its slot type is crate-private), so PlutoLib adds a wrapper to `native_items.rs` plus a `Host` helper fragment (direct host attachment `pluto-block-entity-container-reach`, guarded like the others). `fill_fluid` with `execute` goes through the host's `fluid_insert` (a block with a `fluid_filler` answers through the host's custom fluid hooks, see [block-entity](block-entity.md)); asked only (`execute` false) it asks the filler on a copy of the block entity, and for other blocks sums the room of their fillable tanks (a cauldron's thirds are not modelled). `block_state` reads the level's block state. Not covered: a tank another mod's block declares through the host's own custom fluid hooks is read through `fluid_tanks` like any other. The ticking block entity itself is never reachable through its own position (the host keeps it out of the world while it ticks), so `container_slots`/`fluid_tanks` of the ticker's own position answer nothing, while `block_state` of it answers. A block whose fluid storage is its saved data read through its type's hooks (a `fluid_reader`, `fluid_drainer` or `fluid_forward`) is answered through `FluidStorage` over the host's block entities instead of the host's tank calls, which cannot serve it (a forwarding block has no tanks of its own, and a hooked block is listed as not drainable); declared tanks without hooks keep the host's calls.

## Reference

### `Neighbours`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The queries on the blocks around a ticking block entity.

> Every method answers "nothing there" (`None`, or the whole stack back) outside a scope.

```rust
use pluto_lib::{Direction, Neighbours};
assert_eq!(Neighbours::item_slots([0, 63, 0], Direction::Up), None, "no scope, nothing there");
```

### `Neighbours::beside`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The block next to `pos` on `side` and the face of it that looks back.

`fn beside(pos: BlockPos, side: Direction) -> (BlockPos, Direction)`

> Pass both answers to the other queries: the neighbour's position, and the face of it
> that looks back at your block, so a container is always queried through the side that
> touches you.

```rust
use pluto_lib::{Direction, Neighbours};
assert_eq!(Neighbours::beside([5, 70, 5], Direction::Down), ([5, 69, 5], Direction::Up));
```

### `Neighbours::item_slots`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Lists the slots `face` reaches (`slot`, `stack`, `can_take`), `None` where there is no container.

`fn item_slots(pos: BlockPos, face: Direction) -> Option<Vec<NeighbourSlot>>`

> Each `NeighbourSlot` names one slot the face reaches, the stack currently in it, and
> whether items may be taken from it. A position with no container answers `None` rather
> than an empty list.

```rust
use pluto_lib::{Direction, Identifier, ItemSlot, MemoryNeighbourOps, Neighbours, with_neighbours};
let mut world = MemoryNeighbourOps::default();
world.containers.insert([0, 63, 0], vec![Some(ItemSlot::new(Identifier::parse("minecraft:coal").unwrap(), 2))]);
let slots = with_neighbours(&mut world, || Neighbours::item_slots([0, 63, 0], Direction::Up)).unwrap();
assert!(slots[0].can_take && slots[0].slot == 0 && slots[0].stack.is_some());
```

### `Neighbours::extract_item`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Takes up to `count` items out of a slot.

`fn extract_item(pos: BlockPos, face: Direction, slot: usize, count: u32) -> Option<ItemSlot>`

> The taken items come back as an `ItemSlot`; asking for more than the slot holds takes
> what is there. A position or face with nothing to take answers `None`.

```rust
use pluto_lib::{Direction, Identifier, ItemSlot, MemoryNeighbourOps, Neighbours, with_neighbours};
let mut world = MemoryNeighbourOps::default();
world.containers.insert([0, 63, 0], vec![Some(ItemSlot::new(Identifier::parse("minecraft:coal").unwrap(), 2))]);
let taken = with_neighbours(&mut world, || Neighbours::extract_item([0, 63, 0], Direction::Up, 0, 1));
assert_eq!(taken.unwrap().count, 1);
```

### `Neighbours::insert_item`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Returns what did not fit (the whole stack when nothing answers).

`fn insert_item(pos: BlockPos, face: Direction, stack: ItemSlot) -> Option<ItemSlot>`

> What fits is moved into the container and what does not comes back, so a returned stack
> equal to the one you passed means nothing was accepted. When the whole stack fits, the
> answer is `None`.

```rust
use pluto_lib::{Direction, Identifier, ItemSlot, Neighbours};
let stack = ItemSlot::new(Identifier::parse("minecraft:coal").unwrap(), 1);
assert_eq!(Neighbours::insert_item([0, 0, 0], Direction::Up, stack.clone()), Some(stack));
```

### `Neighbours::fluid_tanks`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Reads the tanks `face` reaches (`fluid`, `capacity`).

`fn fluid_tanks(pos: BlockPos, face: Direction) -> Option<Vec<NeighbourTank>>`

> Each `NeighbourTank` reports the fluid it currently holds and the tank's capacity, so
> you can judge a transfer before attempting it. A face that reaches no tank answers
> `None`.

```rust
use pluto_lib::{Direction, Neighbours};
assert!(Neighbours::fluid_tanks([0, 0, 0], Direction::Up).is_none());
```

### `Neighbours::drain_fluid`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Takes up to `amount` millibuckets of the first fluid there.

`fn drain_fluid(pos: BlockPos, face: Direction, amount: u32) -> Option<FluidStack>`

> Up to `amount` millibuckets are taken, of the first fluid the face reaches, and returned
> as a `FluidStack`. When there is no fluid to take, the answer is `None`.

```rust
use pluto_lib::{Direction, Neighbours};
assert_eq!(Neighbours::drain_fluid([0, 0, 0], Direction::Up, 50), None);
```

### `NeighbourSlot`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One slot of a neighbouring container.

> `slot` (its number, for `extract_item`), `stack` (what it holds) and `can_take` (whether
> the queried side may take it).

```rust
use pluto_lib::NeighbourSlot;
let slot = NeighbourSlot { slot: 0, stack: None, can_take: false };
assert!(slot.stack.is_none() && !slot.can_take && slot.slot == 0);
```

### `NeighbourTank`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One tank of a neighbouring block.

> `fluid` is `None` for an empty tank, so an empty tank and a missing one are told apart
> by whether a `NeighbourTank` comes back at all. `capacity` is the tank's size in
> millibuckets, full or not.

```rust
use pluto_lib::NeighbourTank;
let tank = NeighbourTank { fluid: None, capacity: 4000 };
assert!(tank.fluid.is_none() && tank.capacity == 4000);
```

### `NeighbourOps`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

What the game does for neighbours: the item, tank, fill and block state queries above.

> The host implements it; `fill_fluid` and `block_state` have defaults that answer
> nothing, so older implementations keep working.

```rust
use pluto_lib::{Direction, FluidStack, ItemSlot, NeighbourOps, NeighbourSlot, NeighbourTank};
struct Nothing;
impl NeighbourOps for Nothing {
    fn item_slots(&mut self, _: [i32; 3], _: Direction) -> Option<Vec<NeighbourSlot>> { None }
    fn extract_item(&mut self, _: [i32; 3], _: Direction, _: usize, _: u32) -> Option<ItemSlot> { None }
    fn insert_item(&mut self, _: [i32; 3], _: Direction, stack: ItemSlot) -> Option<ItemSlot> { Some(stack) }
    fn fluid_tanks(&mut self, _: [i32; 3], _: Direction) -> Option<Vec<NeighbourTank>> { None }
    fn drain_fluid(&mut self, _: [i32; 3], _: Direction, _: u32) -> Option<FluidStack> { None }
}
assert_eq!(Nothing.block_state([0; 3]), None);
```

### `with_neighbours`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Opens the scope on this thread.

`fn with_neighbours<R>(ops: &mut dyn NeighbourOps, f: impl FnOnce() -> R) -> R`

> The scope is per-thread and lasts only for the call: queries made on other threads, or
> after `with_neighbours` returns, are not answered by `ops`. This is how a host — or a
> test's `MemoryNeighbourOps` — is plugged in behind the `Neighbours` functions.

```rust
use pluto_lib::{Direction, MemoryNeighbourOps, Neighbours, with_neighbours};
let mut world = MemoryNeighbourOps::default();
assert!(with_neighbours(&mut world, || Neighbours::item_slots([0; 3], Direction::Up)).is_none(), "an empty world has no container");
```

### `MemoryNeighbourOps`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A world of containers (`containers`), tanks with their capacity (`tanks`) and block states as text (`states`) by position, for tests.

> Build one holding the containers, tanks and states your test needs and drive the
> neighbour operations against it instead of a live world. Because every face reaches
> everything, it exercises the transfer logic itself rather than face handling.

```rust
use pluto_lib::{Direction, FluidStack, Identifier, MemoryNeighbourOps, Neighbours, with_neighbours};
let mut world = MemoryNeighbourOps::default();
world.tanks.insert([0, 0, 0], vec![(Some(FluidStack { fluid: Identifier::parse("minecraft:lava").unwrap(), amount: 100, tag: None }), 4000)]);
world.states.insert([0, 1, 0], "minecraft:stone".to_owned());
assert!(world.containers.is_empty());
let drained = with_neighbours(&mut world, || Neighbours::drain_fluid([0, 0, 0], Direction::Up, 30));
assert_eq!(drained.unwrap().amount, 30);
```

### `Neighbours::fill_fluid`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

How many millibuckets of `stack` the block at `pos` takes through `face` (0 outside a scope or where nothing answers); with `execute` false it only asks.

`fn fill_fluid(pos: BlockPos, face: Direction, stack: &FluidStack, execute: bool) -> u32`

> A faucet drains the tank behind it and fills the one below this way. A block with its
> own fluid rule (`BlockEntityType::fluid_filler`) decides how much itself; the host
> adapter that installs this is the same one the caution above names.

```rust
use pluto_lib::{Direction, FluidStack, Identifier, MemoryNeighbourOps, Neighbours, with_neighbours};
let mut world = MemoryNeighbourOps::default();
world.tanks.insert([0, 63, 0], vec![(None, 90)]);
let lava = FluidStack { fluid: Identifier::parse("minecraft:lava").unwrap(), amount: 500, tag: None };
assert_eq!(Neighbours::fill_fluid([0, 63, 0], Direction::Up, &lava, true), 0, "no scope");
let taken = with_neighbours(&mut world, || Neighbours::fill_fluid([0, 63, 0], Direction::Up, &lava, true));
assert_eq!(taken, 90, "the tank holds 90");
```

### `Neighbours::block_state`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The state of the block at `pos` (a ticking block entity reads its own block's properties here, such as a faucet's `facing`).

`fn block_state(pos: BlockPos) -> Option<BlockState>`

> It answers `None` outside a scope, or where the block at `pos` is not loaded. A ticking
> block entity reads its own block's properties here, such as a faucet's `facing`.

```rust
use pluto_lib::{MemoryNeighbourOps, Neighbours, with_neighbours};
let mut world = MemoryNeighbourOps::default();
world.states.insert([0, 64, 0], "example:faucet[facing=north]".to_owned());
let state = with_neighbours(&mut world, || Neighbours::block_state([0, 64, 0])).unwrap();
assert_eq!(state.property("facing"), Some("north"));
```
