# World: reading and changing blocks

A `World` is a handle to one dimension. With it a callback can look at a block and put another one in its place: a projectile that lights up where it lands, a command that builds a platform, a machine that places a block. This page first walks through using it, then lists every item in the [Reference](#reference).

## Basic Example

Place a block when a player runs a command:

```rust
use pluto_lib::{BlockState, CommandRegistrationCallback, Identifier, World};

CommandRegistrationCallback::EVENT.register(|registrar| {
    registrar.register("example platform", |_ctx| {
        let world = World::new("minecraft:overworld");
        let glass = BlockState::new(Identifier::new("minecraft", "glass").unwrap());
        for x in 0..3 {
            world.set_block_state([x, 100, 0], &glass);
        }
        1
    });
});
```

## Block States and Their Properties

A `BlockState` is a block with its properties, written as `/setblock` writes it. A property you leave out has the block's default:

```rust
use pluto_lib::BlockState;

let stairs = BlockState::parse("minecraft:oak_stairs[facing=east,half=bottom]").unwrap();
assert_eq!(stairs.property("facing"), Some("east"));
assert_eq!(stairs.to_string(), "minecraft:oak_stairs[facing=east,half=bottom]");

let lit = BlockState::parse("minecraft:redstone_lamp").unwrap().with("lit", "true");
assert_eq!(lit.property("lit"), Some("true"));
```

## Reading Before Writing

`get_block_state` answers `None` when the chunk is not loaded, which is the case for any position far from the players. Check before you decide:

```rust
use pluto_lib::{with_world, BlockState, Identifier, MemoryWorldOps, World};

fn light_if_air(world: &World, pos: [i32; 3]) -> bool {
    match world.get_block_state(pos) {
        Some(state) if state.block.path() == "air" => {
            world.set_block_state(pos, &BlockState::new(Identifier::new("minecraft", "light").unwrap()))
        }
        _ => false,
    }
}

let mut ops = MemoryWorldOps::default();
let world = World::new("minecraft:overworld");
with_world(&mut ops, || {
    assert!(light_if_air(&world, [0, 64, 0]));
    assert!(!light_if_air(&world, [0, 64, 0]), "no longer air");
});
```

> **Use with Caution.** A `World` reaches the game only while a callback that carries a world runs (a command, a projectile impact, a player joining or leaving). Elsewhere its methods do nothing and say so (`None`, `false`). Whole-block writes of blocks with block entities create the block entity, but a block entity's data starts at its initial data.

## Drawing from the Server World's Random Stream

Use `World::random_float` for behavior that should consume the same stream as vanilla's level logic, such as two independently pitched block sounds. It draws from the existing authoritative dimension, never from a new generator or the clock:

```rust
use pluto_lib::World;

fn pitch(world: &World) -> Option<f32> {
    Some(0.5 + world.random_float()? * 1.2)
}
assert_eq!(pitch(&World::new("minecraft:overworld")), None); // no callback scope
```

> **Use with Caution.** A draw advances the level's native stream. Preserve the original order and short-circuit conditions; do not draw once and reuse it for two independent decisions. Outside a mutable world callback, on a client, or for an unopened dimension, the result is `None`. The native bridge compiled and supplied real crystal projectile sound callbacks in game. Exact native RNG traces, client/unopened-level runtime rejection and multiplayer remain unverified.

## Testing

Run your code inside `with_world` with a `MemoryWorldOps`, then look at `blocks`:

```rust
use pluto_lib::{with_world, BlockState, Identifier, MemoryWorldOps, World};

let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || {
    World::new("minecraft:the_nether").set_block_state([1, 2, 3], &BlockState::new(Identifier::new("minecraft", "glowstone").unwrap()));
});
assert_eq!(ops.blocks[&("minecraft:the_nether".to_owned(), [1, 2, 3])], "minecraft:glowstone");
```

## Reference

### `BlockState`

A block with its properties. `block` is the block (`minecraft:oak_stairs`), `properties` its properties by name.

```rust
use pluto_lib::{BlockState, Identifier};

let state = BlockState::new(Identifier::new("minecraft", "furnace").unwrap()).with("lit", "false");
assert_eq!(state.block.path(), "furnace");
assert_eq!(state.properties["lit"], "false");
```

### `BlockState::new`

`fn new(block: Identifier) -> BlockState`. The block with its default properties.

```rust
use pluto_lib::{BlockState, Identifier};

assert!(BlockState::new(Identifier::new("minecraft", "stone").unwrap()).properties.is_empty());
```

### `BlockState::with`

`fn with(self, key: &str, value: &str) -> BlockState`. The same state with one property set.

```rust
use pluto_lib::{BlockState, Identifier};

let door = BlockState::new(Identifier::new("minecraft", "oak_door").unwrap()).with("open", "true").with("half", "lower");
assert_eq!(door.properties.len(), 2);
```

### `BlockState::property`

`fn property(&self, key: &str) -> Option<&str>`. The value of a property, if the state names it.

```rust
use pluto_lib::BlockState;

let state = BlockState::parse("minecraft:wheat[age=3]").unwrap();
assert_eq!(state.property("age"), Some("3"));
assert_eq!(state.property("facing"), None);
```

### `BlockState::is_in_tag`

`fn is_in_tag(&self, tag: &str) -> bool`. Whether the block is in a block tag, such as `minecraft:mineable/pickaxe`. False for an unknown tag, and outside a `with_world` scope. Inside the break-speed callback the scope is open.

```rust
use pluto_lib::{with_world, BlockState, MemoryWorldOps};

let stone = BlockState::parse("minecraft:stone").unwrap();
let mut ops = MemoryWorldOps::default();
ops.tags.push(("minecraft:stone".into(), "minecraft:mineable/pickaxe".into()));
with_world(&mut ops, || {
    assert!(stone.is_in_tag("minecraft:mineable/pickaxe"));
    assert!(!stone.is_in_tag("minecraft:mineable/axe"));
});
assert!(!stone.is_in_tag("minecraft:mineable/pickaxe")); // no scope
```

### `BlockState::parse`

`fn parse(text: &str) -> Result<BlockState, IdentifierError>`. Reads `minecraft:oak_stairs[facing=east,half=bottom]`; the brackets are optional and a block without a namespace is `minecraft:`.

```rust
use pluto_lib::BlockState;

assert_eq!(BlockState::parse("stone").unwrap().to_string(), "minecraft:stone");
assert!(BlockState::parse("Not A Block").is_err());
```

### `WorldOps`

What the game can do for a world: `block_state(dimension, pos) -> Option<String>` (the state as text, `None` if the chunk is not loaded), `set_block_state(dimension, pos, state) -> bool`, `play_sound(dimension, sound, at, volume, pitch) -> bool`, `random_float(dimension) -> Option<f32>` (default `None`), and `block_in_tag(block, tag) -> bool` (default `false`). PlutoLib implements it over Horizon; a test can implement it or use `MemoryWorldOps`.

```rust
use pluto_lib::{with_world, World, WorldOps};

/// A world where nothing can be built.
struct Frozen;
impl WorldOps for Frozen {
    fn block_state(&mut self, _: &str, _: [i32; 3]) -> Option<String> { None }
    fn set_block_state(&mut self, _: &str, _: [i32; 3], _: &str) -> bool { false }
    fn play_sound(&mut self, _: &str, _: &str, _: [f64; 3], _: f32, _: f32) -> bool { false }
}
with_world(&mut Frozen, || assert_eq!(World::new("minecraft:overworld").get_block_state([0, 0, 0]), None));
```

#### Supplying a Level Random Stream

`fn random_float(&mut self, dimension: &str) -> Option<f32>`. A host supplies the next `[0, 1)` float from the requested authoritative level's existing stream. The default returns `None`; custom and memory worlds must opt in explicitly. Never silently create a generator when the dimension or authority is unavailable.

```rust
use pluto_lib::{MemoryWorldOps, WorldOps};
let mut memory = MemoryWorldOps::default();
assert_eq!(memory.random_float("minecraft:overworld"), None);
```

### `with_world`

`fn with_world<R>(ops: &mut dyn WorldOps, f: impl FnOnce() -> R) -> R`. Runs `f` with `ops` available to every `World` method called on this thread during `f`; the previous scope is restored afterwards, also when `f` panics.

```rust
use pluto_lib::{with_world, MemoryWorldOps, World};

let mut ops = MemoryWorldOps::default();
let seen = with_world(&mut ops, || World::new("minecraft:overworld").get_block_state([0, 64, 0]).is_some());
assert!(seen);
assert!(World::new("minecraft:overworld").get_block_state([0, 64, 0]).is_none(), "the scope is closed");
```

### `MemoryWorldOps`

An in-memory world for tests: every block set is remembered in `blocks` (by dimension and position), every sound played in `sounds`, the `(block, tag)` pairs of `tags` that `block_in_tag` answers true for, and every other position reads as air.

```rust
use pluto_lib::{with_world, BlockState, Identifier, MemoryWorldOps, World};

let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || {
    World::new("minecraft:overworld").set_block_state([0, 0, 0], &BlockState::new(Identifier::new("minecraft", "dirt").unwrap()));
});
assert_eq!(ops.blocks.len(), 1);
```

### `World`

A handle to one dimension. `dimension` is its id, such as `minecraft:overworld`.

```rust
use pluto_lib::World;

let world = World::new("minecraft:the_end");
assert_eq!(world.dimension, "minecraft:the_end");
```

### `World::new`

`fn new(dimension: &str) -> World`. A handle to the dimension `dimension`.

```rust
use pluto_lib::World;

assert_eq!(World::new("minecraft:overworld").dimension, "minecraft:overworld");
```

### `World::get_block_state`

`fn get_block_state(&self, pos: BlockPos) -> Option<BlockState>`. The block at `pos`; `None` if the chunk is not loaded or the call is made outside a callback that carries a world.

```rust
use pluto_lib::World;

assert_eq!(World::new("minecraft:overworld").get_block_state([0, 64, 0]), None);
```

### `World::play_sound`

`fn play_sound(&self, sound: &str, at: [f64; 3], volume: f32, pitch: f32) -> bool`. Plays the sound event `sound` (such as `minecraft:block.grass.place`) for the players near `at`. False if the sound is unknown, or the call is made outside a callback that carries a world.

```rust
use pluto_lib::{with_world, MemoryWorldOps, World};

let world = World::new("minecraft:overworld");
assert!(!world.play_sound("minecraft:block.grass.place", [0.5, 64.5, 0.5], 1.0, 1.0)); // outside a callback
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || assert!(world.play_sound("minecraft:block.grass.place", [0.5, 64.5, 0.5], 1.0, 1.0)));
assert_eq!(ops.sounds.len(), 1);
```

### `World::random_float`

`fn random_float(&self) -> Option<f32>`. Draw the next `[0, 1)` float from this dimension's server-level random stream. `None` outside a suitable callback, on a client, for an unopened dimension or when the host does not implement the operation. This is not a repeatable per-call seed: every successful draw advances the shared stream.

```rust
use pluto_lib::{with_world, World, WorldOps};

struct Tape(f32);
impl WorldOps for Tape {
    fn block_state(&mut self, _: &str, _: [i32; 3]) -> Option<String> { None }
    fn set_block_state(&mut self, _: &str, _: [i32; 3], _: &str) -> bool { false }
    fn play_sound(&mut self, _: &str, _: &str, _: [f64; 3], _: f32, _: f32) -> bool { false }
    fn random_float(&mut self, dimension: &str) -> Option<f32> {
        if dimension != "minecraft:the_end" { return None; }
        let value = self.0;
        self.0 += 0.25;
        Some(value)
    }
}
let mut tape = Tape(0.25);
let world = World::new("minecraft:the_end");
assert_eq!(world.random_float(), None);
with_world(&mut tape, || {
    assert_eq!(world.random_float(), Some(0.25));
    assert_eq!(world.random_float(), Some(0.5));
    assert_eq!(World::new("minecraft:overworld").random_float(), None);
});
assert_eq!(tape.0, 0.75);
```

### `World::set_block_state`

`fn set_block_state(&self, pos: BlockPos, state: &BlockState) -> bool`. Puts the state at `pos`; false if it could not be set (chunk not loaded, unknown block, outside a callback).

```rust
use pluto_lib::{BlockState, Identifier, World};

let stone = BlockState::new(Identifier::new("minecraft", "stone").unwrap());
assert!(!World::new("minecraft:overworld").set_block_state([0, 64, 0], &stone), "outside a callback nothing is set");
```
