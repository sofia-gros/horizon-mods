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

What the game can do for a world: `block_state(dimension, pos) -> Option<String>` (the state as text, `None` if the chunk is not loaded), `set_block_state(dimension, pos, state) -> bool`, `play_sound(dimension, sound, at, volume, pitch) -> bool`, `random_float(dimension) -> Option<f32>` (default `None`), `build_height(dimension) -> Option<[i32; 2]>` (the build range of the dimension's level, default `None`), `block_in_tag(block, tag) -> bool` (default `false`), `summon_entity(dimension, kind, pos, nbt) -> Option<u32>`, and `explode(dimension, center, radius, fire, interaction) -> bool` (default `false`). PlutoLib implements it over Horizon; a test can implement it or use `MemoryWorldOps`.

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

An in-memory world for tests: every block set is remembered in `blocks` (by dimension and position), the saved data of block entities is read and merged in `entities`, every sound played in `sounds`, the `(block, tag)` pairs of `tags` that `block_in_tag` answers true for, the build ranges published per dimension in `build_heights`, every entity summoned in `summons`, every explosion asked for in `explosions`, every brush stroke asked for in `brushes`, every generic brush sound asked for in `brush_fallbacks`, every particle send asked for in `particles`, and every other position reads as air.

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

### `World::build_height`

`fn build_height(&self) -> Option<[i32; 2]>`. Minecraft: `Level.getMinBuildHeight` and `Level.getMaxBuildHeight`: the build range of this dimension's level, [lowest buildable y, first y above the build limit]. The host answers inside a callback that carries a world; outside one, the range the level last published on its tick answers (a level publishes every tick it runs), so a projectile's flight telling reads its own level's range too. `None` when neither has answered for the dimension — an unloaded dimension, a client, or a level that never ticked. A custom dimension reads its own range, not the overworld's.

```rust
use pluto_lib::{with_world, World, WorldOps};

struct Levels;
impl WorldOps for Levels {
    fn block_state(&mut self, _: &str, _: [i32; 3]) -> Option<String> { None }
    fn set_block_state(&mut self, _: &str, _: [i32; 3], _: &str) -> bool { false }
    fn play_sound(&mut self, _: &str, _: &str, _: [f64; 3], _: f32, _: f32) -> bool { false }
    fn build_height(&mut self, dimension: &str) -> Option<[i32; 2]> {
        match dimension {
            "minecraft:overworld" => Some([-64, 320]),
            "example:custom" => Some([-32, 128]),
            _ => None,
        }
    }
}
let mut levels = Levels;
assert_eq!(World::new("example:custom").build_height(), None);
with_world(&mut levels, || {
    assert_eq!(World::new("example:custom").build_height(), Some([-32, 128]));
    assert_eq!(World::new("minecraft:overworld").build_height(), Some([-64, 320]));
    assert_eq!(World::new("minecraft:the_end").build_height(), None);
});
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

### `World::brush_block`

`fn brush_block(&self, pos: BlockPos, face: BlockPos) -> bool`. One brush stroke on the block at `pos`, on the face `face` (a unit offset): Minecraft's `BrushableBlockEntity.brush`. A brushable block entity (suspicious sand or gravel) advances its archaeology count, its `dusted` state follows, and a completed block drops its item beside the brushed face and turns back into plain sand or gravel; the block entity remembers the first face brushed as the face its item shows from. Returns whether the stroke was accepted (over Horizon, queued for the game's brush seam to apply within the tick — the game has no mod call that brushes, so the request is carried into the game, as an explosion is); it does not say whether the block is brushable, which `World::get_block_state` answers. False outside a callback that carries a world.

> **Use with Caution.** Host adapter written, not yet compiled or game-verified. `HostWorld` in `hooks.rs` queues the request on `pluto_brush_access`; the game's creature tick seam (`pluto_brush_flush`, in the game's own brush module) plays the brushable block's brush sound, spawns its dust, and calls the game's own `BrushableBlockEntity.brush` port in the dimension named.

```rust
use pluto_lib::{with_world, MemoryWorldOps, World};

let world = World::new("minecraft:overworld");
assert!(!world.brush_block([0, 64, 0], [0, 1, 0]), "outside a world scope");
let mut ops = MemoryWorldOps::default();
ops.blocks.insert(("minecraft:overworld".to_owned(), [0, 64, 0]), "minecraft:suspicious_sand".to_owned());
with_world(&mut ops, || {
    assert!(world.brush_block([0, 64, 0], [0, 1, 0]));
    assert!(!world.brush_block([1, 64, 0], [0, 1, 0]), "plain air is not brushable");
});
assert_eq!(ops.brushes.len(), 2);
assert_eq!(ops.brushes[0].pos, [0, 64, 0]);
```

### `BrushRecord`

One brush stroke a `MemoryWorldOps` was asked to apply, as it was asked: `dimension`, `pos` and `face`.

```rust
use pluto_lib::{with_world, MemoryWorldOps, World};

let world = World::new("minecraft:overworld");
let mut ops = MemoryWorldOps::default();
ops.blocks.insert(("minecraft:overworld".to_owned(), [0, 64, 0]), "minecraft:suspicious_sand".to_owned());
with_world(&mut ops, || {
    assert!(world.brush_block([0, 64, 0], [0, 1, 0]));
});
let record = &ops.brushes[0];
assert_eq!(record.dimension, "minecraft:overworld");
assert_eq!(record.pos, [0, 64, 0]);
assert_eq!(record.face, [0, 1, 0]);
```

### `World::brush_fallback_sound`

`fn brush_fallback_sound(&self, pos: BlockPos) -> bool`. The generic brush sound (`minecraft:item.brush.brushing.generic`) for a stroke that brushed nothing at `pos`: `BrushModule.onUsingTick` plays `SoundEvents.BRUSH_GENERIC` when a stroke's damage is 0, on the client only. Returns whether the sound was accepted (over Horizon, queued for the game's brush seam to play within the tick, for the brushing player alone — unshared when they are the local player, sent to them alone when they are a guest). False outside a callback that carries a world.

> **Use with Caution.** Host adapter written, not yet compiled or game-verified. `HostWorld` in `hooks.rs` queues the request on `pluto_brush_access` (its fallback queue); the game's creature tick seam (`pluto_brush_flush`, in the game's own brush module) drains it after the strokes and plays the generic sound for the acting player alone.

```rust
use pluto_lib::{with_world, MemoryWorldOps, World};

let world = World::new("minecraft:overworld");
assert!(!world.brush_fallback_sound([0, 64, 0]), "outside a world scope");
let mut ops = MemoryWorldOps::default();
with_world(&mut ops, || {
    assert!(world.brush_fallback_sound([0, 64, 0]));
});
assert_eq!(ops.brush_fallbacks.len(), 1);
assert_eq!(ops.brush_fallbacks[0].pos, [0, 64, 0]);
```

### `BrushFallbackRecord`

One generic brush sound a `MemoryWorldOps` was asked to play, as it was asked: `dimension` and `pos`.



```rust
use pluto_lib::BrushFallbackRecord;

let record = BrushFallbackRecord { dimension: "minecraft:overworld".into(), pos: [0, 64, 0] };
assert_eq!(record.pos, [0, 64, 0]);
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

## Mob splits

### `MobSplitContext`

The dying parent and the split the game is about to perform: `parent` is the parent entity, `position` its position, `size` its size and `child_size` the children's size (half of it).

```rust
use pluto_lib::{EntityInfo, MobSplitContext};
let context = MobSplitContext { parent: EntityInfo { id: 7, kind: "example:parent".into(), dimension: "minecraft:overworld".into() },
    position: [1.0, 64.0, 2.0], size: 4, child_size: 2 };
assert_eq!(context.child_size, 2);
assert_eq!(context.parent.id, 7);
```

### `MobSplitRandom`

Random draws for inheritance decisions, from the parent's own stream.

```rust
use pluto_lib::MobSplitRandom;
struct Fixed;
impl MobSplitRandom for Fixed {
    fn next_float(&mut self) -> f32 { 0.25 }
    fn next_int(&mut self, bound: i32) -> Result<i32, String> { Ok(bound - 1) }
}
let mut random = Fixed;
assert_eq!(random.next_float(), 0.25);
assert_eq!(random.next_int(3), Ok(2));
```

### `MobSplitChild`

One spawned child, dressed before it joins the world; its `EntityInfo` carries the child's real id, so attachment writes land on the child.

```rust
use pluto_lib::{EntityInfo, MobSplitChild, StackData};
struct Child { info: EntityInfo }
impl MobSplitChild for Child {
    fn info(&self) -> EntityInfo { self.info.clone() }
    fn set_equipment(&mut self, _: &str, _: StackData) -> Result<(), String> { Ok(()) }
    fn set_custom_name(&mut self, _: Option<String>) -> Result<(), String> { Ok(()) }
    fn set_no_ai(&mut self, _: bool) -> Result<(), String> { Ok(()) }
    fn set_invulnerable(&mut self, _: bool) -> Result<(), String> { Ok(()) }
}
let child = Child { info: EntityInfo { id: 8, kind: "example:parent".into(), dimension: "minecraft:overworld".into() } };
assert_eq!(child.info().id, 8);
```

### `MobSplitInheritance`

The inheritance view handed to split callbacks after the game spawned the children: `context` is the split, `head` is the parent's head stack (children receive copies), `head_drop_chance` is the parent's recorded head drop chance, `custom_name`, `no_ai` and `invulnerable` are the parent's flags, `random` draws from the parent's stream, and `children` are the spawned children in the game's spawn order.

```rust
use pluto_lib::{EntityInfo, MobSplitChild, MobSplitContext, MobSplitInheritance, MobSplitRandom, StackData};
struct Fixed;
impl MobSplitRandom for Fixed {
    fn next_float(&mut self) -> f32 { 0.5 }
    fn next_int(&mut self, bound: i32) -> Result<i32, String> { Ok(bound - 1) }
}
struct Child { info: EntityInfo }
impl MobSplitChild for Child {
    fn info(&self) -> EntityInfo { self.info.clone() }
    fn set_equipment(&mut self, _: &str, _: StackData) -> Result<(), String> { Ok(()) }
    fn set_custom_name(&mut self, _: Option<String>) -> Result<(), String> { Ok(()) }
    fn set_no_ai(&mut self, _: bool) -> Result<(), String> { Ok(()) }
    fn set_invulnerable(&mut self, _: bool) -> Result<(), String> { Ok(()) }
}
let context = MobSplitContext { parent: EntityInfo { id: 7, kind: "example:parent".into(), dimension: "minecraft:overworld".into() },
    position: [1.0, 64.0, 2.0], size: 4, child_size: 2 };
let mut random = Fixed;
let mut children: Vec<Box<dyn MobSplitChild>> = vec![Box::new(Child { info: EntityInfo { id: 8, kind: "example:parent".into(), dimension: "minecraft:overworld".into() } })];
let view = MobSplitInheritance { context: &context, head: None, head_drop_chance: 0.085,
    custom_name: None, no_ai: false, invulnerable: false, random: &mut random, children: &mut children };
assert_eq!(view.children.len(), 1);
assert_eq!(view.head_drop_chance, 0.085);
```

### `MobSplitCallback`

A split observer with two phases, both defaulting to keeping the game's split: `prepare` tunes the child count or cancels the split; `inherit` dresses the spawned children.

```rust
use pluto_lib::MobSplitCallback;
struct Observer;
impl MobSplitCallback for Observer {}
let _observer = Observer;
```

### `MobSplitCallbacks`

The registration-order split dispatch: preparation callbacks run in order before the game spawns, inheritance callbacks in order after.

```rust
use pluto_lib::MobSplitCallbacks;
let _registry = MobSplitCallbacks;
```

### `MobSplitCallbacks::register`

Registers a split callback under a unique name; duplicate names reject.

```rust
use pluto_lib::{MobSplitCallback, MobSplitCallbacks};
struct Observer;
impl MobSplitCallback for Observer {}
MobSplitCallbacks::register("example:observer", Observer).unwrap();
assert!(MobSplitCallbacks::has_callbacks());
```

### `MobSplitCallbacks::has_callbacks`

Whether any split callback is registered; the native split hooks stay inert while none is.

```rust
use pluto_lib::MobSplitCallbacks;
let active: bool = MobSplitCallbacks::has_callbacks();
let _ = active;
```

## Spawn placements

### `SpawnPlacementContext`

The facts of one candidate spawn position: `entity_type` and `dimension` name the spawn, `position` is the block position of the mob's feet, `reason` is the backend reason, `peaceful` reports a peaceful level, `block_below` is the block below the candidate, `fluid_at` and `fluid_below` are the fluids at and below it (`minecraft:empty` when none), `block_below_valid_spawn` says the block below is a valid spawn block for the type's base, and `dark_enough` says a monster would be dark enough to spawn there.

```rust
use pluto_lib::{Identifier, SpawnPlacementContext};
let context = SpawnPlacementContext { entity_type: Identifier::parse("example:slime").unwrap(),
    dimension: Identifier::parse("minecraft:overworld").unwrap(), position: [0, 64, 0], reason: "natural".into(),
    peaceful: false, block_below: Identifier::parse("example:grass").unwrap(),
    fluid_at: Identifier::parse("minecraft:empty").unwrap(), fluid_below: Identifier::parse("minecraft:empty").unwrap(),
    block_below_valid_spawn: true, dark_enough: true };
assert_eq!(context.position[1], 64);
```

### `SpawnPlacementRegistry`

Per-type spawn placement registration: one predicate per type, whose verdict replaces the inherited species rules for that type where the game consults placements.

```rust
use pluto_lib::SpawnPlacementRegistry;
let _registry = SpawnPlacementRegistry;
```

### `SpawnPlacementRegistry::register`

Registers a type's placement predicate; a second registration for the same type rejects.

```rust
use pluto_lib::{Identifier, SpawnPlacementRegistry};
SpawnPlacementRegistry::register(Identifier::parse("example:slime").unwrap(), |context| {
    !context.peaceful && context.block_below == Identifier::parse("example:grass").unwrap()
}).unwrap();
assert!(SpawnPlacementRegistry::is_registered(&Identifier::parse("example:slime").unwrap()));
```

### `SpawnPlacementRegistry::is_registered`

Whether a placement predicate is registered for the type.

```rust
use pluto_lib::{Identifier, SpawnPlacementRegistry};
let registered: bool = SpawnPlacementRegistry::is_registered(&Identifier::parse("example:absent").unwrap());
let _ = registered;
```

### `SpawnPlacementRegistry::register_or`

Registers a type's OR placement predicate (the original's `Operation.OR`): positions the inherited species rules rejected may still spawn when the predicate accepts them. The game's own selection never proposes those positions, so a permit-add pass after the game's natural pass proposes its own candidates around players and spawns the accepted ones through the game's own preparation and finalization. One OR predicate per type, independent of any replacing predicate; a second OR registration for the same type rejects.

```rust
use pluto_lib::{Identifier, SpawnPlacementRegistry};
SpawnPlacementRegistry::register_or(Identifier::parse("example:or_slime").unwrap(), |context| {
    !context.peaceful && context.block_below == Identifier::parse("example:grass").unwrap()
}).unwrap();
assert!(SpawnPlacementRegistry::is_or_registered(&Identifier::parse("example:or_slime").unwrap()));
```

### `SpawnPlacementRegistry::is_or_registered`

Whether an OR placement predicate is registered for the type.

```rust
use pluto_lib::{Identifier, SpawnPlacementRegistry};
let registered: bool = SpawnPlacementRegistry::is_or_registered(&Identifier::parse("example:absent").unwrap());
let _ = registered;
```

### `FishingRegistry`

The registry of fishing rods a mod casts with: which items count as rods (a hook stays alive only while its owner holds one — the game's own check names the vanilla rod, and the transport asks this registry for the rest), and how a rod stack's cast profile is computed. The game's fishing is built around its own hook, whose rod use is gated to the vanilla item; a registered rod's use goes through [`Fishing::use_rod`](#fishing) instead.

```rust
use pluto_lib::{FishingProfile, FishingRegistry, Grapple, Identifier, StackData};

FishingRegistry::register(Identifier::parse("example:war_rod").unwrap(), |stack| {
    (stack.count > 0).then_some(FishingProfile { power: 2.0, grapple: Grapple::Drill, ..FishingProfile::default() })
});
assert!(FishingRegistry::is_rod("example:war_rod"));
let stack = StackData { item: "example:war_rod".into(), count: 1, ..StackData::default() };
assert_eq!(FishingRegistry::profile(&stack).map(|profile| profile.power), Some(2.0));
```

### `FishingProfile`

The numbers a rod casts with, computed from the stack used: the `luck` and `lure` the hook fishes with, the `velocity` it flies at, the cast's `inaccuracy` spread, the `power` a hooked entity is hurt by (0 fishes without fighting), the `knockback` bonus on the retrieve pull, whether the hook `grapple`s and collects what it touches.

```rust
use pluto_lib::{FishingProfile, Grapple};

let plain = FishingProfile::default();
assert_eq!((plain.velocity, plain.power, plain.grapple), (1.0, 0.0, Grapple::None));
let war = FishingProfile { luck: 2, lure: 1, power: 2.0, grapple: Grapple::Dash, collecting: true, ..plain };
assert_eq!((war.luck, war.lure, war.collecting), (2, 1, true));
```

### `Grapple`

How a grapple hook behaves: `None` only fishes and fights, `Dash` pulls its owner toward the hook when it lands, and `Drill` also pulls when it hooks an entity.

```rust
use pluto_lib::Grapple;

assert_eq!(Grapple::default(), Grapple::None);
assert_ne!(Grapple::Dash, Grapple::Drill);
```

### `Fishing`

Fishing with a mod's rod: the calls a mod makes. `use_rod` queues a use — the game's fishing tick casts with the profile when no hook of this rod is out and retrieves when one is — and `take_wear` drains the rod wear the game side reports (a cast's and a hit's 1, a retrieve's cost), which the mod pays into its own tool at its next use, so the wear lands at most one use late. The reach is the local player's, as the game's fishing simulation is single-hook and local.

```rust
use pluto_lib::{Fishing, FishingProfile, ServerPlayer};

let player = ServerPlayer { uuid: 7, name: "Alex".into() };
Fishing::use_rod(&player, FishingProfile::default());
assert_eq!(Fishing::take_use(), Some((7, FishingProfile::default())));
Fishing::note_wear(7, 3);
assert_eq!(Fishing::take_wear(&player), 3);
assert_eq!(Fishing::take_wear(&player), 0);
```

### `combat_damage`

The damage a hooked entity takes at retrieve: the impact speed times the power times 15, rounded up to a whole tenth.

```rust
use pluto_lib::combat_damage;

assert_eq!(combat_damage(2.0, 2.0), 6.0);
assert_eq!(combat_damage(0.5, 1.0), 0.8);
assert_eq!(combat_damage(3.0, 0.0), 0.0);
```

### `hook_damage_type`

The damage type a combat hook's retrieve hurt is dealt as: the ranged `tconstruct:fishing_hook`, or the melee `tconstruct:melee_fishing_hook` when the hooked target is an enderman carrying the enderference effect (the enderference override), so the hurt lands on a mob that dodges projectiles.

```rust
use pluto_lib::hook_damage_type;

assert_eq!(hook_damage_type(true), "tconstruct:melee_fishing_hook");
assert_eq!(hook_damage_type(false), "tconstruct:fishing_hook");
```

### `pull_delta`

The velocity a hooked entity gains at retrieve: toward the owner, a tenth of the way, plus the knockback bonus `0.25 * knockback / distance` when the rod has knockback.

```rust
use pluto_lib::pull_delta;

assert_eq!(pull_delta([0.0, 64.0, 0.0], [10.0, 64.0, 0.0], 0.0), [-1.0, 0.0, 0.0]);
assert_eq!(pull_delta([0.0, 64.0, 0.0], [10.0, 64.0, 0.0], 4.0), [-2.0, 0.0, 0.0]);
```

### `grapple_delta`

The velocity the owner gains from a grapple: toward the hook, scaled by `0.58 / sqrt(distance)`.

```rust
use pluto_lib::grapple_delta;

let pull = grapple_delta([0.0, 64.0, 0.0], [16.0, 64.0, 0.0]);
assert!((pull[0] - 2.32).abs() < 1e-9);
assert_eq!(grapple_delta([0.0, 64.0, 0.0], [0.0, 64.0, 0.0]), [0.0; 3]);
```

### `retrieve_damage`

The rod wear a retrieve costs, over the game's own retrieve number: a grounded hook costs at least 2 and at most 3; anything else costs at most 3.

```rust
use pluto_lib::retrieve_damage;

assert_eq!(retrieve_damage(5, true), 3);
assert_eq!(retrieve_damage(0, true), 2);
assert_eq!(retrieve_damage(5, false), 3);
assert_eq!(retrieve_damage(1, false), 1);
```

### `DispenseRegistry::register`

Registers the behaviour for an item, replacing any earlier one for it.

```rust
use pluto_lib::{DispenseBehavior, DispenseRegistry, Identifier};

DispenseRegistry::register(
    Identifier::parse("example:ball").unwrap(),
    DispenseBehavior::Shoot { entity: Identifier::parse("example:ball").unwrap(), power: 1.1, inaccuracy: 6.0 },
);
assert!(DispenseRegistry::get("example:ball").is_some());
```

### `DispenseRegistry::get`

The behaviour registered for the item named, if any.

```rust
use pluto_lib::DispenseRegistry;

assert!(DispenseRegistry::get("example:never_registered").is_none());
```

### `FishingRegistry::register`

Registers a rod item and its cast profile computation: when the item is used, the computation reads the stack and answers the profile to cast with, or None to leave the use to the game.

```rust
use pluto_lib::{FishingProfile, FishingRegistry, Identifier};

FishingRegistry::register(Identifier::parse("example:rod").unwrap(), |_| Some(FishingProfile::default()));
assert!(FishingRegistry::is_rod("example:rod"));
```

### `FishingRegistry::register_rod`

Names an item a fishing rod for the hook's lifetime check, without a cast computation of its own.

```rust
use pluto_lib::{FishingRegistry, Identifier};

FishingRegistry::register_rod(Identifier::parse("example:plain_rod").unwrap());
assert!(FishingRegistry::is_rod("example:plain_rod"));
```

### `FishingRegistry::is_rod`

Whether the item named is a registered rod.

```rust
use pluto_lib::FishingRegistry;

assert!(!FishingRegistry::is_rod("example:not_a_rod"));
```

### `FishingRegistry::profile`

The profile a stack casts with, when its item registered a computation and the computation answers one.

```rust
use pluto_lib::{FishingRegistry, StackData};

assert_eq!(FishingRegistry::profile(&StackData::default()), None);
```

### `Fishing::use_rod`

Queues a use of a fishing rod by the player with the profile: the game's fishing tick casts with it when no hook of this rod is out, and retrieves when one is.

```rust
use pluto_lib::{Fishing, FishingProfile, ServerPlayer};

let player = ServerPlayer { uuid: 3, name: "Sam".into() };
Fishing::use_rod(&player, FishingProfile::default());
assert!(Fishing::take_use().is_some());
```

### `Fishing::take_use`

Takes the queued use, if any; the transport calls this.

```rust
use pluto_lib::Fishing;

assert_eq!(Fishing::take_use(), None);
```

### `Fishing::note_wear`

Records rod wear the game side reports; the transport calls this.

```rust
use pluto_lib::{Fishing, ServerPlayer};

Fishing::note_wear(5, 2);
let player = ServerPlayer { uuid: 5, name: "Sam".into() };
assert_eq!(Fishing::take_wear(&player), 2);
```

### `Fishing::take_wear`

The rod wear waiting for the player, drained: the total the game side has reported since the last call.

```rust
use pluto_lib::{Fishing, ServerPlayer};

let player = ServerPlayer { uuid: 6, name: "Sam".into() };
assert_eq!(Fishing::take_wear(&player), 0);
```

### `PlacementFactsLookup`

`pub type PlacementFactsLookup = fn(&str) -> Option<PlacementFacts>`. A placement-facts lookup the host installs: the facts for a named block, if the game's table names it.

```rust
use pluto_lib::{PlacementFacts, PlacementFactsLookup};

fn no_facts(_block: &str) -> Option<PlacementFacts> { None }
let lookup: PlacementFactsLookup = no_facts;
assert!(lookup("minecraft:stone").is_none());
```

### `WorldView::new`

`fn new(world: &World, sky_darken: u8) -> WorldView`. A view over `world`, reading light against `sky_darken` (the host's `Level.getSkyDarken`): the view's brightness is the darkened one, while its `canSurvive` reads the undarkened one, as the host's own placement checks do.

```rust
use pluto_lib::{World, WorldView};

let world = World::new("minecraft:overworld");
let _view = WorldView::new(&world, 0);
```
