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

## Reading a Box of Blocks, and the World of a Ticking Block Entity

A machine that checks a structure around itself needs two things a command does not: the world of the block entity that is ticking, and a box of blocks read all at once. `World::ticking()` is the dimension the host is ticking a block entity in; `read_region` reads every block between two corners and answers `None` unless all of them are loaded, so a check never judges half a region:

```rust
use pluto_lib::{with_ticking_dimension, with_world, MemoryWorldOps, World};

let mut ops = MemoryWorldOps::default();
let walls_are_stone = with_ticking_dimension("minecraft:overworld", || {
    with_world(&mut ops, || {
        let world = World::ticking()?;
        let region = world.read_region([0, 64, 0], [2, 65, 2])?;
        Some(region.state([1, 64, 1]).is_some_and(|state| state.block.path() == "air"))
    })
});
assert_eq!(walls_are_stone, Some(true));
```

> **Use with Caution.** Host adapter written, not yet compiled or game-verified. `block_entity_tick` in `hooks.rs` opens `with_ticking_dimension` (the host's level in place) and the `with_world` scope around every ticker of `BlockEntityTypeRegistry`, so `World::ticking` and `read_region` work inside tickers. The ticking block entity is kept out of the world while it ticks: `set_block_state` of a block that is already there changes only the properties the state names (through the host's property write, so the block entity stays; properties the state leaves out keep their value), while a different block is placed whole and a block with a block entity gets a fresh one. A region read is one block read per position: keep boxes to what the check needs (the smeltery reads at most 16 by 16 by 64).

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

## Item-fragment Particle Factories

`ParticleFactoryRegistry` (Fabric's particle-factory registry) can register the standard item-fragment constructor under a particle ID. The factory keeps an owned `StackData`, so model selection can use the stack's item, damage, durability override and custom data without retaining an inventory reference. Re-registering replaces the declaration; invalid replacements leave the previous one intact.

```rust
use pluto_lib::{Identifier, ParticleFactoryRegistry, StackData};
let particle = Identifier::parse("example:clay_crumb").unwrap();
ParticleFactoryRegistry::register_item(particle.clone(), StackData {
    item: "minecraft:clay_ball".into(), count: 1, ..Default::default()
}).unwrap();
assert_eq!(ParticleFactoryRegistry::item(&particle).unwrap().item, "minecraft:clay_ball");
```

> **Use with Caution.** Registration and independent stack copies are pure-tested. A source-declared adapter handles named requests through the native client particle API: the requesting Game resolves the stack's current Ground model, then delegates once with ordinary Item options to preserve native validation and simulation. Its actual HEAD/bridge helpers pass trace-facade tests, not native stack conversion, model baking, particle settings/RNG or rendering. Cube landing emission now has a source-declared particle-kind override and a shared mob-event consumer used by both the host and remote-client tick. It changes only the kind, leaving the inherited loop/positions/RNG in place. The actual selector/event/constructor helper chain passes trace-facade tests, not the native loop or model/filter/RNG/render pipeline. Other engine-direct named requests, nonzero particle bursts, arbitrary entity emitters and network type registration remain incomplete or unverified. No custom particle physics or sprite-set factory is provided by this item-constructor API. Tinkers registers its original Sky/Ender slime-ball and Terracube clay-ball factories here.

## Reference

### `World::ticking`

`World::ticking() -> Option<World>` is the world of the block entity being ticked (Minecraft: `BlockEntity.getLevel`); `None` outside a ticker.

```rust
use pluto_lib::{with_ticking_dimension, World};
assert!(World::ticking().is_none());
with_ticking_dimension("minecraft:the_nether", || assert_eq!(World::ticking().unwrap().dimension, "minecraft:the_nether"));
```

### `with_ticking_dimension`

`with_ticking_dimension(dimension, || ...)` is the scope the host opens around a ticker, so that `World::ticking` answers.

```rust
use pluto_lib::{with_ticking_dimension, World};
with_ticking_dimension("minecraft:overworld", || assert!(World::ticking().is_some()));
```

### `World::read_region`

`read_region(from, to) -> Option<Region>` (Minecraft: `Level.hasChunksAt` then `getBlockState` over the box; corners in any order); `None` unless every block is loaded.

```rust
use pluto_lib::World;
assert!(World::new("minecraft:overworld").read_region([0, 0, 0], [1, 1, 1]).is_none(), "outside a world scope nothing is loaded");
```

### `Region`

The blocks `read_region` read: `state(pos)` and `bounds()`.

```rust
use pluto_lib::{with_world, MemoryWorldOps, World};
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || assert!(World::new("minecraft:overworld").read_region([0, 0, 0], [1, 1, 1]).is_some()));
```

### `Region::state`

`fn state(&self, pos: BlockPos) -> Option<&BlockState>`. The state read at `pos`, `None` outside the box.

```rust
use pluto_lib::{with_world, MemoryWorldOps, World};
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || {
    let region = World::new("minecraft:overworld").read_region([0, 0, 0], [1, 1, 1]).unwrap();
    assert!(region.state([0, 0, 0]).is_some() && region.state([5, 5, 5]).is_none());
});
```

### `Region::bounds`

`fn bounds(&self) -> (BlockPos, BlockPos)`. The corners given to `read_region`.

```rust
use pluto_lib::{with_world, MemoryWorldOps, World};
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || {
    let region = World::new("minecraft:overworld").read_region([0, 0, 0], [1, 1, 1]).unwrap();
    assert_eq!(region.bounds(), ([0, 0, 0], [1, 1, 1]));
});
```

### `ParticleFactoryRegistry`

General-purpose item-fragment factory registration, independent of any mod's particle names. It retains owned stacks rather than live inventory/model pointers; native rendering remains unvalidated.

```rust
use pluto_lib::ParticleFactoryRegistry;
let _ = ParticleFactoryRegistry::item;
```

### `ParticleFactoryRegistry::register_item`

`fn register_item(particle: Identifier, stack: StackData) -> Result<(), String>` registers or replaces the factory. Invalid item IDs, zero counts, counts outside positive Java int, nonpositive durability overrides and a poisoned registry return an error. Validation completes before replacement. Count, damage, durability and custom data are retained.

```rust
use pluto_lib::{Identifier, ParticleFactoryRegistry, StackData};
let particle = Identifier::parse("example:empty_crumb").unwrap();
assert!(ParticleFactoryRegistry::register_item(particle, StackData {
    item: "minecraft:clay_ball".into(), count: 0, ..Default::default()
}).is_err());
```

### `ParticleFactoryRegistry::register_entity_override`

`fn register_entity_override(entity: Identifier, original: Identifier, replacement: Identifier) -> Result<(), String>` replaces a specific inherited particle for one exact entity-type ID. The replacement must be an already registered item factory in a non-Minecraft namespace; invalid replacement leaves the old rule intact. Native plain particle names normalize to `minecraft:`. Unmatched types and particles keep their inherited names. Native source caller coverage currently includes cube landing events with count zero; other emitters remain unsupported or unverified.

```rust
use pluto_lib::{Identifier, ParticleFactoryRegistry, StackData};
let replacement = Identifier::parse("example:custom_cube_crumb").unwrap();
ParticleFactoryRegistry::register_item(replacement.clone(), StackData {
    item: "minecraft:clay_ball".into(), count: 1, ..Default::default()
}).unwrap();
ParticleFactoryRegistry::register_entity_override(
    Identifier::parse("example:custom_cube").unwrap(),
    Identifier::parse("minecraft:item_slime").unwrap(), replacement
).unwrap();
```

> **Use with Caution.** Native events require static particle names. This process-lifetime registry interns each declared factory ID once; replacing its stack reuses that name, including for in-flight events. It retains no Game, model or inventory references and has no unregister operation. Non-Minecraft replacement names prevent the later native enum consumer from emitting the same event twice. The requesting Game still supplies its current item model; native integration remains gated and unvalidated.

### `ParticleFactoryRegistry::item`

`fn item(particle: &Identifier) -> Option<StackData>` returns an independent stack copy or `None` for an absent factory or unavailable registry. Direct lookup does not emit a particle or establish native attachment.

```rust
use pluto_lib::{Identifier, ParticleFactoryRegistry, StackData};
let particle = Identifier::parse("example:independent_crumb").unwrap();
ParticleFactoryRegistry::register_item(particle.clone(), StackData {
    item: "minecraft:clay_ball".into(), count: 1, ..Default::default()
}).unwrap();
let mut copy = ParticleFactoryRegistry::item(&particle).unwrap();
copy.count = 7;
assert_eq!(ParticleFactoryRegistry::item(&particle).unwrap().count, 1);
```

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

What the game can do for a world: `block_state(dimension, pos) -> Option<String>` (the state as text, `None` if the chunk is not loaded), `set_block_state(dimension, pos, state) -> bool`, `play_sound(dimension, sound, at, volume, pitch) -> bool`, `random_float(dimension) -> Option<f32>` (default `None`), `block_in_tag(block, tag) -> bool` (default `false`), `summon_entity(dimension, kind, pos, nbt) -> Option<u32>`, and `explode(dimension, center, radius, fire, interaction) -> bool` (default `false`). PlutoLib implements it over Horizon; a test can implement it or use `MemoryWorldOps`.

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

An in-memory world for tests: every block set is remembered in `blocks` (by dimension and position), the saved data of block entities is read and merged in `entities`, every sound played in `sounds`, the `(block, tag)` pairs of `tags` that `block_in_tag` answers true for, every entity summoned in `summons`, every explosion asked for in `explosions`, every particle send asked for in `particles`, and every other position reads as air.

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

### `World::light`

`fn light(&self, pos: BlockPos) -> Option<(u8, u8)>`. Minecraft: `getBrightness` for the block and sky layers at `pos`: (block light, sky light), each 0..=15. `None` when the chunk is not loaded or the call is made outside a callback that carries a world.

```rust
use pluto_lib::World;

assert_eq!(World::new("minecraft:overworld").light([0, 64, 0]), None);
```

### `World::break_block`

`fn break_block(&self, pos: BlockPos, drop: bool) -> bool`. Minecraft: `Level.destroyBlock`. Breaks the block at `pos` through the game's own destroy path, with its drops when `drop` is set (the drops are the level's own, rolled without a tool). The request is queued and the level's tick seams apply it in request order, a tick late at most, so true means the break is accepted, not that it has happened when the call returns. False outside a callback that carries a world, or when the host cannot break blocks.

```rust
use pluto_lib::World;

assert!(!World::new("minecraft:overworld").break_block([0, 64, 0], true), "outside a callback nothing is broken");
```

### `World::use_block`

`fn use_block(&self, pos: BlockPos, face: BlockPos, hit: [f64; 3], item: Option<&str>) -> bool`. Minecraft: `BlockState.useItemOn` then `useWithoutItem`, re-dispatched: a use of the block at `pos` as a survival player holding the item `item` (None for an empty hand), clicked on the face `face` (a unit offset) at the absolute hit point `hit`. Queued and answered like `World::break_block`: true means the use will be dispatched at the level's next seam, not that it consumed the action. False outside a callback or when the host cannot re-dispatch.

```rust
use pluto_lib::World;

assert!(!World::new("minecraft:overworld").use_block([0, 64, 0], [0, 1, 0], [0.5, 65.0, 0.5], None));
```

### `World::set_block_state_queued`

`fn set_block_state_queued(&self, pos: BlockPos, state: &BlockState) -> bool`. Minecraft: `setBlockState`, at the level's next tick seam: the write joins the queued block actions (see `World::break_block`) and lands in request order among them, for a write that must follow a queued break of the same block. False outside a callback or when the host cannot queue one.

```rust
use pluto_lib::{BlockState, Identifier, World};

let stone = BlockState::new(Identifier::new("minecraft", "stone").unwrap());
assert!(!World::new("minecraft:overworld").set_block_state_queued([0, 64, 0], &stone), "outside a callback nothing is queued");
```

### `World::block_entity_data`

`fn block_entity_data(&self, pos: BlockPos) -> Option<NbtCompound>`. The saved data of the block entity at `pos` (Minecraft: `BlockEntity.saveWithoutMetadata`); `None` where there is none, the chunk is not loaded or the call is made outside a callback.

> **Use with Caution.** Host adapter written, not yet compiled or game-verified. `HostWorld` in `hooks.rs` answers it from the host's `mod_block_entity` (the data a mod block entity saves, without its position and id), in the dimension in place only: a call for another dimension answers `None`, and so does a block entity the host does not hold for a mod (a vanilla chest).

```rust
use pluto_lib::{with_world, MemoryWorldOps, Nbt, NbtCompound, World};

let mut ops = MemoryWorldOps::default();
let mut data = NbtCompound::new();
data.insert("master".to_owned(), Nbt::IntArray(vec![0, 64, 0]));
ops.entities.insert(("minecraft:overworld".to_owned(), [1, 64, 0]), data);
let world = World::new("minecraft:overworld");
assert!(world.block_entity_data([1, 64, 0]).is_none(), "outside a world scope");
with_world(&mut ops, || assert!(world.block_entity_data([1, 64, 0]).unwrap().contains_key("master")));
```

### `World::merge_block_entity_data`

`fn merge_block_entity_data(&self, pos: BlockPos, data: &NbtCompound) -> bool`. Merges `data` into the saved data of the block entity at `pos`: each key given replaces the same key, the others stay. A controller uses it to tell the blocks of its structure who their master is. False if there is no block entity there or the call is made outside a callback.

> **Use with Caution.** Host adapter written, not yet compiled or game-verified. `HostWorld` in `hooks.rs` merges through the host's `change_mod_block_entity` in the dimension in place and syncs the block entity to clients; another dimension, or a block entity the host does not hold for a mod, answers false.

```rust
use pluto_lib::{with_world, MemoryWorldOps, Nbt, NbtCompound, World};

let mut ops = MemoryWorldOps::default();
ops.entities.insert(("minecraft:overworld".to_owned(), [1, 64, 0]), NbtCompound::new());
let mut master = NbtCompound::new();
master.insert("master".to_owned(), Nbt::IntArray(vec![0, 64, 0]));
let world = World::new("minecraft:overworld");
assert!(!world.merge_block_entity_data([1, 64, 0], &master), "outside a world scope");
with_world(&mut ops, || assert!(world.merge_block_entity_data([1, 64, 0], &master)));
assert_eq!(ops.entities.len(), 1);
```

### `World::summon`

`fn summon(&self, kind: &str, pos: [f64; 3], nbt: &NbtCompound) -> Option<u32>`. Summons an entity of `kind` at `pos`, loaded from the saved data `nbt` (Minecraft: `/summon <kind> <pos> <nbt>`; the game writes the position itself, so `nbt` needs no `Pos`, and passengers in the data come along). Returns the new entity's id; `None` when the chunk is not loaded, the position is out of bounds, the kind is one the game cannot load, a callback kept the entity out, or the call is made outside a callback that carries a world. Projectile kinds load like any other; [projectile](projectile.md) builds its spawns on this.

> **Use with Caution.** Host adapter written, not yet compiled or game-verified. `HostWorld` in `hooks.rs` summons through the host's summon call (the game's own `/summon`, API 59) in the dimension named, and answers the id it returns.

```rust
use pluto_lib::{with_world, MemoryWorldOps, Nbt, NbtCompound, World};

let world = World::new("minecraft:overworld");
let mut nbt = NbtCompound::new();
nbt.insert("Item".into(), Nbt::String("minecraft:stone".into()));
assert_eq!(world.summon("minecraft:item", [0.5, 64.0, 0.5], &nbt), None, "outside a world scope");
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || {
    assert_eq!(world.summon("minecraft:item", [0.5, 64.0, 0.5], &nbt), Some(1));
});
assert_eq!(ops.summons[0].kind, "minecraft:item");
```

### `SummonRecord`

One entity a `MemoryWorldOps` was asked to summon, as it was asked: `dimension`, `kind`, `pos` and `nbt`. A summon's id is its 1-based place in `MemoryWorldOps::summons`.

```rust
use pluto_lib::{with_world, MemoryWorldOps, Nbt, NbtCompound, World};

let mut ops = MemoryWorldOps::default();
let mut nbt = NbtCompound::new();
nbt.insert("Item".into(), Nbt::String("minecraft:stone".into()));
with_world(&mut ops, || {
    assert_eq!(World::new("minecraft:overworld").summon("minecraft:item", [0.5, 64.0, 0.5], &nbt), Some(1));
});
let record = &ops.summons[0];
assert_eq!(record.dimension, "minecraft:overworld");
assert_eq!(record.kind, "minecraft:item");
assert_eq!(record.pos, [0.5, 64.0, 0.5]);
assert_eq!(record.nbt.get("Item"), Some(&Nbt::String("minecraft:stone".into())));
```

### `World::explode`

`fn explode(&self, center: [f64; 3], radius: f32, fire: bool, interaction: ExplosionInteraction) -> bool`. Sets off an explosion at `center` (Minecraft: `Level.explode`): entities in the radius are hurt and pushed back, `fire` starts fires on the blocks the blast exposed, and `interaction` says what happens to the blocks. `radius` is the explosion's size as the game's own calls state it (a primed tnt's is 4.0); the game accepts a finite centre within the world and a radius up to 32. Returns whether the explosion was set off (or, over Horizon, accepted for the level's seam to set off within the tick — the game has no mod call that explodes, so the request is carried to the level and goes off there); false for a centre or radius the game would refuse, or outside a callback that carries a world. The blast has no source entity: nothing is credited with it.

> **Use with Caution.** Host adapter written, not yet compiled or game-verified. `HostWorld` in `hooks.rs` validates the centre and radius as the game's `native_explode` (API 66) does and queues the request on `pluto_explosion_access`; the level's mob-event seam (`pluto_explosion_flush`) sets it off with `native_explode` in the dimension named, like the block ticks the seams drain.

```rust
use pluto_lib::{with_world, ExplosionInteraction, MemoryWorldOps, World};

let world = World::new("minecraft:overworld");
assert!(!world.explode([0.0, 64.0, 0.0], 4.0, false, ExplosionInteraction::Tnt), "outside a world scope");
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || {
    assert!(world.explode([8.5, 64.0, -2.5], 4.0, true, ExplosionInteraction::Tnt));
    assert!(!world.explode([8.5, 64.0, -2.5], 0.0, false, ExplosionInteraction::Keep), "no radius, no blast");
});
assert_eq!(ops.explosions.len(), 1);
assert_eq!(ops.explosions[0].interaction, ExplosionInteraction::Tnt);
```

### `ExplosionInteraction`

What an explosion does to the blocks it reaches (Minecraft: `ExplosionInteraction`): `Keep` leaves them (Java: `NONE`), `Block` breaks them by the block-explosion rule (`BLOCK`), and `Tnt` breaks them by the tnt-explosion rule (`TNT`; the original's `DESTROY` is the same rule).

```rust
use pluto_lib::ExplosionInteraction;

assert_eq!(ExplosionInteraction::Keep, ExplosionInteraction::Keep);
assert_ne!(ExplosionInteraction::Block, ExplosionInteraction::Tnt);
assert_ne!(ExplosionInteraction::Keep, ExplosionInteraction::Block);
```

### `ExplosionRecord`

One explosion a `MemoryWorldOps` was asked to set off, as it was asked: `dimension`, `center`, `radius`, `fire` and `interaction`.

```rust
use pluto_lib::{with_world, ExplosionInteraction, MemoryWorldOps, World};

let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || {
    assert!(World::new("minecraft:overworld").explode([8.5, 64.0, -2.5], 4.0, true, ExplosionInteraction::Tnt));
});
let record = &ops.explosions[0];
assert_eq!(record.dimension, "minecraft:overworld");
assert_eq!(record.center, [8.5, 64.0, -2.5]);
assert_eq!(record.radius, 4.0);
assert!(record.fire);
assert_eq!(record.interaction, ExplosionInteraction::Tnt);
```

### `SturdyLookup`

`type SturdyLookup = fn(&str, u8) -> Option<bool>`. A face-sturdiness lookup the host installs: whether the named block's face in a direction (Minecraft's Direction ordinal: 0 down, 1 up, 2 north, 3 south, 4 west, 5 east) is sturdy for the full support type (`BlockState.isFaceSturdy(.., SupportType.FULL)`), as the game's own block table reports it. The table answers by block name, so a block whose states differ answers with its first state.

```rust
use pluto_lib::world_api_v1::SturdyLookup;

let lookup: SturdyLookup = |block, _direction| if block == "minecraft:stone" { Some(true) } else { None };
assert_eq!(lookup("minecraft:stone", 1), Some(true));
assert_eq!(lookup("minecraft:glass", 1), None);
```

### `set_sturdy_lookup`

`fn set_sturdy_lookup(lookup: SturdyLookup)`. Installs the face-sturdiness lookup, once during startup. There is no uninstall.

```rust
use pluto_lib::world_api_v1::{face_sturdy, set_sturdy_lookup};

set_sturdy_lookup(|block, _direction| if block == "minecraft:stone" { Some(true) } else { None });
assert_eq!(face_sturdy("minecraft:stone", 1), Some(true));
```

### `face_sturdy`

`fn face_sturdy(block: &str, direction: u8) -> Option<bool>`. Whether the named block's face in `direction` (a Direction ordinal) is sturdy for the full support type, when the game's table answers for the block. `None` when no lookup is installed or the table has no answer for the block.

```rust
use pluto_lib::world_api_v1::{face_sturdy, set_sturdy_lookup};

assert_eq!(face_sturdy("minecraft:stone", 1), None, "no lookup installed yet");
set_sturdy_lookup(|block, _direction| if block == "minecraft:stone" { Some(true) } else { None });
assert_eq!(face_sturdy("minecraft:stone", 1), Some(true));
assert_eq!(face_sturdy("minecraft:glass", 1), None);
```

### `PlacementFacts`

`struct PlacementFacts { classes: Vec<String>, full_collision: bool, solid_render: bool }`. What the game's block table says about a block's shape and kind, for placement checks: the block's class chain (most specific first, in the game's own names, such as `TorchBlock`), whether its collision shape is a whole block, and whether it renders solid. Published from the game's table by the block facts head, by block name, so a block whose states differ answers with its first state.

```rust
use pluto_lib::world_api_v1::PlacementFacts;

let facts = PlacementFacts {
    classes: ["TorchBlock", "BaseTorchBlock", "Block"].iter().map(|c| c.to_string()).collect(),
    full_collision: false,
    solid_render: false,
};
assert_eq!(facts.classes[0], "TorchBlock");
```

### `set_placement_facts_lookup`

`fn set_placement_facts_lookup(lookup: PlacementFactsLookup)`. Installs the placement-facts lookup, once during startup. There is no uninstall.

```rust
use pluto_lib::world_api_v1::{PlacementFacts, set_placement_facts_lookup};

set_placement_facts_lookup(|block| if block == "minecraft:torch" {
    Some(PlacementFacts { classes: vec!["TorchBlock".to_string()], full_collision: false, solid_render: false })
} else { None });
```

### `placement_facts`

`fn placement_facts(block: &str) -> Option<PlacementFacts>`. The published placement facts for the named block. `None` when no lookup is installed or the table has no answer for the block.

```rust
use pluto_lib::world_api_v1::{PlacementFacts, placement_facts, set_placement_facts_lookup};

assert_eq!(placement_facts("minecraft:torch"), None, "no lookup installed yet");
set_placement_facts_lookup(|block| if block == "minecraft:torch" {
    Some(PlacementFacts { classes: vec!["TorchBlock".to_string()], full_collision: false, solid_render: false })
} else { None });
assert!(placement_facts("minecraft:torch").is_some());
assert_eq!(placement_facts("minecraft:glass"), None);
```

### `World::can_survive`

`fn can_survive(&self, state: &BlockState, pos: [i32; 3], sky_darken: u8) -> Option<bool>`. Whether `state` may stay at `pos`, as the game's `BlockState.canSurvive` answers it: the same class-chain dispatch the game runs, over the world view, so a torch needs a sturdy face below, a wall torch the face it leans on, and frosted ice survives anywhere. A mod-registered `BlockBehavior::can_survive` answers first, as the game's mod seam does. `None` where a needed fact is not published (the solid flag, the centre and rigid support bits, fire odds, the multiface attach table): a placement check must treat `None` as "cannot say", not as a refusal.

```rust
use pluto_lib::world_api_v1::{MemoryWorldOps, PlacementFacts, World, set_placement_facts_lookup, with_world};

set_placement_facts_lookup(|block| if block == "minecraft:frosted_ice" {
    Some(PlacementFacts { classes: vec!["FrostedIceBlock".to_string()], full_collision: true, solid_render: false })
} else { None });
let world = World::new("minecraft:overworld");
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || {
    assert_eq!(world.can_survive(&pluto_lib::BlockState::parse("minecraft:frosted_ice").unwrap(), [0, 64, 0], 0), Some(true));
});
```

### `World::is_unobstructed`

`fn is_unobstructed(&self, state: &BlockState, pos: [i32; 3]) -> Option<bool>`. Whether `state` may be placed at `pos` without standing in something's way, as the game's `Level.isUnobstructed` answers it: the replacement's collision shape against the entities around the cell. Only a whole-block shape is known, from the published placement facts, so any other state answers `None`. Entities are read through the entity scope, which the player tick opens; item entities never block, and an entity whose feet stand exactly on the cell's top face does not block it. `None` when the shape is not a whole block, no facts are published for the block, or no entity scope is open.

```rust
use pluto_lib::world_api_v1::{MemoryWorldOps, PlacementFacts, World, set_placement_facts_lookup, with_world};

set_placement_facts_lookup(|block| if block == "minecraft:frosted_ice" {
    Some(PlacementFacts { classes: vec!["FrostedIceBlock".to_string()], full_collision: true, solid_render: false })
} else { None });
let world = World::new("minecraft:overworld");
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || {
    // No entity scope is open here, so the answer is unknown.
    assert_eq!(world.is_unobstructed(&pluto_lib::BlockState::parse("minecraft:frosted_ice").unwrap(), [0, 64, 0]), None);
});
```

### `WorldView`

`struct WorldView<'a> { world: &'a World, sky_darken: u8 }`. A `BlockView` over a live `World`: block states, light and sturdy facts read through the world scope, and placement facts through the installed lookup. `brightness` darkens the sky light by `sky_darken`, as the game's getMaxLocalRawBrightness does; `raw_brightness` does not. Unreadable light is 0 for `brightness` and `None` for `raw_brightness`; `is_full_block` is the published `full_collision`, and `build_height` is the vanilla [-64, 320].

```rust
use pluto_lib::{BlockView, World};
use pluto_lib::world_api_v1::{MemoryWorldOps, WorldView, with_world};

let world = World::new("minecraft:overworld");
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || {
    let view = WorldView::new(&world, 0);
    assert!(view.is_air([0, 64, 0]));
});
```
