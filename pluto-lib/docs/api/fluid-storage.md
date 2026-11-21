# Fluid storage of block entities

What a neighbour's fluid query (a faucet, a channel, a pipe, a bucket) is answered with, written over the hooks of a block entity type and a store that holds the saved data. A block with a `fluid_forward` hook answers with the storage of the block it points at, narrowed by its filter; a block with `fluid_reader`, `fluid_filler` and `fluid_drainer` hooks answers by them; any other block answers from its declared tanks. The hooks are on the [block entity page](block-entity.md#forwarding-and-custom-fluid-storage).

## Basic Example

A gate that forwards to a pool, letting only lava through while it holds a lava bucket:

```rust
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, FluidFilter, FluidForward, FluidStack, FluidStorage, Identifier, ItemSlot, MemoryBlockEntities, Nbt, NbtCompound};

let id = |text: &str| Identifier::parse(text).unwrap();
BlockEntityTypeRegistry::register(BlockEntityType::new(id("example:pool")).fluid_tanks([1000]));
BlockEntityTypeRegistry::register(BlockEntityType::new(id("example:gate")).item_slots(1).fluid_forward(|data, items| {
    let Some(Nbt::IntArray(at)) = data.get("to") else { return None };
    let filter = if items[0].is_some() { FluidFilter::Only(Identifier::parse("minecraft:lava").unwrap()) } else { FluidFilter::Any };
    Some(FluidForward { target: [at[0], at[1], at[2]], filter })
}));
let mut world = MemoryBlockEntities::default();
world.blocks.insert([0, 0, 0], "example:pool".to_owned());
world.entities.insert([0, 0, 0], (NbtCompound::new(), Vec::new()));
let mut gate = NbtCompound::new();
gate.insert("to".to_owned(), Nbt::IntArray(vec![0, 0, 0]));
world.blocks.insert([1, 0, 0], "example:gate".to_owned());
world.entities.insert([1, 0, 0], (gate, vec![Some(ItemSlot::new(id("minecraft:lava_bucket"), 1))]));

let mut storage = FluidStorage::new(&mut world);
let water = FluidStack { fluid: id("minecraft:water"), amount: 100, tag: None };
let lava = FluidStack { fluid: id("minecraft:lava"), amount: 100, tag: None };
assert_eq!(storage.fill([1, 0, 0], &water, true), 0, "the gate lets only lava through");
assert_eq!(storage.fill([1, 0, 0], &lava, true), 100);
assert_eq!(storage.drain([1, 0, 0], None, 30, true).unwrap().amount, 30);
```

> **Use with Caution.** The pure rules and the in-memory world are tested. The host adapter is written, not yet compiled or game-verified: `HostBlockEntities` in `hooks.rs` implements [`BlockEntityStore`](#blockentitystore) over the host's loaded block entities, and the neighbour adapter answers a neighbour's tank, fill and drain queries through `FluidStorage` for a block whose storage is its saved data read through its type's hooks (a reader, a drainer or a forward; see [neighbours](neighbours.md)). Blocks whose tanks the host holds itself keep the host's own calls. The generated `Host` signatures can only be checked by a native build. A forward is followed at most four times; a cycle answers nothing.

## Reference

### `BlockEntityStore`

What the storage reads and writes: `block_at(pos)` the block id, `read(pos)` the saved data and item slots of its block entity, `write(pos, data, items)` changed data back. The host implements it over its loaded chunks.

```rust
use pluto_lib::{BlockEntityStore, ItemSlot, MemoryBlockEntities, NbtCompound};

let mut world = MemoryBlockEntities::default();
world.write([0, 64, 0], NbtCompound::new(), Vec::<Option<ItemSlot>>::new());
assert_eq!(world.block_at([0, 64, 0]), None, "no block was put there");
assert!(world.read([0, 64, 0]).is_some());
```

### `MemoryBlockEntities`

A world of block entities for tests: `blocks` (block ids by position) and `entities` (saved data and item slots by position).

```rust
use pluto_lib::{BlockEntityStore, MemoryBlockEntities, NbtCompound};

let mut world = MemoryBlockEntities::default();
world.blocks.insert([0, 0, 0], "example:pool".to_owned());
world.entities.insert([0, 0, 0], (NbtCompound::new(), Vec::new()));
assert_eq!(world.block_at([0, 0, 0]).as_deref(), Some("example:pool"));
```

### `FluidStorage`

Fluid queries over a store: [`tanks`](#fluidstoragetanks), [`fill`](#fluidstoragefill) and [`drain`](#fluidstoragedrain) at a position, following forwards.

```rust
use pluto_lib::{FluidStorage, MemoryBlockEntities};

let mut world = MemoryBlockEntities::default();
assert!(FluidStorage::new(&mut world).tanks([0, 0, 0]).is_none(), "nothing there");
```

### `FluidStorage::new`

`fn new(store: &mut dyn BlockEntityStore) -> FluidStorage`. Queries over `store`.

```rust
use pluto_lib::{FluidStorage, MemoryBlockEntities};

let mut world = MemoryBlockEntities::default();
let _storage = FluidStorage::new(&mut world);
```

### `FluidStorage::tanks`

`fn tanks(&mut self, pos: BlockPos) -> Option<Vec<NeighbourTank>>`. The tanks the block at `pos` shows, as its filter lists them: all for `Any`, those empty or holding the fluid for `Only`, the last one while it is empty for `Closed`. `None` where there is no storage.

```rust
use pluto_lib::{FluidStorage, MemoryBlockEntities};

let mut world = MemoryBlockEntities::default();
assert_eq!(FluidStorage::new(&mut world).tanks([5, 5, 5]), None);
```

### `FluidStorage::fill`

`fn fill(&mut self, pos: BlockPos, stack: &FluidStack, execute: bool) -> u32`. How much of `stack` the block at `pos` takes, through its `fluid_filler` or into its declared tanks; a filter refuses other fluids and `Closed` takes none. With `execute` false it only asks.

```rust
use pluto_lib::{FluidStack, FluidStorage, Identifier, MemoryBlockEntities};

let mut world = MemoryBlockEntities::default();
let water = FluidStack { fluid: Identifier::parse("minecraft:water").unwrap(), amount: 10, tag: None };
assert_eq!(FluidStorage::new(&mut world).fill([0, 0, 0], &water, true), 0, "nothing there takes it");
```

### `FluidStorage::drain`

`fn drain(&mut self, pos: BlockPos, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack>`. Takes up to `amount` millibuckets out of the block at `pos`: of `wanted` when named, otherwise of what the storage gives first; a filter that names a fluid gives that fluid only, and `Closed` none.

```rust
use pluto_lib::{FluidStorage, MemoryBlockEntities};

let mut world = MemoryBlockEntities::default();
assert_eq!(FluidStorage::new(&mut world).drain([0, 0, 0], None, 50, true), None);
```
