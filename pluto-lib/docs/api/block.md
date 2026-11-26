# Block Behaviour

Say what a block of your mod does that its content file cannot say: where it may stand, which state it takes when placed, how a neighbour changes it, and what happens to an entity inside it. The block itself (its template, its state properties, its shapes, its loot) is declared in the mod's content files; a `BlockBehavior` is attached to it by id. This page first walks through using it, then lists every item in the [Reference](#reference).

> **Current clean-port status (2026-11-14).** PlutoLib now attaches player inside-block callbacks and mob restitution directly to pinned Horizon source, retaining the native sweep and movement paths. Full attachment planning/application has passed for the earlier input; native typechecking is being repaired as host drift is found. Native/game results elsewhere on this page are historical, not verification of these changed inputs. See [current build status](../../../docs/development.md#plutolib-owned-direct-horizon-attachment).

## Basic Example

A hook that hangs from the block above it, and breaks when that block goes:

```rust
use pluto_lib::{BlockBehavior, BlockBehaviors, Direction, Identifier};

BlockBehaviors::register(
    BlockBehavior::new(Identifier::new("example", "hook").unwrap())
        .can_survive(|c| c.view.is_face_sturdy(Direction::Up.relative(c.pos), Direction::Down)),
);
```

`c.view` looks at the blocks around; `Direction::Up.relative(c.pos)` is the block above. The game asks again whenever a neighbour changes, so the hook is removed (and drops its item) as soon as the block above stops being solid.

## Choosing the State When Placed

`state_for_placement` answers with a `Placement`: leave the game's choice alone, refuse, or give a state. This one faces away from the clicked side, and refuses water:

```rust
use pluto_lib::{BlockBehavior, Identifier, Placement};

let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap())
    .state_for_placement(|c| {
        if c.in_water {
            return Placement::Refuse;
        }
        Placement::State(c.state.clone().with("facing", c.face.opposite().name()))
    });
assert_eq!(behavior.block().to_string(), "example:post");
```

## Following the Neighbours

`update_shape` is asked when a neighbour changes. Answer with the new state, or `None` to keep it. This one remembers whether the block to its north is a post as well:

```rust
use pluto_lib::{BlockBehavior, Direction, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap())
    .update_shape(|c| {
        if c.direction != Direction::North {
            return None;
        }
        let joined = c.neighbour.as_ref().is_some_and(|n| n.block.path() == "post");
        Some(c.state.clone().with("joined", if joined { "true" } else { "false" }))
    });
assert!(behavior.block().path() == "post");
```

> **Tip.** The game asks at least six times for every block set. Keep `update_shape` short, and answer `None` when nothing changes.

## Growing Over Time

`random_tick` runs when the game picks the block for a random tick, a few blocks in every chunk section each tick. The callback gets a `TickContext` whose `world` can read the blocks around and change them. This block turns into its grown form one time in four:

```rust
use pluto_lib::{BlockBehavior, BlockBehaviors, Identifier};

BlockBehaviors::register(BlockBehavior::new(Identifier::new("example", "shoot").unwrap()).random_tick(|c| {
    if c.world.random_int(4) == 0 {
        c.world.set_block_state(c.pos, &c.state.clone().with("grown", "true"));
    }
}));
```

`scheduled_tick` is the same for a tick the block asked for with `world.schedule_tick(pos, "example:shoot", 20)`.

> **Use with Caution.** Having a `random_tick` or `scheduled_tick` callback stops the game's own tick of that block, as the original blocks did. Do everything the block needs in the callback. Roll dice with `world.random_int` and not with your own random numbers, so the order of draws is the game's.

## Bone Meal

Three callbacks decide what a bone meal does: `can_bonemeal` says whether it can be used (a used meal is spent only on yes), `bonemeal_success` whether it works this time, and `on_bonemeal` what grows:

```rust
use pluto_lib::{BlockBehavior, BlockBehaviors, Identifier};

BlockBehaviors::register(
    BlockBehavior::new(Identifier::new("example", "sapling").unwrap())
        .can_bonemeal(|c| c.view.is_air([c.pos[0], c.pos[1] + 1, c.pos[2]]))
        .bonemeal_success(|c| c.world.random_int(100) < 45)
        .on_bonemeal(|c| {
            c.world.set_block_state(c.pos, &c.state.clone().with("stage", "1"));
        }),
);
```

## Replacement and Generator Limits

Use `is_replaceable` when an algorithm asks whether a state can be replaced; air, collision shape and block tags are different questions. Use `generation_depth` for a generator's vertical depth, never infer it from the build ceiling. A callback without a generator context returns `None`:

```rust
use pluto_lib::{BlockView, MemoryBlockView};
let view = MemoryBlockView::default();
assert!(view.is_replaceable([0, 64, 0])); // empty memory world
assert_eq!(view.generation_depth(), None);
```

> **Use with Caution.** Native FEATURES-region queries are implemented; live views currently have no generator-depth context and return `None`. The bridge passes a full native build and the four-island Blood generation/reopen fixture; this does not establish live generator depth or full predicate/RNG parity. Custom views default to air-only replacement and no depth; they must opt in rather than silently inventing a generator limit.

## Testing


`MemoryBlockView` is a small world for tests, and a `BlockEditor` too: the blocks you set are solid, everything else is air, `rolls` holds the numbers `random_int` hands out and `scheduled` collects the ticks asked for. Build the context yourself and call your own functions:

```rust
use pluto_lib::{BlockState, BlockView, Direction, MemoryBlockView};

let mut view = MemoryBlockView::default();
assert!(view.is_air([0, 65, 0]));
view.blocks.insert([0, 65, 0], BlockState::parse("minecraft:stone").unwrap());
assert!(view.is_face_sturdy(Direction::Up.relative([0, 64, 0]), Direction::Down));
```

## Handling Landing Damage

A block may change calculated landing damage through `fall_on`. Damage and bounce velocity are separate: setting damage to zero does not make an entity bounce. The verified adapter handles resident-player landings after native block-contact listeners. Concrete mob damage/post-fall adapters also pass the pinned build and four-kind cow bounce/health gate with a stone damage control; other species remain unverified. The item post-fall expression adapter passes the full native build and four-kind dropped-item bounce/retained-stack game gate with stone and unregistered-inset negatives. The pinned native build and a resident-player game gate pass: zero-damage landings preserve health while an unregistered stone control takes damage. Other entities and LAN remain unverified.

```rust
use pluto_lib::{BlockBehavior, BlockBehaviors, Identifier};
BlockBehaviors::register(BlockBehavior::new(Identifier::new("example", "cushion").unwrap())
    .fall_on(|c| c.damage = 0.0));
```

> **Use with Caution.** Callbacks run without the behaviour-registry lock. A non-finite damage result is ignored and negative damage clamps to zero. No velocity, fall-distance reset, particles or network synchronization is implied.

## Static Piston Reactions

A declared mod block can override its template's piston reaction for every state. This is a static block property, not a sticky-pair predicate or a request to move a block entity:

```rust
use pluto_lib::{BlockBehavior, BlockBehaviors, Identifier, PistonBehavior};
BlockBehaviors::register(BlockBehavior::new(Identifier::parse("example:wall_head").unwrap())
    .piston_behavior(PistonBehavior::Normal));
```

> **Use with Caution.** Register before content generation collects its immutable settings snapshot; later registrations do not change that snapshot. The source adapter validates declared mod-block IDs, includes the settings in the content-root cache identity and patches individual gameplay rows without copying the whole gameplay export. Vanilla/unregistered block requests reject root generation. Native types/sites and actual piston behavior remain unverified behind native attachment review. Normal does not bypass the native block-entity movement veto. Omitting an override preserves the template.

## Reference

### `Direction`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

One of the six block faces: `Down`, `Up`, `North`, `South`, `West`, `East`.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::North.name(), "north");
assert_ne!(Direction::Up, Direction::Down);
```

### `Direction::ALL`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

All six directions, in the game's order: down, up, north, south, west, east.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::ALL.len(), 6);
assert_eq!(Direction::ALL[0], Direction::Down);
```

### `Direction::name`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn name(self) -> &'static str`. The name a block state spells it with: `down`, `up`, `north`, `south`, `west`, `east`.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::East.name(), "east");
```

### `Direction::parse`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn parse(name: &str) -> Option<Direction>`. Reads a name written by `name`; None for anything else.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::parse("west"), Some(Direction::West));
assert_eq!(Direction::parse("sideways"), None);
```

### `Direction::offset`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn offset(self) -> [i32; 3]`. The step one block in this direction takes: `[0, 1, 0]` for up, `[0, 0, -1]` for north.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::Up.offset(), [0, 1, 0]);
assert_eq!(Direction::North.offset(), [0, 0, -1]);
```

### `Direction::from_offset`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn from_offset(offset: [i32; 3]) -> Option<Direction>`. Reads a step written by `offset`; None for a step that is not one block along an axis.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::from_offset([1, 0, 0]), Some(Direction::East));
assert_eq!(Direction::from_offset([1, 1, 0]), None);
```

### `Direction::opposite`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn opposite(self) -> Direction`. The other way: down for up, north for south.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::South.opposite(), Direction::North);
```

### `Direction::clockwise`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn clockwise(self) -> Direction`. A quarter turn clockwise seen from above (north, east, south, west); up and down stay as they are.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::North.clockwise(), Direction::East);
assert_eq!(Direction::Down.clockwise(), Direction::Down);
```

### `Direction::nearest_looking`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn nearest_looking(yaw: f32, pitch: f32) -> [Direction; 6]`. The six directions ordered by how closely a player turned `yaw` degrees and looking `pitch` degrees (negative looks up) faces them, nearest first.

```rust
use pluto_lib::Direction;

let order = Direction::nearest_looking(0.0, 0.0); // facing south, level
assert_eq!(order[0], Direction::South);
assert_eq!(order[5], Direction::North);
assert_eq!(Direction::nearest_looking(0.0, -90.0)[0], Direction::Up);
```

### `Direction::relative`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn relative(self, pos: BlockPos) -> BlockPos`. The position one block from `pos` in this direction.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::West.relative([5, 64, 5]), [4, 64, 5]);
```

### Neighbor signal queries

`BlockView::has_neighbor_signal(&self, pos: BlockPos) -> Option<bool>` is Minecraft's `Level.hasNeighborSignal` query. `Some(false)` is a known unpowered answer; `None` means unavailable, not false. It does not inspect the block's `powered` property or infer power from tags. Native views without a signal context return `None` (stub). A borrowed client-view adapter now source-delegates to the real native query after validating its origin, but its tick driver is unconnected and native compilation/gameplay remain unverified. Native code retains its own neighbor/conductor semantics; this does not emulate redstone from properties or tags. `MemoryBlockView::neighbor_signals` supplies explicit test answers; unloaded positions still return `None`, and memory does not simulate redstone.

```rust
use pluto_lib::{BlockView, MemoryBlockView};
let mut world = MemoryBlockView::default();
assert_eq!(world.has_neighbor_signal([1, 64, 2]), None);
world.neighbor_signals.insert([1, 64, 2], false);
assert_eq!(world.has_neighbor_signal([1, 64, 2]), Some(false));
world.neighbor_signals.insert([1, 64, 2], true);
assert_eq!(world.has_neighbor_signal([1, 64, 2]), Some(true));
world.unloaded.insert([1, 64, 2]);
assert_eq!(world.has_neighbor_signal([1, 64, 2]), None);
```

> **Use with Caution.** Skip work requiring power information when the query is unavailable; a fabricated false can stop an animation or change machine behavior incorrectly.

### `BlockView`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

What a behaviour can see of the blocks around, with the game's own answers: `block_state(pos) -> Option<BlockState>` is the state with its properties (None where nothing is loaded), `is_air(pos) -> bool`, `is_face_sturdy(pos, face) -> bool` whether the whole face is solid, `is_in_tag(pos, tag) -> bool` whether the block is in a block tag such as `tconstruct:slimy_leaves`, `brightness(pos) -> u8` the light at the position (0 to 15), `light_passes_up(pos) -> bool` whether light passes from that block into the one above it, `is_water(pos) -> bool` whether it holds water, `is_fluid_full(pos) -> bool` whether its fluid is a full source, `is_area_loaded(pos, radius) -> bool` whether the chunks around are loaded (check it in a tick callback before touching blocks near the edge), `is_full_block(pos) -> bool` whether its collision shape is a whole block and `build_height() -> [i32; 2]` the lowest buildable y and the first y above the build limit. Three more answers ride on the same view: `face_sturdy(pos, face) -> Option<bool>` whether the face is sturdy when the game's table has an answer for the block, `raw_brightness(pos) -> Option<u8>` the light before the sky darkens it, and `placement_facts(state) -> Option<PlacementFacts>` the published shape and class facts for the state.

```rust
use pluto_lib::{BlockState, BlockView, Direction};

/// A view of a world with one stone block.
struct OneStone;
impl BlockView for OneStone {
    fn block_state(&self, pos: [i32; 3]) -> Option<BlockState> {
        (pos == [0, 0, 0]).then(|| BlockState::parse("minecraft:stone").unwrap())
    }
    fn is_air(&self, pos: [i32; 3]) -> bool { pos != [0, 0, 0] }
    fn is_face_sturdy(&self, pos: [i32; 3], _face: Direction) -> bool { pos == [0, 0, 0] }
    fn is_in_tag(&self, _pos: [i32; 3], tag: &str) -> bool { tag == "minecraft:base_stone_overworld" }
    fn brightness(&self, _pos: [i32; 3]) -> u8 { 15 }
    fn light_passes_up(&self, pos: [i32; 3]) -> bool { pos != [0, 0, 0] }
    fn is_water(&self, _pos: [i32; 3]) -> bool { false }
    fn is_fluid_full(&self, _pos: [i32; 3]) -> bool { false }
    fn is_area_loaded(&self, _pos: [i32; 3], _radius: i32) -> bool { true }
    fn is_full_block(&self, pos: [i32; 3]) -> bool { pos == [0, 0, 0] }
    fn build_height(&self) -> [i32; 2] { [-64, 320] }
}

assert!(OneStone.is_face_sturdy([0, 0, 0], Direction::Up));
assert!(OneStone.is_air([1, 0, 0]));
```

#### Replacement Queries

`fn is_replaceable(&self, pos: [i32; 3]) -> bool`. The native state's replaceable flag. Custom views default to `is_air(pos)`; memory worlds also accept positions explicitly listed in `replaceable`. This is not a check for a non-full collision box or a particular tag.

```rust
use pluto_lib::{BlockState, BlockView, MemoryBlockView};
let mut view = MemoryBlockView::default();
view.blocks.insert([1, 64, 0], BlockState::parse("minecraft:stone").unwrap());
assert!(!view.is_replaceable([1, 64, 0]));
view.replaceable.insert([1, 64, 0]);
assert!(view.is_replaceable([1, 64, 0]));
```

#### Generator Depth

`fn generation_depth(&self) -> Option<i32>`. Vertical depth of the active chunk generator, independently of world build limits. Native FEATURES callbacks supply it; live callbacks and custom views without generator context return `None`. Memory worlds explicitly set `generator_depth`.

```rust
use pluto_lib::{BlockView, MemoryBlockView};
let mut view = MemoryBlockView::default();
assert_eq!(view.generation_depth(), None);
view.generator_depth = Some(128);
assert_eq!(view.generation_depth(), Some(128));
assert_eq!(view.build_height(), [-64, 320]); // build range is a different query
```

### `MemoryBlockView`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

A `BlockView` for tests. `replaceable` lists explicitly replaceable non-air positions; `generator_depth` supplies an optional depth independently of build bounds. `neighbor_signals: BTreeMap<BlockPos, bool>` supplies explicit neighbor-signal answers (`None` when absent or unloaded), without redstone simulation. `blocks` holds the states set (each position in it is solid and not air), `weak` the positions that hold a block but not a sturdy one (a slab's top, glass), `tags` the tags the block at each position is in, `state_tags` explicit registry tags by block id, `light` the light of each position (15 where not listed), `water` the positions that hold a full source of water, `unloaded` the positions that are not loaded, `rolls` the numbers `random_int` hands out (each modulo its bound, 0 once used up), `floats` those of `random_float` (0.0 once used up) and `scheduled` the ticks asked for and `features` the features placed. `placement_facts` holds the placement facts a test supplies by block name, for `can_survive` and `is_unobstructed`. `break_requests` records destruction requests while removing their blocks without native loot/fluid effects. `writes` records each successful write as `(position, state, flags)`, including ordinary flags=3 writes; it records requests, not simulated neighbour/client effects.

```rust
use pluto_lib::{BlockState, BlockView, Direction, MemoryBlockView};

let mut view = MemoryBlockView::default();
view.blocks.insert([0, 0, 0], BlockState::parse("minecraft:glass").unwrap());
view.weak.insert([0, 0, 0]);
assert!(!view.is_air([0, 0, 0]));
assert!(!view.is_face_sturdy([0, 0, 0], Direction::Up));
assert_eq!(view.block_state([0, 0, 0]).unwrap().to_string(), "minecraft:glass");
```

### `BlockEditor`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

**Breaking blocks.**

`fn break_block(&mut self, pos: [i32; 3], drop: bool) -> Option<bool>` uses native destruction, with particles, optional loot and replacement by the contained fluid. It is not `remove_block`, which writes air. `None` means that editor cannot perform destruction; `Some(false)` is a supported call that changed nothing, including an already-air position. Live block callbacks and untracked FEATURES editors implement it; tracked tree editors return `None` so neighbour changes cannot escape tree tracking. The full native build and a live planted-fungus fixture pass. Native loot contents, particles, fluid replacement and FEATURES destruction effects are not independently game-verified.

```rust
use pluto_lib::{BlockEditor, BlockState, BlockView, MemoryBlockView};
let mut world = MemoryBlockView::default();
world.blocks.insert([0, 64, 0], BlockState::parse("minecraft:stone").unwrap());
assert_eq!(world.break_block([0, 64, 0], true), Some(true));
assert_eq!(world.break_block([0, 64, 0], false), Some(false));
assert!(world.is_air([0, 64, 0]));
assert_eq!(world.break_requests, vec![([0, 64, 0], true), ([0, 64, 0], false)]);
```

> **Use with Caution.** Memory worlds remove blocks and record `break_requests`; they do not simulate loot, particles, fluid replacement or neighbour effects. Custom editors default to `None` without writes. Earlier successful changes are not rolled back when a later operation is unavailable.


A `BlockView` that can also change the world, handed to tick and bone meal callbacks as `world`. `set_block_state(pos, &state) -> bool` sets a block (neighbours and clients are told) and is false if it could not be set, `remove_block(pos) -> bool` replaces a block with air, `random_int(bound) -> i32` is a number from 0 up to but not including `bound` and `random_float() -> f32` one from 0.0 up to but not including 1.0, both from the level's own random source, `schedule_tick(pos, block, delay) -> bool` asks the game to tick `block` at `pos` after `delay` game ticks, and `place_feature(feature, pos) -> bool` places a configured worldgen feature declared in data (false if it is unknown or found no room).

```rust
use pluto_lib::{BlockEditor, BlockState, BlockView, MemoryBlockView};

let mut world = MemoryBlockView::default();
world.rolls = vec![7];
assert!(world.set_block_state([0, 64, 0], &BlockState::parse("minecraft:stone").unwrap()));
assert!(!world.is_air([0, 64, 0]));
assert!(world.remove_block([0, 64, 0]));
assert!(world.is_air([0, 64, 0]));
assert_eq!(world.random_int(5), 2); // 7 modulo 5
assert_eq!(world.random_int(5), 0); // used up
assert!(world.schedule_tick([0, 64, 0], "example:shoot", 20));
assert_eq!(world.scheduled.len(), 1);
assert!(world.place_feature("example:big_shoot", [0, 64, 0]));
assert_eq!(world.features[0].0, "example:big_shoot");
```

> **Use with Caution.** `place_feature` delegates live placement to the active native catalog through PlutoLib's concrete level boundary, preserving the native level RNG and updates. Four real bone-meal fungus-growth cases have been checked. This call does not register new feature types, root placers or decorators, and does not connect a custom algorithm to natural generation. `MemoryBlockView` records requests; it does not generate terrain.

### Explicit Update Flags

`fn set_block_state_with_flags(&mut self, pos: BlockPos, state: &BlockState, flags: i32) -> bool`. Write with explicit Minecraft flags instead of the ordinary `set_block_state` flags=3. The live adapter forwards the value unchanged: 1 notifies neighbours, 2 clients, and 16 marks a known shape (skipping neighbour shape updates). For example, 19 is 1 | 2 | 16. Other engine flag bits remain the engine's contract; this method does not normalize the mask.

```rust
use pluto_lib::{BlockEditor, BlockState, MemoryBlockView};

let mut world = MemoryBlockView::default();
let root = BlockState::parse("minecraft:mangrove_roots").unwrap();
assert!(world.set_block_state_with_flags([-3, 65, 8], &root, 1 | 2 | 16));
assert_eq!(world.writes, vec![([-3, 65, 8], root, 19)]);
```

> **Use with Caution.** Real bone-meal feature placement checked shape notifications for masks 3/2 and suppression for 18/19; other flag effects are unverified. `MemoryBlockView` records exact flags but does not emulate the engine's update queues. Existing third-party editors keep a conservative default: flags=3 calls their existing setter; any other value returns false without a write, never silently substituting 3. Override the method to support additional masks. Writes may have side effects and do not form a rollback transaction.

### Native Trees with Extra Decorators

`fn place_tree(&mut self, configuration: &serde_json::Value, pos: BlockPos, decorators: &[TreeDecorator]) -> bool`. Decode a complete native `minecraft:tree` configured-feature value against the active catalog, place its native roots/trunk/foliage and native decorators, then run the supplied extra decorators in order before native leaf-distance and edge-shape finalization. It returns false for invalid configurations, placement failure or an extra decorator error. Decode extra decorator codecs before calling this method.

```rust
use pluto_lib::{BlockEditor, BlockState, MemoryBlockView, TreeDecorator};

fn grow_with_marker(world: &mut dyn BlockEditor, tree: &serde_json::Value) -> bool {
    let decorator = TreeDecorator::new(|context| {
        if let Some(&leaf) = context.leaves.first() {
            let marker = [leaf[0] + 1, leaf[1], leaf[2]];
            if !context.world.set_block_state_with_flags(marker, &BlockState::parse("minecraft:stone").unwrap(), 19) {
                return Err("marker write failed".into());
            }
        }
        Ok(())
    });
    world.place_tree(tree, [0, 64, 0], &[decorator])
}
// Supply a full native tree configuration in game. MemoryBlockView does not
// generate native trees and must reject even a syntactically valid request.
let mut world = MemoryBlockView::default();
assert!(!grow_with_marker(&mut world, &serde_json::Value::Null));
assert!(world.writes.is_empty());
```

> **Use with Caution.** A live oak fixture checked logs/leaves/vines, final leaf distances, tracked extra-write edge updates, callback failure ownership and invalid-config rejection; its three images were inspected. Extra decorators are appended, not interleaved with native decorators. Writes use the same native placement RNG and are tracked for native finalization; on an error earlier writes remain and finalization stops. The advanced RNG is written back even on unwind, then the original panic resumes. Existing custom editors and `MemoryBlockView` return false without writing or invoking decorators. Inside a live tree's extra callback, nested `place_feature` and `place_tree` return false without writing until nested write tracking is implemented. Custom root overrides use the separate method below. Natural generation and automatic decoding of registered decorators inside native tree JSON remain unconnected.

### Root Placement Overrides

`fn place_tree_with_roots(&mut self, configuration: &serde_json::Value, pos: BlockPos, roots: &RootPlacement, decorators: &[TreeDecorator]) -> bool`. Like `place_tree`, but run a per-position override before native parent root placement. The base tree still selects its native root simulator/shape. Every holder/provider descriptor binds to the active registry before the tree writes anything. `Some(state)` is waterlogged and tracked as a root, `None` calls the native parent for that position, and `Err` stops without parent fallback or rollback. Extra decorators still run after native decorators and before finalization.

```rust
use pluto_lib::{BlockEditor, MemoryBlockView, RootPlacement, WorldgenBindings};
let roots = RootPlacement::new(WorldgenBindings::default(), |_| panic!("unsupported editor must not call roots"));
let mut world = MemoryBlockView::default();
assert!(!world.place_tree_with_roots(&serde_json::Value::Null, [0, 64, 0], &roots, &[]));
assert!(world.writes.is_empty());
```

> **Use with Caution.** Five live fixtures checked weighted first-match selection, waterlogging, native parent fallback, no-state provider errors and eager rejection of invalid unselected bindings; all five images were inspected. Custom/memory editors default to false without writes/callbacks. This method does not replace native root simulation, automatically decode custom root types inside native JSON, or connect natural generation. Nested tree/feature requests inside root callbacks are rejected until their writes can be merged into outer tracking. Provider failure must never select another provider or request parent placement. See [worldgen](worldgen.md) for binding descriptors and codecs.

### Ordered Tree Decoration

`fn place_tree_with_plan(&mut self, configuration: &serde_json::Value, pos: BlockPos, roots: Option<&RootPlacement>, plan: &[TreeDecoration]) -> bool`. Run native and custom decorators in the supplied order, after native tree geometry and before native finalization. Pass `None` to keep native roots or `Some(&roots)` for bound per-position overrides. Every native decorator index must appear exactly once; invalid plans reject before tree writes or callbacks. At most 512 steps and 256 native decorators are accepted. Configuration and root bindings still decode before writes.

```rust
use pluto_lib::{BlockEditor, MemoryBlockView, RootPlacement, WorldgenBindings, TreeDecoration, TreeDecorator};
let mut world = MemoryBlockView::default();
let plan = [TreeDecoration::Custom(TreeDecorator::new(|_| panic!("unsupported editor")))];
assert!(!world.place_tree_with_plan(&serde_json::Value::Null, [0, 64, 0], None, &plan));
let roots = RootPlacement::new(WorldgenBindings::default(), |_| panic!("unsupported roots"));
assert!(!world.place_tree_with_plan(&serde_json::Value::Null, [0, 64, 0], Some(&roots), &plan));
assert!(world.writes.is_empty());
```

> **Use with Caution.** Three live fixtures checked interleaving, error ownership and pre-write invalid-plan rejection; all three images were inspected. All steps share the original position lists and placement RNG; successful custom writes are tracked before the next step. Errors stop remaining decorators and finalization without rollback. Default/custom-memory editors reject without writes or callbacks. Nested placement remains unsupported. Existing append-only methods keep their ordering. See [worldgen](worldgen.md#treedecoration) for both step variants.

### `TickContext`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

A tick or a bone meal reached the block. `state` is the block's state, `pos` its position and `world` the world around, which can be changed.

```rust
use pluto_lib::{BlockState, MemoryBlockView, TickContext};

let mut world = MemoryBlockView::default();
let mut context = TickContext { state: BlockState::parse("example:shoot").unwrap(), pos: [0, 64, 0], world: &mut world };
assert_eq!(context.pos, [0, 64, 0]);
assert_eq!(context.state.block.path(), "shoot");
assert!(context.world.set_block_state(context.pos, &context.state.clone()));
```

### `BonemealContext`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

Where a bone meal is about to be used. `state` is the block's state, `pos` its position and `view` the blocks around.

```rust
use pluto_lib::{BlockState, BonemealContext, MemoryBlockView};

let view = MemoryBlockView::default();
let context = BonemealContext { state: BlockState::parse("example:sapling").unwrap(), pos: [0, 64, 0], view: &view };
assert!(context.view.is_air([context.pos[0], context.pos[1] + 1, context.pos[2]]));
assert_eq!(context.state.block.path(), "sapling");
```

### `PlaceOnContext`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

Where a plant is asked what it may stand on. `state` is the plant's state, `ground` the block below and `ground_pos` its position, `view` the blocks around.

```rust
use pluto_lib::{BlockState, MemoryBlockView, PlaceOnContext};

let view = MemoryBlockView::default();
let context = PlaceOnContext {
    state: BlockState::parse("example:fern").unwrap(),
    ground: BlockState::parse("minecraft:dirt").unwrap(),
    ground_pos: [0, 63, 0],
    view: &view,
};
assert_eq!(context.ground.block.path(), "dirt");
assert_eq!(context.ground_pos, [0, 63, 0]);
assert_eq!(context.state.block.path(), "fern");
assert!(context.view.is_air([0, 64, 0]));
```

### `SurviveContext`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

Where a block is asked whether it may stay. `state` is the block's state (it may not be in the world yet), `pos` the position and `view` the blocks around.

```rust
use pluto_lib::{BlockState, MemoryBlockView, SurviveContext};

let view = MemoryBlockView::default();
let context = SurviveContext { state: BlockState::parse("example:hook").unwrap(), pos: [0, 64, 0], view: &view };
assert_eq!(context.pos, [0, 64, 0]);
assert_eq!(context.state.to_string(), "example:hook");
assert!(context.view.is_air([0, 65, 0]));
```

### `PlacementContext`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

What a placement is told. `state` is the block's default state, `pos` where it would go, `face` the face that was clicked, `yaw` and `pitch` the placer's turn and look in degrees, `sneaking` whether the placer sneaks, `in_water` whether the position holds water now, `item` the item in hand, `replace_clicked` whether the clicked block is replaced (grass, a snow layer) so the block goes into the clicked position itself, and `view` the blocks around.

```rust
use pluto_lib::{BlockState, Direction, MemoryBlockView, PlacementContext};

let view = MemoryBlockView::default();
let context = PlacementContext {
    state: BlockState::parse("example:post").unwrap(),
    pos: [1, 64, 1],
    face: Direction::Up,
    yaw: 90.0,
    pitch: 0.0,
    sneaking: false,
    in_water: false,
    item: "example:post".into(),
    replace_clicked: false,
    view: &view,
};
assert!(!context.replace_clicked);
assert_eq!(context.face.opposite(), Direction::Down);
assert!(!context.sneaking && !context.in_water);
assert_eq!((context.pos, context.yaw, context.pitch), ([1, 64, 1], 90.0, 0.0));
assert_eq!(context.item, "example:post");
assert_eq!(context.state.to_string(), "example:post");
assert!(context.view.is_air([1, 63, 1]));
```

### `PlacementContext::nearest_looking_directions`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn nearest_looking_directions(&self) -> [Direction; 6]`. The directions in the order the placer looks at them, nearest first, with the side opposite the clicked face moved to the front unless the clicked block is replaced. A vine tries them in this order to find a side it can hold on.

```rust
use pluto_lib::{BlockState, Direction, MemoryBlockView, PlacementContext};

let view = MemoryBlockView::default();
let context = PlacementContext {
    state: BlockState::parse("example:vine").unwrap(),
    pos: [0, 64, 0],
    face: Direction::East,
    yaw: 0.0,
    pitch: 0.0,
    sneaking: false,
    in_water: false,
    item: String::new(),
    replace_clicked: false,
    view: &view,
};
assert_eq!(context.nearest_looking_directions()[0], Direction::West); // the side opposite the clicked face
```

### `Placement`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

What a placement becomes: `Placement::Default` lets the game's own placement run, `Placement::Refuse` places nothing and keeps the item, `Placement::State(state)` places the block in `state` (it has to be a state of the same block, with the properties declared).

```rust
use pluto_lib::{BlockState, Placement};

let state = BlockState::parse("example:post[lit=true]").unwrap();
assert_ne!(Placement::Default, Placement::Refuse);
assert_eq!(Placement::State(state.clone()), Placement::State(state));
```

### `ShapeUpdateContext`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

A neighbour changed. `state` is the block's state, `pos` its position, `direction` the side the change came from, `neighbour_pos` the neighbour's position, `neighbour` the neighbour's state if it is a block with a behaviour, and `view` the blocks around.

```rust
use pluto_lib::{BlockState, Direction, MemoryBlockView, ShapeUpdateContext};

let view = MemoryBlockView::default();
let context = ShapeUpdateContext::new(
    BlockState::parse("example:post").unwrap(), [0, 64, 0],
    Direction::North, [0, 64, -1], None, &view,
);
assert_eq!(context.direction.relative(context.pos), context.neighbour_pos);
assert!(context.neighbour.is_none());
assert_eq!(context.state.block.path(), "post");
assert!(context.view.is_air(context.neighbour_pos));
```

### `ShapeUpdateContext::new`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn new(state: BlockState, pos: BlockPos, direction: Direction, neighbour_pos: BlockPos, neighbour: Option<BlockState>, view: &dyn BlockView) -> ShapeUpdateContext`. Creates a borrowed update with no tick request. The supplied neighbour may differ from the state currently in the world.

```rust
use pluto_lib::{BlockState, Direction, MemoryBlockView, ShapeUpdateContext};
let view = MemoryBlockView::default();
let update = ShapeUpdateContext::new(BlockState::parse("example:leaves").unwrap(),
    [0,64,0], Direction::East, [1,64,0], None, &view);
assert_eq!(update.scheduled_tick_delay(), None);
```

### `ShapeUpdateContext::schedule_tick`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn schedule_tick(&self, delay: i32) -> bool`. Requests this block's scheduled tick at `pos` without changing its state. Positive delays are accepted; repeated requests keep the earliest. The native queue still checks loaded chunks and deduplicates; accepting a request is not proof it executed. Source forwarding uses the existing update hook's tick queue; native validation is pending the native attachment gate.

```rust
use pluto_lib::{BlockState, Direction, MemoryBlockView, ShapeUpdateContext};
let view = MemoryBlockView::default();
let update = ShapeUpdateContext::new(BlockState::parse("example:leaves").unwrap(),
    [0,64,0], Direction::Up, [0,65,0], None, &view);
assert!(!update.schedule_tick(0));
assert!(update.schedule_tick(5));
assert!(update.schedule_tick(1));
assert!(update.schedule_tick(3));
assert_eq!(update.scheduled_tick_delay(), Some(1));
```

### `ShapeUpdateContext::scheduled_tick_delay`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn scheduled_tick_delay(&self) -> Option<i32>`. Reads the request for an adapter/custom host; it does not enqueue or execute a tick by itself.

```rust
use pluto_lib::{BlockState, Direction, MemoryBlockView, ShapeUpdateContext};
let view = MemoryBlockView::default();
let update = ShapeUpdateContext::new(BlockState::parse("example:block").unwrap(),
    [0,64,0], Direction::Up, [0,65,0], None, &view);
assert_eq!(update.scheduled_tick_delay(), None);
update.schedule_tick(1);
assert_eq!(update.scheduled_tick_delay(), Some(1));
```

### Method `BlockView::is_state_in_tag`

`fn is_state_in_tag(&self, state: &BlockState, tag: &str) -> Option<bool>`. Queries registry membership of the supplied state, not the block at a position. Native live views source-forward the active registry; unsupported views return `None`. `MemoryBlockView::state_tags` supplies explicit test answers.

```rust
use pluto_lib::{BlockState, BlockView, MemoryBlockView};
let mut view = MemoryBlockView::default();
let log = BlockState::parse("example:log[axis=x]").unwrap();
assert_eq!(view.is_state_in_tag(&log, "example:logs"), None);
view.state_tags.insert("example:log".into(), ["example:logs".into()].into_iter().collect());
assert_eq!(view.is_state_in_tag(&log, "example:logs"), Some(true));
assert_eq!(view.is_state_in_tag(&log, "example:leaves"), Some(false));
```

### `EntityInsideContext`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

An entity is inside the block. `dimension` is the dimension id, `state` the block's state, `pos` its position, `entity` the entity's id and `kind` its type. `bounds` is the entity's bounding box as `[min x, min y, min z, max x, max y, max z]` and `fall_distance` how far it has fallen, in blocks: the two numbers the original's contact tests and fall bonuses are computed from (a punji hurts only a living entity whose box crosses the block's mid-plane, and deals the fall distance + 1 more after a fall).

```rust
use pluto_lib::{BlockState, EntityInsideContext};

let context = EntityInsideContext {
    dimension: "minecraft:overworld".into(),
    state: BlockState::parse("example:post").unwrap(),
    pos: [0, 64, 0],
    entity: 12,
    kind: "minecraft:zombie".into(),
    bounds: [0.2, 64.0, 0.2, 0.8, 65.9, 0.8],
    fall_distance: 3.5,
    actions: Default::default(),
};
assert_eq!(context.kind, "minecraft:zombie");
assert_eq!(context.bounds[4], 65.9);
assert_eq!(context.fall_distance, 3.5);
assert_eq!((context.entity, context.pos), (12, [0, 64, 0]));
assert_eq!(context.dimension, "minecraft:overworld");
assert_eq!(context.state.block.path(), "post");
```

### `PistonBehavior`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

A static block piston property: `Normal` permits ordinary push/pull checks; `Destroy` permits destruction when the resolver allows it; `Block` refuses movement; `Ignore` is Minecraft's ignore reaction; `PushOnly` permits pushing but not pulling. The native resolver retains its bounds, hardness, extended-piston and block-entity checks.

```rust
use pluto_lib::{BlockBehavior, Identifier, PistonBehavior};
for reaction in [PistonBehavior::Normal, PistonBehavior::Destroy, PistonBehavior::Block,
    PistonBehavior::Ignore, PistonBehavior::PushOnly] {
    let block = BlockBehavior::new(Identifier::parse("example:reaction").unwrap()).piston_behavior(reaction);
    assert_eq!(block.piston_behavior_override(), Some(reaction));
}
```

### `BlockBehavior::piston_behavior`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn piston_behavior(self, behavior: PistonBehavior) -> BlockBehavior`. Assigns the static reaction for all states of an already declared mod block. Call before the content snapshot is collected; the native adapter is source-declared, not runtime-verified.

```rust
use pluto_lib::{BlockBehavior, Identifier, PistonBehavior};
let block = BlockBehavior::new(Identifier::parse("example:fragile").unwrap())
    .piston_behavior(PistonBehavior::Destroy);
assert_eq!(block.piston_behavior_override(), Some(PistonBehavior::Destroy));
```

### `BlockBehavior::piston_behavior_override`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn piston_behavior_override(&self) -> Option<PistonBehavior>`. Reads only the explicitly registered setting; `None` retains the native content template rather than assuming Normal.

```rust
use pluto_lib::{BlockBehavior, Identifier, PistonBehavior};
let block = BlockBehavior::new(Identifier::parse("example:template").unwrap());
assert_eq!(block.piston_behavior_override(), None);
let block = block.piston_behavior(PistonBehavior::Block);
assert_eq!(block.piston_behavior_override(), Some(PistonBehavior::Block));
```

### `PathNodeType`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

A host-independent Minecraft path-node category. Variants are `Blocked`, `Open`, `Walkable`, `WalkableDoor`, `Trapdoor`, `PowderSnow`, `OnTopOfPowderSnow`, `Fence`, `Lava`, `Water`, `WaterBorder`, `Rail`, `UnpassableRail`, `FireInNeighbor`, `Fire`, `DamagingInNeighbor`, `Damaging`, `DoorOpen`, `DoorWoodClosed`, `DoorIronClosed`, `Breach`, `Leaves`, `StickyHoney`, `Cocoa`, `DamageCautious`, `OnTopOfTrapdoor`, and `BigMobsCloseToDanger`. `name(self) -> &'static str` returns the uppercase Minecraft name; `parse(name: &str) -> Option<Self>` rejects unknown names rather than silently blocking a route.

```rust
use pluto_lib::PathNodeType;
assert_eq!(PathNodeType::StickyHoney.name(), "STICKY_HONEY");
assert_eq!(PathNodeType::parse("BLOCKED"), Some(PathNodeType::Blocked));
assert_eq!(PathNodeType::parse("UNKNOWN_CATEGORY"), None);
```

### `PathNodeType::name`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

`fn name(self) -> &'static str`. Return this category's uppercase Minecraft name.

```rust
use pluto_lib::PathNodeType;
assert_eq!(PathNodeType::StickyHoney.name(), "STICKY_HONEY");
```

### `PathNodeType::parse`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

`fn parse(name: &str) -> Option<PathNodeType>`. Read an uppercase category name, returning `None` for an unknown category rather than a guessed blocked node.

```rust
use pluto_lib::PathNodeType;
assert_eq!(PathNodeType::parse("BLOCKED"), Some(PathNodeType::Blocked));
assert_eq!(PathNodeType::parse("UNKNOWN_CATEGORY"), None);
```

### `PathNodeTypeContext`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

A borrowed path query: `state: BlockState`, `pos: [i32; 3]`, `view: &dyn BlockView`, `entity: Option<i32>`, and `kind: Option<String>`. Static queries have no entity identity. The callback must not retain the borrowed view.

```rust
use pluto_lib::{BlockState, MemoryBlockView, PathNodeTypeContext};
let view = MemoryBlockView::default();
let query = PathNodeTypeContext { state: BlockState::parse("example:slow_floor").unwrap(),
    pos: [3, 64, 5], view: &view, entity: None, kind: None };
assert_eq!(query.pos, [3, 64, 5]);
assert!(query.entity.is_none());
```

### `BlockBehavior::path_node_type`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn path_node_type(self, f: impl Fn(&PathNodeTypeContext<'_>) -> Option<PathNodeType> + Send + Sync + 'static) -> BlockBehavior`. Classify this block's path node using its state and surroundings. `Some(Blocked)` explicitly blocks the node; `None` keeps the native answer. Consumer code runs outside the registration lock.

```rust
use pluto_lib::{BlockBehavior, BlockBehaviors, Identifier, PathNodeType};
BlockBehaviors::register(BlockBehavior::new(Identifier::new("example", "slow_floor").unwrap())
    .path_node_type(|query| (query.pos[1] >= 0).then_some(PathNodeType::StickyHoney)));
```

> **Use with Caution.** Native support is not yet verified. The borrowed-world adapter and static query hook are implemented and pass planning, but guarded compilation and game verification are pending. This entry point is static, so native queries have no entity identity. A registered category alone does not prove a mob will choose a different route.

### `EntityInsideMotionContext`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

Current clean-port source review (2026-11-14): mob contacts retain the native sweep captured before `stepOn`, including when native damage immediately teleports the mob. The adapter delegates contact damage to the pinned host's teleport-aware `hurt` function. A production-helper trace regression passes; the corrected bundle still requires combined-contract approval, native compilation and game verification. Historical gates below do not certify this changed input.

Mutable motion during a swept block contact: `state: BlockState`, `pos: [i32; 3]`, `entity: i32`, `kind: String`, `server: bool`, `suppressing_bounce: bool`, `position: [f64; 3]`, `previous_position: [f64; 3]`, `velocity: [f64; 3]`, `sync_velocity: bool`, and `step_sound: Option<[f32; 2]>` (volume/pitch). Position fields are the current and previous tick positions, not the sweep segment endpoints. Only finite `velocity` is applied; one non-finite component restores the complete original velocity and synchronization request. Setting `sync_velocity` requests the server's existing velocity packet after travel; false does not clear an earlier native mark. `step_sound` requests the contacted block's step sound at the block centre in the blocks category, without the native footstep volume multiplier. Volume and pitch must be finite/nonnegative; invalid velocity or sound restores all three outputs together. Requests use non-persistent world-owned storage, bounded to 256 entities and 256 sounds per entity, and are drained in the current authoritative player's effects phase. Queueing is not an audio-delivery guarantee. The player velocity/packet adapter passes the falling-side/suppression game gate; the pinned sound build and native-request gate also pass for four kinds, including volume/pitch 1 and no requests in suppression/control cases. Audible playback and broader LAN delivery remain unverified. The mob/item side adapters pass the guarded native build and four-face falling-contact gates for cows and dropped items on all four production kinds, with unregistered-inset negatives, retained item count, and same-entity callback/sync/native-sound checks. Their world-tick queue is separate from the player's clock, clones empty with world snapshots, never decodes saved attachments through a shared borrow, and releases its lock before delivery. All 91 images across these gates, landing/player-side regression and crafting/Part Builder are inspected. Other species, held-forward flat contact, exact per-tick parity and audible/broader LAN delivery remain unverified or unresolved.

```rust
use pluto_lib::{EntityInsideMotionContext, BlockState};
let c = EntityInsideMotionContext { state: BlockState::parse("example:pad").unwrap(), pos:[0,64,0],
    entity:1, kind:"minecraft:player".into(), server:true, suppressing_bounce:false,
    position:[0.0,64.2,0.5], previous_position:[-0.2,64.2,0.5], velocity:[0.2,-0.08,0.0], sync_velocity:false, step_sound:None };
assert_eq!(c.previous_position[0], -0.2);
```

### `AfterFallContext`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

A registered block's post-landing motion request: `state: BlockState`, `pos: [i32; 3]`, `entity: i32`, `kind: String`, `living: bool`, `item: bool`, `suppressing_bounce: bool`, `velocity: [f64; 3]`, `fall_distance: f64`, and `on_ground: bool`. Only the last three motion fields are applied back. Native horizontal restitution has already run; native vertical restitution does not run after this callback. A non-finite velocity/distance or negative distance restores all three original motion fields together. The handler is captured before native effects, so later registry replacement does not swap it mid-request. Native coverage includes resident players and concrete mobs. The pinned build, ordinary/suppressed player bounce and four-kind cow bounce/immunity with a stone damage control pass; exact per-tick parity, other species and LAN remain unverified. The item post-fall expression adapter passes the guarded native build and a real dropped-item gate: all four production kinds bounce, stone/unregistered-inset controls do not, and stack count stays one. The combined player/cow/side/sound regression and ordinary crafting/Part Builder pass on the same binary; all 35 images inspected. This is observed bounce, not an exact per-tick restitution proof. It retains native restitution for unregistered blocks, replaces only the scalar vertical result for registered handlers, and leaves the caller's later effects/drag/tracking intact. Items have no accumulated native landing-damage distance: `fall_distance` is zero and its output is not stored; velocity and `on_ground` outputs are applied. Moving-piston support is not covered.

```rust
use pluto_lib::{AfterFallContext, BlockState};
let c = AfterFallContext { state: BlockState::parse("example:cushion").unwrap(), pos: [0,64,0],
    entity: 1, kind: "minecraft:player".into(), living: true, item: false, suppressing_bounce: false,
    velocity: [0.0,-0.7,0.0], fall_distance: 5.0, on_ground: true };
assert_eq!(c.velocity[1], -0.7);
```

### `FallOnContext`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

Calculated server landing data: `state: BlockState`, `pos: [i32; 3]`, `entity: i32`, `kind: String`, `distance: f64`, and editable `damage: f32`. `distance` is the accumulated fall, not a velocity. Only `damage` is copied back to the native adapter; other fields describe the request. Verified coverage is resident players and the four-kind cow immunity/bounce fixture with a stone damage control. The concrete mob adapter is general, but other species are unverified; items are not connected.

```rust
use pluto_lib::{BlockState, FallOnContext};
let c = FallOnContext { state: BlockState::parse("example:cushion").unwrap(),
    pos: [0, 64, 0], entity: 1, kind: "minecraft:player".into(), distance: 10.0, damage: 7.0 };
assert_eq!(c.pos, [0, 64, 0]);
assert_eq!(c.damage, 7.0);
```

### `BlockBehavior`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

What one block of the mod does. Build it with the methods below, then give it to `BlockBehaviors::register`.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap());
assert_eq!(behavior.block().path(), "post");
```

### `BlockBehavior::new`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn new(block: Identifier) -> BlockBehavior`. A behaviour for `block` that changes nothing yet.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap());
assert_eq!(behavior.block().to_string(), "example:post");
```

### `BlockBehavior::can_survive`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn can_survive(self, f: impl Fn(&SurviveContext) -> bool) -> BlockBehavior`. Where the block may stay. Answer `false` and it is refused when placed, and breaks (with its drops) when a neighbour changes. The game asks often, so keep it short. Registered structure plant markers also ask this callback against their borrowed generation-region view; a registered `false` is final, while a missing callback retains native plant survival. The view has no lighting yet during FEATURES. The full native build and natural End-island fixture pass: 34 tall grass and 3 ferns survive alongside trees and vines. Registered allow/deny, unregistered fallback and unlocked callback dispatch have a pure regression. This does not validate every plant type or make lighting available during generation.

```rust
use pluto_lib::{BlockBehavior, Direction, Identifier};

// a sign that needs the block below
let behavior = BlockBehavior::new(Identifier::new("example", "sign").unwrap())
    .can_survive(|c| c.view.is_face_sturdy(Direction::Down.relative(c.pos), Direction::Up));
assert!(behavior.block().path() == "sign");
```

### `can_survive`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn can_survive(state: &BlockState, pos: [i32; 3], view: &dyn BlockView) -> Option<bool>`. Whether `state` may stay at `pos`, as the game's `BlockState.canSurvive` answers it — the same class-chain dispatch the game runs, over the view: a torch needs a sturdy face below, a wall torch the face it leans on, a crop the farmland below, and frosted ice survives anywhere. A mod-registered `BlockBehavior::can_survive` answers first, as the game's mod seam does. `None` where a needed fact is not published (the solid flag, the centre and rigid support bits, fire odds, the multiface attach table, fluid tags): a placement check must treat `None` as "cannot say", not as a refusal.

```rust
use pluto_lib::{BlockState, MemoryBlockView, PlacementFacts, can_survive};

let mut view = MemoryBlockView::default();
view.placement_facts.insert("minecraft:frosted_ice".to_string(), PlacementFacts {
    classes: vec!["FrostedIceBlock".to_string()],
    full_collision: true,
    solid_render: false,
});
assert_eq!(can_survive(&BlockState::parse("minecraft:frosted_ice").unwrap(), [0, 64, 0], &view), Some(true));
assert_eq!(can_survive(&BlockState::parse("minecraft:torch").unwrap(), [0, 64, 0], &view), None, "a torch's class facts are not in this view");
```

### `BlockBehavior::state_for_placement`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn state_for_placement(self, f: impl Fn(&PlacementContext) -> Placement) -> BlockBehavior`. The state the block takes when placed (see [Choosing the State When Placed](#choosing-the-state-when-placed)).

```rust
use pluto_lib::{BlockBehavior, Identifier, Placement};

// never placed by a sneaking player
let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap())
    .state_for_placement(|c| if c.sneaking { Placement::Refuse } else { Placement::Default });
assert!(behavior.block().path() == "post");
```

### `BlockBehavior::update_shape`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn update_shape(self, f: impl Fn(&ShapeUpdateContext) -> Option<BlockState>) -> BlockBehavior`. How a neighbour's change turns the block's state: `Some(state)` becomes the block's state (the same state too, which stops the game's own handling of that update), `None` leaves the update to the game. Asked at least six times for every block set, so keep it short.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap())
    .update_shape(|c| (c.state.property("powered") == Some("true")).then(|| c.state.clone().with("powered", "false")));
assert!(behavior.block().path() == "post");
```

### `BlockBehavior::is_slime_block`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn is_slime_block(self, f: impl Fn(&BlockState) -> bool + Send + Sync + 'static) -> BlockBehavior`. Registers the source slime-block identity predicate, separately from sticky connectivity. **Stub:** the pinned native piston/entity-push paths do not invoke it. Registration is not proof of slime push or bounce gameplay.

```rust
use pluto_lib::{BlockBehavior, Identifier};
let b = BlockBehavior::new(Identifier::parse("example:gel").unwrap()).is_slime_block(|_| true);
assert_eq!(b.block().path(), "gel");
```

### `BlockBehavior::is_sticky_block`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn is_sticky_block(self, f: impl Fn(&BlockState) -> bool + Send + Sync + 'static) -> BlockBehavior`. Registers whether a block participates in sticky branching. **Stub:** accepted but not invoked by the pinned native piston resolver. It is not a replacement for the complete resolver's movement limits or pair rules.

```rust
use pluto_lib::{BlockBehavior, Identifier};
let b = BlockBehavior::new(Identifier::parse("example:adhesive").unwrap()).is_sticky_block(|_| true);
assert_eq!(b.block().path(), "adhesive");
```

### `BlockBehavior::can_stick_to`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn can_stick_to(self, f: impl Fn(&BlockState, &BlockState) -> bool + Send + Sync + 'static) -> BlockBehavior`. Registers this block's directional predicate for its state and the neighbouring state. A false result must not be changed to native fallback; the reverse block's predicate and complete resolver still need separate handling. **Stub:** the pinned native resolver does not invoke this registration; the generic `Sim` method injection experiment is rejected. Pure callback tests are not native piston verification.

```rust
use pluto_lib::{BlockBehavior, Identifier};
let b = BlockBehavior::new(Identifier::parse("example:selective").unwrap())
    .can_stick_to(|state, other| state.block == other.block);
assert_eq!(b.block().path(), "selective");
```

### `BlockBehavior::entity_inside_motion`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn entity_inside_motion(self, f: impl Fn(&mut EntityInsideMotionContext) + Send + Sync + 'static) -> BlockBehavior`. Edits velocity at a block's swept contact. This is separate from the existing read-only `entity_inside` notification and from landing restitution. The handler is captured without holding the registry lock during invocation. The native player velocity/packet adapter passes the four-kind falling-side/suppression game gate on all four horizontal faces, with an identical-shape unregistered control on each face. Held-forward flat contact remains unresolved. `step_sound` forwards explicit volume/pitch through the native authoritative sound queue; the pinned build and four-kind native request gate pass, but audible playback and broader LAN delivery remain unverified.

```rust
use pluto_lib::{BlockBehavior, Identifier};
let behavior = BlockBehavior::new(Identifier::parse("example:pad").unwrap())
    .entity_inside_motion(|c| {
        if c.server && !c.suppressing_bounce { c.velocity[0] = 0.2; }
    });
assert_eq!(behavior.block().path(), "pad");
```

### `BlockBehavior::update_entity_after_fall_on`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn update_entity_after_fall_on(self, f: impl Fn(&mut AfterFallContext) + Send + Sync + 'static) -> BlockBehavior`. Owns the vertical collision response after a landing, separately from fall damage and side contacts. A block without this callback keeps the complete native response. The player adapter retains native horizontal response before invoking the captured callback; no native gravity/drag vertical correction follows. The item adapter is source-wired to the native resolved support position (including bounding-box edge landings), rather than recomputing the center block; absent contacts/callbacks retain native restitution. Its production-helper routing regression passes, but the changed item binding is not native-build or game-verified. Item fall-distance edits are not copied back. Historical resident-player bounce evidence does not certify the current combined bundle; exact per-tick velocity parity and LAN remain unverified.

```rust
use pluto_lib::{BlockBehavior, Identifier};
let behavior = BlockBehavior::new(Identifier::parse("example:cushion").unwrap())
    .update_entity_after_fall_on(|c| {
        c.velocity[1] = if !c.suppressing_bounce && c.velocity[1] < 0.0 { -c.velocity[1] } else { 0.0 };
        c.fall_distance = 0.0;
    });
assert_eq!(behavior.block().path(), "cushion");
```

### `BlockBehavior::fall_on`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn fall_on(self, f: impl Fn(&mut FallOnContext) + Send + Sync + 'static) -> BlockBehavior`. Changes calculated landing damage through `context.damage`. The game-verified native adapter handles players and preserves native listeners before this callback; the concrete mob adapter also passes the native build and four-kind cow immunity/bounce plus stone damage control (other species unverified). Invalid values are ignored; negative damage becomes zero. The pinned build and player zero-damage/stone-control gate pass; other entities and LAN remain unverified. This is not an after-fall bounce callback.

```rust
use pluto_lib::{BlockBehavior, Identifier};
let behavior = BlockBehavior::new(Identifier::parse("example:cushion").unwrap())
    .fall_on(|c| c.damage *= 0.5);
assert_eq!(behavior.block().path(), "cushion");
```

### `BlockBehavior::entity_inside`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn entity_inside(self, f: impl Fn(&EntityInsideContext)) -> BlockBehavior`. An entity is inside the block's space. The player, mob and item movement seams invoke the callbacks on the server side, with the level's dimension, the block's full state, the entity's id and kind, its bounding box and its fall distance (see the support matrix for what is not covered).

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "thorns").unwrap())
    .entity_inside(|c| println!("{} touched the thorns at {:?}", c.kind, c.pos));
assert!(behavior.block().path() == "thorns");
```

### `BlockBehavior::random_tick`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn random_tick(self, f: impl Fn(&mut TickContext)) -> BlockBehavior`. A random tick reached the block. Having this callback stops the game's own random tick of the block.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "moss").unwrap()).random_tick(|c| {
    let above = [c.pos[0], c.pos[1] + 1, c.pos[2]];
    if c.world.is_air(above) && c.world.random_int(6) == 0 {
        c.world.set_block_state(above, &c.state.clone());
    }
});
assert_eq!(behavior.block().path(), "moss");
```

### `BlockBehavior::scheduled_tick`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn scheduled_tick(self, f: impl Fn(&mut TickContext)) -> BlockBehavior`. A tick the block scheduled with `schedule_tick` came due. Having this callback stops the game's own tick of the block.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "fuse").unwrap()).scheduled_tick(|c| {
    c.world.remove_block(c.pos);
});
assert_eq!(behavior.block().path(), "fuse");
```

### `BlockBehavior::can_bonemeal`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn can_bonemeal(self, f: impl Fn(&BonemealContext) -> bool) -> BlockBehavior`. Whether a bone meal can be used on the block; a used meal is spent only on yes.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "sapling").unwrap())
    .can_bonemeal(|c| c.view.is_air([c.pos[0], c.pos[1] + 1, c.pos[2]]));
assert_eq!(behavior.block().path(), "sapling");
```

### `BlockBehavior::bonemeal_success`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn bonemeal_success(self, f: impl Fn(&mut TickContext) -> bool) -> BlockBehavior`. Whether a bone meal that was used makes the block grow this time. Without it the game's own chance applies.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "sapling").unwrap()).bonemeal_success(|c| c.world.random_int(100) < 45);
assert_eq!(behavior.block().path(), "sapling");
```

### `BlockBehavior::on_bonemeal`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn on_bonemeal(self, f: impl Fn(&mut TickContext)) -> BlockBehavior`. The block grows from a bone meal.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "sapling").unwrap()).on_bonemeal(|c| {
    c.world.set_block_state(c.pos, &c.state.clone().with("stage", "1"));
});
assert_eq!(behavior.block().path(), "sapling");
```

### `BlockBehavior::may_place_on`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn may_place_on(self, f: impl Fn(&PlaceOnContext) -> bool) -> BlockBehavior`. What a plant may stand on. The block's template has to be a plant that asks, such as a crop or a sapling.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "fern").unwrap()).may_place_on(|c| c.ground.block.path() == "dirt");
assert_eq!(behavior.block().path(), "fern");
```

### `vegetation_may_place_on`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn vegetation_may_place_on(state: &BlockState, ground: &BlockState, ground_pos: [i32; 3], view: &dyn BlockView) -> Option<bool>`. Whether a plant of `state` may stand on `ground`, as the game's support tags answer it — the `minecraft:supports_*` tags on the ground block, such as `supports_crops` for farmland under a crop. A mod-registered `BlockBehavior::may_place_on` answers first. `None` where the ground block carries none of the support tags the plant asks for, or where the plant's support tag is one the view cannot answer (fluid tags such as `supports_frogspawn`).

```rust
use pluto_lib::{BlockState, MemoryBlockView, PlacementFacts, vegetation_may_place_on};

let mut view = MemoryBlockView::default();
view.placement_facts.insert("minecraft:wheat".to_string(), PlacementFacts {
    classes: vec!["CropBlock".to_string()],
    full_collision: false,
    solid_render: false,
});
view.state_tags.insert("minecraft:farmland".to_string(), ["minecraft:supports_crops".to_string()].into_iter().collect());
let wheat = BlockState::parse("minecraft:wheat").unwrap();
let farmland = BlockState::parse("minecraft:farmland").unwrap();
assert_eq!(vegetation_may_place_on(&wheat, &farmland, [0, 63, 0], &view), Some(true));
```

### `PickContext`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

A player picks the block. `state` is the block's state and `pos` its position.

```rust
use pluto_lib::{BlockState, PickContext};

let pick = PickContext { state: BlockState::parse("example:post[lit=true]").unwrap(), pos: [0, 64, 0] };
assert_eq!(pick.pos, [0, 64, 0]);
assert_eq!(pick.state.property("lit"), Some("true"));
```

### `BlockBehavior::pick_stack`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn pick_stack(self, f: impl Fn(&PickContext) -> Option<ItemStack> + Send + Sync + 'static) -> BlockBehavior`. The stack a player gets when picking the block (middle click in creative; Fabric: `Block.getPickStack`). Answer `Some(stack)`, with data components if you like, or `None` for the game's own choice.

```rust
use pluto_lib::{BlockBehavior, Identifier, ItemStack};

let behavior = BlockBehavior::new(Identifier::new("example", "lamp").unwrap())
    .pick_stack(|pick| (pick.state.property("lit") == Some("true")).then(|| ItemStack::new(Identifier::new("example", "lit_lamp").unwrap(), 1)));
assert_eq!(behavior.block().path(), "lamp");
```

### `BlockBehavior::flammable`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn flammable(self, ignite: i32, burn: i32) -> BlockBehavior`. The block's fire odds (Minecraft: `FireBlock.setFlammable`): how readily fire spreads onto the block (ignite) and how strongly it feeds the fire (burn). Vanilla's planks are 5/20, wool 30/60, leaves 30/60; `(0, 0)` never burns. A block without odds keeps the ones its declaration's template carries. The game asks hundreds of times per fire tick, so the answer is a plain registry lookup.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "leaves").unwrap()).flammable(30, 60);
assert_eq!(behavior.flammable_odds(), Some((30, 60)));
```

### `BlockBehavior::flammable_odds`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn flammable_odds(&self) -> Option<(i32, i32)>`. The fire odds given with `flammable`, if any.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap());
assert_eq!(behavior.flammable_odds(), None);
```

### `BlockBehavior::block`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn block(&self) -> &Identifier`. The block this behaviour belongs to.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap());
assert_eq!(behavior.block().namespace(), "example");
```

### `BlockBehaviors`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

The registry of block behaviours.

```rust
use pluto_lib::{BlockBehavior, BlockBehaviors, Identifier};

BlockBehaviors::register(BlockBehavior::new(Identifier::new("example", "registry_page").unwrap()));
assert!(BlockBehaviors::get("example:registry_page").is_some());
```

### `BlockBehaviors::register`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn register(behavior: BlockBehavior)`. Gives `behavior.block()` its behaviour. Registering the same block again replaces the earlier behaviour.

```rust
use pluto_lib::{BlockBehavior, BlockBehaviors, Identifier};

let id = Identifier::new("example", "twice").unwrap();
BlockBehaviors::register(BlockBehavior::new(id.clone()).can_survive(|_| false));
BlockBehaviors::register(BlockBehavior::new(id).can_survive(|_| true));
assert_eq!(BlockBehaviors::registered().iter().filter(|b| b.block().to_string() == "example:twice").count(), 1);
```

### `BlockBehaviors::get`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn get(block: &str) -> Option<BlockBehavior>`. The behaviour registered for `block` (an id such as `tconstruct:punji`), if any.

```rust
use pluto_lib::BlockBehaviors;

assert!(BlockBehaviors::get("example:never_registered").is_none());
```

### `BlockBehaviors::registered`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn registered() -> Vec<BlockBehavior>`. Every block that has a behaviour.

```rust
use pluto_lib::{BlockBehavior, BlockBehaviors, Identifier};

BlockBehaviors::register(BlockBehavior::new(Identifier::new("example", "listed").unwrap()));
assert!(BlockBehaviors::registered().iter().any(|b| b.block().to_string() == "example:listed"));
```

### `EntityInsideContext::act`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

`fn act(&self, action: EntityAction)`. Queues something to do to the entity inside (see `EntityAction`): the action is added to the context's `actions` and lands once the movement seams are done with the entity, since the seams run inside entity movement, where the entity may not be changed.

```rust
use pluto_lib::{ActionTarget, BlockState, EntityAction, EntityInsideContext};

let context = EntityInsideContext {
    dimension: "minecraft:overworld".into(),
    state: BlockState::parse("example:punji").unwrap(),
    pos: [0, 64, 0],
    entity: 12,
    kind: "minecraft:zombie".into(),
    bounds: [0.2, 64.0, 0.2, 0.8, 65.9, 0.8],
    fall_distance: 0.0,
    actions: Default::default(),
};
context.act(EntityAction::Hurt { target: ActionTarget::Victim, amount: 1.0, damage_type: "minecraft:generic".into() });
let queued = context.actions.borrow();
assert_eq!(queued.len(), 1);
assert_eq!(queued[0], EntityAction::Hurt { target: ActionTarget::Victim, amount: 1.0, damage_type: "minecraft:generic".into() });
```

### `ServerBlockEvents::BLOCK_PLACE`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`static BLOCK_PLACE: &'static Event<BlockPlace, ()>`. A block is about to be placed by a player or a walk module (NeoForge: `BlockEvent.EntityPlaceEvent`, the Forge `onBlockPlace`): set `cancel` to stop the placement. Heard before the block lands, so a later callback may still cancel it. `block_place_heard()` says whether any callback hears it, so a placer can skip firing when nobody listens. The walk hook fires it before a frost walker replacement lands. There are no listeners in the tree yet: protection and claim mods are the callers this surface is for.

```rust
use pluto_lib::{BlockPlace, BlockState, ServerBlockEvents, block_place};

ServerBlockEvents::BLOCK_PLACE.register(|place| {
    if place.state.block.to_string() == "minecraft:frosted_ice" {
        place.cancel.set(true);
    }
});
let place = BlockPlace::new(Some(42), "minecraft:overworld", [0, 64, 0],
    BlockState::parse("minecraft:frosted_ice").unwrap(),
    BlockState::parse("minecraft:water[level=0]").unwrap());
assert!(block_place(&place));
```

### `BlockPlace`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

A block about to be placed, heard through `ServerBlockEvents::BLOCK_PLACE` before the block lands: `placer` is the placer's uuid when a player places it (None for a placement no player owns), `dimension` the dimension, `pos` where the block would go, `state` the state that would be placed, `replaced` the state it replaces, and `cancel`, set to true to stop the placement.

```rust
use pluto_lib::{BlockPlace, BlockState};

let place = BlockPlace::new(Some(42), "minecraft:overworld", [0, 64, 0],
    BlockState::parse("minecraft:stone").unwrap(),
    BlockState::parse("minecraft:water[level=0]").unwrap());
assert!(!place.cancel.get());
place.cancel.set(true);
assert!(place.cancel.get());
```

### `BlockPlace::new`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn new(placer: Option<u128>, dimension: &str, pos: BlockPos, state: BlockState, replaced: BlockState) -> BlockPlace`. A placement of `state` at `pos` in `dimension`, replacing `replaced`, by `placer`; `cancel` starts false.

```rust
use pluto_lib::{BlockPlace, BlockState};

let place = BlockPlace::new(None, "minecraft:overworld", [1, 64, 1],
    BlockState::parse("minecraft:stone").unwrap(),
    BlockState::parse("minecraft:water[level=0]").unwrap());
assert_eq!(place.dimension, "minecraft:overworld");
assert_eq!(place.pos, [1, 64, 1]);
assert!(place.placer.is_none());
```

### `ServerBlockEvents`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

The block events a server hears. Its one event today is `BLOCK_PLACE`, a block about to be placed by a player or a walk module.

```rust
use pluto_lib::ServerBlockEvents;

ServerBlockEvents::BLOCK_PLACE.register(|place| {
    if place.state.block.to_string() == "minecraft:bedrock" {
        place.cancel.set(true);
    }
});
```

### `block_place`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn block_place(place: &BlockPlace) -> bool`. Fires `ServerBlockEvents::BLOCK_PLACE` for `place`: whether any callback cancelled it.

```rust
use pluto_lib::{BlockPlace, BlockState, block_place};

let place = BlockPlace::new(Some(42), "minecraft:overworld", [0, 64, 0],
    BlockState::parse("minecraft:stone").unwrap(),
    BlockState::parse("minecraft:water[level=0]").unwrap());
assert!(!block_place(&place));
```

### `block_place_heard`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn block_place_heard() -> bool`. Whether any callback hears `ServerBlockEvents::BLOCK_PLACE`, so a placer can skip building a `BlockPlace` when nobody listens.

```rust
use pluto_lib::{ServerBlockEvents, block_place_heard};

assert!(!block_place_heard());
ServerBlockEvents::BLOCK_PLACE.register(|_| {});
assert!(block_place_heard());
```
