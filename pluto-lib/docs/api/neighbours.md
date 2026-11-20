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

> **Use with Caution.** Source-ready only: the trait, the scope and the in-memory test world are implemented and tested, but the host adapter that installs a scope around block entity tickers (over the game's `native_container_slots`, `native_extract_item`, `native_insert_item`, `native_fluid_tanks` and `native_fluid_extract`) is not written yet, because the generated `Host` signatures can only be checked by a native build. Until it lands, a ticker's neighbour queries all answer nothing. The ticking block entity itself is never reachable through its own position (the host keeps it out of the world while it ticks).

## Reference

### `Neighbours::beside`

`fn beside(pos: BlockPos, side: Direction) -> (BlockPos, Direction)`. The block next to `pos` on `side` and the face of it that looks back; pass both to the other queries.

```rust
use pluto_lib::{Direction, Neighbours};
assert_eq!(Neighbours::beside([5, 70, 5], Direction::Down), ([5, 69, 5], Direction::Up));
```

### `Neighbours::item_slots`, `extract_item`, `insert_item`

`item_slots(pos, face) -> Option<Vec<NeighbourSlot>>` lists the slots `face` reaches (`slot`, `stack`, `can_take`); `extract_item(pos, face, slot, count) -> Option<ItemSlot>` takes up to `count`; `insert_item(pos, face, stack) -> Option<ItemSlot>` returns what did not fit (the whole stack when nothing answers).

```rust
use pluto_lib::{Direction, Identifier, ItemSlot, Neighbours};
let stack = ItemSlot::new(Identifier::parse("minecraft:coal").unwrap(), 1);
assert_eq!(Neighbours::insert_item([0, 0, 0], Direction::Up, stack.clone()), Some(stack));
```

### `Neighbours::fluid_tanks`, `drain_fluid`

`fluid_tanks(pos, face) -> Option<Vec<NeighbourTank>>` reads the tanks (`fluid`, `capacity`); `drain_fluid(pos, face, amount) -> Option<FluidStack>` takes up to `amount` millibuckets of the first fluid there.

```rust
use pluto_lib::{Direction, Neighbours};
assert_eq!(Neighbours::drain_fluid([0, 0, 0], Direction::Up, 50), None);
```

### `NeighbourOps`, `with_neighbours`, `MemoryNeighbourOps`

`NeighbourOps` is what the game does for neighbours; `with_neighbours(&mut ops, || ...)` opens the scope on this thread; `MemoryNeighbourOps` is a world of containers and tanks by position for tests (every face reaches everything).

```rust
use pluto_lib::{Direction, FluidStack, Identifier, MemoryNeighbourOps, Neighbours, with_neighbours};
let mut world = MemoryNeighbourOps::default();
world.tanks.insert([0, 0, 0], vec![(Some(FluidStack { fluid: Identifier::parse("minecraft:lava").unwrap(), amount: 100 }), 4000)]);
let drained = with_neighbours(&mut world, || Neighbours::drain_fluid([0, 0, 0], Direction::Up, 30));
assert_eq!(drained.unwrap().amount, 30);
```
