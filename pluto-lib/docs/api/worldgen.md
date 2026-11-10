# Worldgen: feature types and configuration

A feature type supplies a codec that validates data, then a generator that uses the supplied world and origin. This follows Minecraft/Fabric's registered `Feature` and `ConfiguredFeature` concepts. Rust closures capture the decoded settings; random draws use the supplied `BlockEditor` rather than a newly seeded RNG.

> **Use with Caution.** Direct codec/generator calls have pure tests. Live `BlockEditor::place_feature` dispatch has been checked with an independent real-game consumer: configured state/properties, negative origin, random bounds, invalid configuration, bounded recursion and no native fallthrough after registered failure. Native `/place feature` commands, natural generation workers and custom types referenced inside native feature graphs are not connected. Automatic host dispatch currently runs through PlutoLib's live `BlockEditor::place_feature` and the typed explicit structure-marker world. Registering a type is not evidence that those paths invoke it; see the support matrix. Per-position root overrides have a registry-bound explicit API (native/game fixtures checked), not automatic custom root-type decoding. Decorator codecs can be invoked explicitly by a mod's feature generator. `BlockEditor::place_tree` appends predecoded decorators inside native tree placement (live oak fixture checked); ordinary native tree JSON does not decode registered decorator types.

## Basic Example

Register a feature type whose configuration selects a small glass column:

```rust
use pluto_lib::{BlockState, ConfiguredFeature, FeatureTypeRegistry, Identifier};

assert!(FeatureTypeRegistry::register(Identifier::parse("example:glass_column").unwrap(), |configuration| {
    let height = configuration.get("height").and_then(serde_json::Value::as_u64)
        .filter(|height| (1..=8).contains(height))
        .ok_or_else(|| "height must be 1..=8".to_owned())?;
    let glass = BlockState::parse("minecraft:glass").unwrap();
    Ok(ConfiguredFeature::new(move |context| {
        for offset in 0..height as i32 {
            let Some(y) = context.origin[1].checked_add(offset) else { return false };
            if !context.world.set_block_state([context.origin[0], y, context.origin[2]], &glass) {
                return false;
            }
        }
        true
    }))
}));
```

The codec must reject invalid configuration before touching the world. A generator returning false reports a handled placement failure; it does not roll back earlier successful writes or request a native fallback.

## Live Placement and Configuration

The live adapter reads the configured feature from the active native catalog. Its `type` selects the registered codec, and its `config` value is passed unchanged (missing `config` is JSON null, which the codec may accept or reject). An unregistered type delegates to the existing native placement. A registered type's codec error or false result does not run a second algorithm. Codecs are currently decoded per request, not cached across resource reloads.

To test a codec without a game, pass the same configuration value directly:

```rust
use pluto_lib::{ConfiguredFeature, FeatureContext, FeatureTypeRegistry, Identifier, MemoryBlockView};

let kind = Identifier::parse("example:validate_first").unwrap();
FeatureTypeRegistry::register(kind.clone(), |configuration| {
    if configuration.get("enabled").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err("enabled must be true".into());
    }
    Ok(ConfiguredFeature::new(|context| context.world.is_air(context.origin)))
});
let mut world = MemoryBlockView::default();
let mut context = FeatureContext { world: &mut world, origin: [-4, 70, 9] };
assert!(FeatureTypeRegistry::generate(&kind, &serde_json::json!({"enabled": false}), &mut context).is_err());
assert_eq!(FeatureTypeRegistry::generate(&kind, &serde_json::json!({"enabled": true}), &mut context), Ok(Some(true)));
```

> **Use with Caution.** This direct call exercises the registry, not a renderer, game world or generation worker. Live placement uses the current callback's world, updates and RNG. Recursive live custom-feature requests are bounded at 64; recursion bookkeeping is restored on unwind before rethrowing the original panic. Prior world writes are not rolled back. No borrowed world/context is retained in globals or TLS. Register during initialization, before placement begins. Callbacks execute without registry locks; replacement takes effect on subsequent lookups.

## Tree Decorators

Register a decorator codec, decode it before the tree writes anything, then invoke the returned `TreeDecorator` inside that tree's generation algorithm. Supply the same editor/RNG and the original ordered position lists. The registry neither discovers a canopy nor connects native tree generation automatically.

```rust
use pluto_lib::{Identifier, TreeDecorator, TreeDecoratorTypeRegistry};

TreeDecoratorTypeRegistry::register(Identifier::parse("example:check_leaves").unwrap(), |value| {
    if !value.is_null() { return Err("expected null".into()); }
    Ok(TreeDecorator::new(|context| {
        if context.leaves.is_empty() { return Err("no leaves".into()); }
        Ok(())
    }))
});
let decorator = TreeDecoratorTypeRegistry::decode(&Identifier::parse("example:check_leaves").unwrap(), &serde_json::Value::Null)
    .unwrap().unwrap();
let _snapshot = decorator.clone();
```

> **Use with Caution.** This is an explicit-call API. `BlockEditor::place_tree` appends predecoded decorators before native tracking/finalization (live oak fixture checked), but does not add registered types to ordinary native tree JSON. Other generators remain responsible for tracking/finalization; natural-generation dispatch is unconnected. Failed decorators preserve prior writes; their error is not permission to fall back to a second algorithm. Position lists are not sorted or deduplicated here. An independent live feature fixture checked Tinkers' registered leaf-vine loop: vanilla/configured vine blocks, attachment faces, five-block limit, obstruction, zero probability, invalid configuration and write-error ownership; its five images were inspected. Tinkers' leaf-vine codec syntax-checks its block ID; block/property existence is resolved on write, unlike Java's registry-bound decode. Its full registry/parity integration is unfinished.

## Root Placement Overrides

`RootPlacement` overrides one root write while leaving the native root simulator, trunk offset and tree geometry intact. Return `Some(state)` for a replacement, `None` for native parent placement, or `Err` to stop without fallback. The native adapter applies waterlogging and records the root. `BlockEditor::place_tree_with_roots` accepts the decoded override alongside extra decorators.

Holder sets and state providers are kept as ordered native-codec descriptors in `WorldgenBindings`. The host binds every descriptor against the active registry before any tree writes, including unselected providers. During a callback, test a bound block set or sample exactly the selected provider through `RootPlacementWorld`, using the same tree RNG. Do not flatten weighted/noise/rule-based providers into a fixed state map or try another provider after a sample failure.

```rust
use pluto_lib::{RootPlacement, WorldgenBindings};

let roots = RootPlacement::new(WorldgenBindings {
    block_sets: vec![serde_json::json!(["minecraft:water"])],
    state_providers: vec![serde_json::json!({"type":"minecraft:simple", "state":{"id":"minecraft:mangrove_roots"}})],
}, |context| {
    if context.world.matches_block_set(0, context.pos)? {
        context.world.sample_state_provider(0, context.pos).map(Some)
    } else {
        Ok(None)
    }
});
assert_eq!(roots.bindings().state_providers.len(), 1);
```

> **Use with Caution.** The native build and five live root fixtures pass: first-match weighted providers, waterlogging, native muddy-root fallback, selected-provider no-state failure and invalid unselected bindings; all five images were inspected. This is not full Ender Tree parity. This is a per-position override, not a replacement for native root simulation. Ordinary native JSON/natural generation does not resolve registered custom root types. Descriptors use the pinned host's codec schema; older Java data needs schema conversion, not a silent reinterpretation. Holder/provider binding counts are limited to 256 each, with the native provider node budget. Memory editors report binding queries as unsupported; custom editors reject tree placement unless overridden. Nested feature/tree placement inside root callbacks is also rejected without writes until nested tracking exists. Successful earlier writes are never rolled back after a callback/provider error.

## Structure Planning

A registered `StructureType` codec decodes immutable settings into `ConfiguredStructure`. Its planner receives a borrowed `StructureContext` with the attempt's chunk, height/biome queries and existing random stream. It returns an owned `StructureStart` and piece descriptors; this phase does not write blocks. Keep the biome-check origin separate from individual template anchors and reject the biome before piece-building draws where the original algorithm requires that order.

```rust
use pluto_lib::{ConfiguredStructure, Identifier, StructureTypeRegistry};
let id = Identifier::parse("example:disabled_structure").unwrap();
assert!(StructureTypeRegistry::register(id.clone(), |value| {
    if !value.is_null() { return Err("expected null settings".into()); }
    Ok(ConfiguredStructure::new(|_| Ok(None)))
}));
assert!(StructureTypeRegistry::decode(&id, &serde_json::Value::Null).unwrap().is_some());
```

> **Use with Caution.** Explicit native structure placement and marker-free template piece decoding/placement pass the complete native facade build and eleven real-game cases (all images inspected): rotation/mirror geometry and stair properties, explicit air, rejected/error/invalid planners, pre-write piece validation, marker/entity rejection and preserved built-in placement. Registered template/marker codecs also pass six real-game cases: cross-chunk mutable state, transforms, callback error ownership, invalid state before any piece writes and production IslandPiece tree/grass/vine dispatch. Natural generation is unconnected; saved-piece restoration is implemented but not yet exercised through persisted starts. Unknown registry types return `Ok(None)`; a decoded planner returning `Ok(None)` rejects its candidate, and errors remain errors, never native fallback. No registration or successful plan proves that an island or any other structure was generated. A non-cancellable constructor callback captures owned source data under a weak resource identity rather than reading source files or retaining borrowed contexts. Different source data sharing the same resource identity is ambiguous and custom placement rejects; existing owned snapshots are not overwritten. Shared-resource reload/concurrent-world parity is therefore not complete. Built-in native structure kinds are not overridden by this adapter; only registered custom types decoded by the host as unsupported are dispatched. It binds every piece before publishing a start. Native candidate weighting/cache policy for natural starts remains unfinished.

## Reference

### `FeatureContext`

`FeatureContext<'a> { world: &'a mut dyn BlockEditor, origin: BlockPos }`. A borrowed placement context. Read/write blocks and draw random values through `world`; `origin` is the supplied world coordinate. Unlike Java's separate world/random arguments, this API keeps the random operations on the same editor.

```rust
use pluto_lib::{FeatureContext, MemoryBlockView};

let mut world = MemoryBlockView::default();
world.rolls = vec![7];
let mut context = FeatureContext { world: &mut world, origin: [-2, 60, 3] };
assert_eq!(context.world.random_int(5), 2);
assert!(context.world.is_air(context.origin));
```

### `ConfiguredFeature`

An immutable decoded configuration bound to its algorithm. Cloning shares the thread-safe generator, not a borrowed placement context. Hold validated settings in its closure.

```rust
use pluto_lib::ConfiguredFeature;

let configured = ConfiguredFeature::new(|_| false);
let _same_algorithm = configured.clone();
```

### `ConfiguredFeature::new`

`fn new(generate: impl Fn(&mut FeatureContext<'_>) -> bool + Send + Sync + 'static) -> ConfiguredFeature`. Bind settings to generation. Returning false preserves already completed writes; no rollback is implied. The closure must not retain its borrowed context.

```rust
use pluto_lib::ConfiguredFeature;

let maximum_y = 120;
let _configured = ConfiguredFeature::new(move |context| context.origin[1] < maximum_y);
```

### `ConfiguredFeature::generate`

`fn generate(&self, context: &mut FeatureContext<'_>) -> bool`. Run a decoded algorithm against the caller's context. This direct method neither registers a type nor decodes configuration.

```rust
use pluto_lib::{ConfiguredFeature, FeatureContext, MemoryBlockView};

let configured = ConfiguredFeature::new(|context| context.world.is_air(context.origin));
let mut world = MemoryBlockView::default();
assert!(configured.generate(&mut FeatureContext { world: &mut world, origin: [0, 64, 0] }));
```

### `FeatureTypeRegistry`

A namespaced registry of feature codecs. A later registration replaces only its own type. A lookup clones the factory handle before invoking mod code, so both codecs and generators may register other types without retaining a registry lock.

```rust
use pluto_lib::{ConfiguredFeature, FeatureTypeRegistry, Identifier};

assert!(FeatureTypeRegistry::register(Identifier::parse("example:empty_patch").unwrap(), |_| {
    Ok(ConfiguredFeature::new(|_| false))
}));
```

### `FeatureTypeRegistry::register`

`fn register(id: Identifier, codec: impl Fn(&serde_json::Value) -> Result<ConfiguredFeature, String> + Send + Sync + 'static) -> bool`. Register or replace the type. The codec validates the configuration and returns an algorithm or diagnostic. False means the registry could not be written. Replaced captured values are destroyed outside the write lock. A registration does not schedule placement or remesh a chunk.

```rust
use pluto_lib::{ConfiguredFeature, FeatureTypeRegistry, Identifier};

assert!(FeatureTypeRegistry::register(Identifier::parse("example:unit_feature").unwrap(), |value| {
    if !value.is_null() { return Err("expected null".into()); }
    Ok(ConfiguredFeature::new(|_| true))
}));
```

### `FeatureTypeRegistry::generate`

`fn generate(id: &Identifier, configuration: &serde_json::Value, context: &mut FeatureContext<'_>) -> Result<Option<bool>, String>`. Decode and execute. `Ok(None)` means the type is not registered, `Ok(Some(true/false))` is the registered generator's result, and `Err` reports a codec/registry failure. A codec error occurs before generation/world writes. Neither false nor an error permits native fallback for that registered type.

```rust
use pluto_lib::{FeatureContext, FeatureTypeRegistry, Identifier, MemoryBlockView};

let mut world = MemoryBlockView::default();
let mut context = FeatureContext { world: &mut world, origin: [0, 64, 0] };
let missing = Identifier::parse("example:not_registered").unwrap();
assert_eq!(FeatureTypeRegistry::generate(&missing, &serde_json::Value::Null, &mut context), Ok(None));
```

### `TreeDecoration`

`enum TreeDecoration { Native(usize), Custom(TreeDecorator) }`. One step for `BlockEditor::place_tree_with_plan`. `Native(index)` refers to the zero-based index in the native configuration's `decorators` array. Each native decorator must appear exactly once; omitted, repeated or out-of-range native indices reject placement before tree writes. `Custom` holds an already decoded algorithm. Both variants clone without world access. A plan may contain at most 512 steps, including at most 256 native decorators.

```rust
use pluto_lib::{TreeDecoration, TreeDecorator};
let plan = [
    TreeDecoration::Custom(TreeDecorator::new(|_| Ok(()))),
    TreeDecoration::Native(0),
    TreeDecoration::Custom(TreeDecorator::new(|_| Ok(()))),
];
assert!(matches!(plan[1].clone(), TreeDecoration::Native(0)));
```

> **Use with Caution.** Three live fixtures checked interleaved custom/native writes, stopping after a custom error and rejecting an invalid native-index plan before callbacks/writes; all three images were inspected. These steps do not decode custom decorators inside ordinary native JSON or connect natural generation. Existing `place_tree` and `place_tree_with_roots` retain append-only behaviour.

### `TreeDecoratorContext`

`TreeDecoratorContext<'a> { world: &'a mut dyn BlockEditor, logs: &'a [BlockPos], leaves: &'a [BlockPos], roots: &'a [BlockPos] }`. Borrowed tree positions and the generator's editor/RNG; the context cannot be retained by a decorator. The caller provides the required iteration order, including stable ordering of equal-height positions where its algorithm requires it.

```rust
use pluto_lib::{MemoryBlockView, TreeDecoratorContext};
let mut world = MemoryBlockView::default();
let context = TreeDecoratorContext { world: &mut world, logs: &[[0, 64, 0]], leaves: &[[0, 65, 0]], roots: &[] };
assert_eq!(context.leaves[0], [0, 65, 0]);
```

### `TreeDecorator`

An immutable decoded algorithm. Cloning shares its thread-safe captured settings, never a borrowed world. Decode every decorator before starting a tree's writes.

```rust
use pluto_lib::TreeDecorator;
let decorator = TreeDecorator::new(|_| Ok(()));
let _snapshot = decorator.clone();
```

### `TreeDecorator::new`

`fn new(decorate: impl Fn(&mut TreeDecoratorContext<'_>) -> Result<(), String> + Send + Sync + 'static) -> TreeDecorator`. Capture validated settings. Return `Err` for failed reads/writes; no rollback or fallback is implied. Use explicit update flags when the algorithm needs them.

```rust
use pluto_lib::{BlockState, TreeDecorator};
let marker = BlockState::parse("minecraft:stone").unwrap();
let _decorator = TreeDecorator::new(move |context| {
    for &pos in context.roots {
        if !context.world.set_block_state_with_flags(pos, &marker, 19) {
            return Err("root marker write failed".into());
        }
    }
    Ok(())
});
```

### `TreeDecorator::decorate`

`fn decorate(&self, context: &mut TreeDecoratorContext<'_>) -> Result<(), String>`. Run against the supplied lists without decoding, registering or rearranging them. Successful prior writes remain after an error.

```rust
use pluto_lib::{MemoryBlockView, TreeDecorator, TreeDecoratorContext};
let decorator = TreeDecorator::new(|_| Err("declined".into()));
let mut world = MemoryBlockView::default();
assert_eq!(decorator.decorate(&mut TreeDecoratorContext { world: &mut world, logs: &[], leaves: &[], roots: &[] }), Err("declined".into()));
```

### `TreeDecoratorTypeRegistry`

Namespaced codec registry for explicit tree-generator calls. Later registration replaces the codec for that ID; already decoded decorators retain their previous algorithm. Callbacks and replaced capture destructors run outside registry locks.

```rust
use pluto_lib::{Identifier, TreeDecoratorTypeRegistry};
assert!(TreeDecoratorTypeRegistry::decode(&Identifier::parse("example:missing_decorator").unwrap(), &serde_json::Value::Null).unwrap().is_none());
```

### `TreeDecoratorTypeRegistry::register`

`fn register(id: Identifier, codec: impl Fn(&serde_json::Value) -> Result<TreeDecorator, String> + Send + Sync + 'static) -> bool`. Register or replace a codec; false means the registry could not be written. Codecs validate configuration without any world access.

```rust
use pluto_lib::{Identifier, TreeDecorator, TreeDecoratorTypeRegistry};
assert!(TreeDecoratorTypeRegistry::register(Identifier::parse("example:noop_decorator").unwrap(), |_| Ok(TreeDecorator::new(|_| Ok(())))));
```

### `TreeDecoratorTypeRegistry::decode`

`fn decode(id: &Identifier, configuration: &serde_json::Value) -> Result<Option<TreeDecorator>, String>`. Return `Ok(None)` for unknown types, a decoded decorator for registered valid configuration, or `Err` for a codec/registry failure. A registered type's error must not silently select another algorithm.

```rust
use pluto_lib::{Identifier, TreeDecoratorTypeRegistry};
let id = Identifier::parse("example:invalid_decorator").unwrap();
TreeDecoratorTypeRegistry::register(id.clone(), |_| Err("bad configuration".into()));
assert_eq!(TreeDecoratorTypeRegistry::decode(&id, &serde_json::Value::Null).err().as_deref(), Some("bad configuration"));
```

### `WorldgenBindings`

`WorldgenBindings { block_sets: Vec<serde_json::Value>, state_providers: Vec<serde_json::Value> }`. Ordered immutable configuration inputs once captured by a root override. The lists are independent; their indices are local handles, never engine state IDs. `Default` makes empty lists. Cloning preserves full descriptors, not sampled states.

```rust
use pluto_lib::WorldgenBindings;
let mut bindings = WorldgenBindings::default();
bindings.block_sets.push(serde_json::json!("#minecraft:logs"));
let copy = bindings.clone();
assert_eq!(copy.block_sets, bindings.block_sets);
```

### `RootPlacementWorld`

Extends `BlockEditor` with `fn matches_block_set(&mut self, index: usize, pos: BlockPos) -> Result<bool, String>` and `fn sample_state_provider(&mut self, index: usize, pos: BlockPos) -> Result<BlockState, String>`. Use indices from the override's binding lists. Missing bindings/positions and provider failures are errors, not false matches or native fallback. A selected provider producing no state is also an error; it must not silently reuse the current block. The trait defaults and `MemoryBlockView` report unsupported bindings without writing or sampling.

```rust
use pluto_lib::{MemoryBlockView, RootPlacementWorld};
let mut world = MemoryBlockView::default();
assert!(world.matches_block_set(0, [0, 64, 0]).is_err());
assert!(world.sample_state_provider(0, [0, 64, 0]).is_err());
assert!(world.writes.is_empty());
```

### `RootPlacementContext`

`RootPlacementContext<'a> { world: &'a mut dyn RootPlacementWorld, pos: BlockPos }`. A borrowed editor plus the current root position. No world/RNG borrow may be retained beyond the callback.

```rust
use pluto_lib::{MemoryBlockView, RootPlacementContext};
let mut world = MemoryBlockView::default();
let context = RootPlacementContext { world: &mut world, pos: [-4, 64, 2] };
assert_eq!(context.pos, [-4, 64, 2]);
```

### `RootPlacement`

A decoded per-position root-write override plus binding descriptors. Cloning shares its immutable algorithm; native parent shape/simulation is not copied into this value.

```rust
use pluto_lib::{RootPlacement, WorldgenBindings};
let roots = RootPlacement::new(WorldgenBindings::default(), |_| Ok(None));
let _copy = roots.clone();
```

### `RootPlacement::new`

`fn new(bindings: WorldgenBindings, place: impl Fn(&mut RootPlacementContext<'_>) -> Result<Option<BlockState>, String> + Send + Sync + 'static) -> RootPlacement`. Capture descriptors/settings. The host compiles all bindings before tree writes. The callback returns a replacement, explicit parent fallback, or an error; an error is never permission to run the parent instead.

```rust
use pluto_lib::{BlockState, RootPlacement, WorldgenBindings};
let state = BlockState::parse("minecraft:mangrove_roots").unwrap();
let _roots = RootPlacement::new(WorldgenBindings::default(), move |_| Ok(Some(state.clone())));
```

### `RootPlacement::bindings`

`fn bindings(&self) -> &WorldgenBindings`. Read the original ordered descriptors; this does not bind registries, sample providers, access a world or draw random values.

```rust
use pluto_lib::{RootPlacement, WorldgenBindings};
let roots = RootPlacement::new(WorldgenBindings::default(), |_| Ok(None));
assert!(roots.bindings().block_sets.is_empty());
```

### `RootPlacement::place`

`fn place(&self, context: &mut RootPlacementContext<'_>) -> Result<Option<BlockState>, String>`. Execute one position override. Direct calls do not bind the context's providers or perform native parent placement/waterlogging/tracking; the caller must handle the returned decision.

```rust
use pluto_lib::{MemoryBlockView, RootPlacement, RootPlacementContext, WorldgenBindings};
let roots = RootPlacement::new(WorldgenBindings::default(), |_| Ok(None));
let mut world = MemoryBlockView::default();
assert_eq!(roots.place(&mut RootPlacementContext { world: &mut world, pos: [0, 64, 0] }), Ok(None));
assert!(world.writes.is_empty());
```

### `RootPlacementTypeRegistry`

Namespaced per-position root-override codecs. Existing native root simulation remains in the supplied base tree configuration. Registration alone does not alter native root-type decoding or natural generation. Replaced captures and user callbacks execute outside locks.

```rust
use pluto_lib::{Identifier, RootPlacementTypeRegistry};
assert!(RootPlacementTypeRegistry::decode(&Identifier::parse("example:missing_root_override").unwrap(), &serde_json::Value::Null).unwrap().is_none());
```

### `RootPlacementTypeRegistry::register`

`fn register(id: Identifier, codec: impl Fn(&serde_json::Value) -> Result<RootPlacement, String> + Send + Sync + 'static) -> bool`. Register or replace a codec; false means the registry could not be written. The codec validates configuration structure without world writes. Host registry-binding errors are checked separately before native generation.

```rust
use pluto_lib::{Identifier, RootPlacement, RootPlacementTypeRegistry, WorldgenBindings};
assert!(RootPlacementTypeRegistry::register(Identifier::parse("example:parent_roots").unwrap(), |_| Ok(RootPlacement::new(WorldgenBindings::default(), |_| Ok(None)))));
```

### `RootPlacementTypeRegistry::decode`

`fn decode(id: &Identifier, configuration: &serde_json::Value) -> Result<Option<RootPlacement>, String>`. `Ok(None)` is unknown; `Ok(Some(...))` is a decoded override; `Err` is a codec/registry failure. Preserve registered errors rather than falling through to another algorithm.

```rust
use pluto_lib::{Identifier, RootPlacementTypeRegistry};
let id = Identifier::parse("example:invalid_root_override").unwrap();
RootPlacementTypeRegistry::register(id.clone(), |_| Err("invalid root settings".into()));
assert_eq!(RootPlacementTypeRegistry::decode(&id, &serde_json::Value::Null).err().as_deref(), Some("invalid root settings"));
```

### `StructureWorld`

Borrowed query/RNG interface for one structure attempt. These required methods have no guessed default implementation:

| Method | Return and meaning |
|---|---|
| `top_of_chunk_center(&mut self, chunk: [i32; 2])` | `Result<Option<BlockPos>, String>`: native WORLD_SURFACE center; `None` when the column has no possible valid biome |
| `is_valid_biome(&mut self, pos: BlockPos)` | `Result<bool, String>`: the attempt's native biome predicate at a block position; explicit placement retains its accept-all predicate |
| `first_occupied_height(&mut self, x: i32, z: i32)` | `Result<i32, String>`: occupied WORLD_SURFACE_WG height, not first free Y |
| `generation_depth(&self)` | `i32`: generator depth, not maximum build Y |
| `sea_level(&self)` | `i32`: generator sea level |
| `random_int(&mut self, bound: i32)` | `Result<i32, String>`: one draw from the existing structure RNG, for a positive bound |

The following in-memory example supplies explicit answers, not a model of native terrain generation:

```rust
use pluto_lib::StructureWorld;
struct Queries;
impl StructureWorld for Queries {
    fn top_of_chunk_center(&mut self, _: [i32; 2]) -> Result<Option<[i32; 3]>, String> { Ok(Some([8, 70, 8])) }
    fn is_valid_biome(&mut self, _: [i32; 3]) -> Result<bool, String> { Ok(true) }
    fn first_occupied_height(&mut self, _: i32, _: i32) -> Result<i32, String> { Ok(70) }
    fn generation_depth(&self) -> i32 { 384 }
    fn sea_level(&self) -> i32 { 63 }
    fn random_int(&mut self, bound: i32) -> Result<i32, String> {
        if bound == 5 { Ok(3) } else { Err("unexpected bound".into()) }
    }
}
let mut world = Queries;
assert_eq!(world.top_of_chunk_center([0, 0]), Ok(Some([8, 70, 8])));
assert_eq!(world.is_valid_biome([8, 70, 8]), Ok(true));
assert_eq!(world.first_occupied_height(7, 7), Ok(70));
assert_eq!(world.generation_depth(), 384);
assert_eq!(world.sea_level(), 63);
assert_eq!(world.random_int(5), Ok(3));
assert!(world.random_int(0).is_err());
```

> **Use with Caution.** Explicit native commands provide a real borrowed structure context; natural-start dispatch remains unconnected. Do not implement unavailable columns as height zero, replace a failed draw, reseed per method or retain borrowed contexts in globals/TLS.

### `StructureContext`

`StructureContext<'a> { world: &'a mut dyn StructureWorld, chunk: [i32; 2] }`. The `chunk` identifies the generation attempt, and `world` supplies that attempt's queries/randomness. There is deliberately no block-writing editor during planning.

```rust
use pluto_lib::{StructureContext, StructureWorld};
struct Unavailable;
impl StructureWorld for Unavailable {
    fn top_of_chunk_center(&mut self, _: [i32; 2]) -> Result<Option<[i32; 3]>, String> { Err("unavailable".into()) }
    fn is_valid_biome(&mut self, _: [i32; 3]) -> Result<bool, String> { Err("unavailable".into()) }
    fn first_occupied_height(&mut self, _: i32, _: i32) -> Result<i32, String> { Err("unavailable".into()) }
    fn generation_depth(&self) -> i32 { 384 }
    fn sea_level(&self) -> i32 { 63 }
    fn random_int(&mut self, _: i32) -> Result<i32, String> { Err("unavailable".into()) }
}
let mut world = Unavailable;
let context = StructureContext { world: &mut world, chunk: [-2, 3] };
assert_eq!(context.chunk, [-2, 3]);
assert!(context.world.first_occupied_height(-25, 55).is_err());
```

### `StructurePiece`

`StructurePiece { kind: Identifier, configuration: serde_json::Value }`. The owned `kind` selects a piece codec; `configuration` carries its settings and anchors, not host registry IDs. The native adapter under verification accepts only `pluto_lib:template_piece`, with exactly `template` (namespaced ID), `origin` (three i32 coordinates), `rotation` (`NONE`, `CLOCKWISE_90`, `CLOCKWISE_180`, `COUNTERCLOCKWISE_90`) and `mirror` (`NONE`, `LEFT_RIGHT`, `FRONT_BACK`). All four fields are required. Empty/missing templates, entities, structure markers, jigsaws, unsafe template paths and overflowing transformed bounds reject before start publication. Structure void blocks are ignored; native plain-template placement supplies flags 2. Custom piece codecs are not connected. Starts are bounded to 1–256 pieces; descriptors are limited to depth 128, 65,536 nodes and 1 MiB of strings, with encoded piece settings also capped at 1 MiB. Settings are saved with piece version 1 and decoded again on restore; saved bounds do not override recomputed bounds. The explicit placement subset passes eleven live cases; persistence, every possible rotation/mirror combination and all descriptor-budget boundaries have not been runtime-verified.

```rust
use pluto_lib::{Identifier, StructurePiece};
let piece = StructurePiece {
    kind: Identifier::parse("pluto_lib:template_piece").unwrap(),
    configuration: serde_json::json!({"template":"example:house", "origin":[8,80,8], "rotation":"NONE", "mirror":"NONE"}),
};
assert_eq!(piece.clone().configuration["origin"], serde_json::json!([8,80,8]));
```

### `StructureStart`

`StructureStart { position: BlockPos, pieces: Vec<StructurePiece> }`. `position` is the generation stub's biome-check origin. `pieces` are owned codec descriptors in the planner's order; template anchors may be elsewhere inside their configuration. This value is not a native cached start and does not write or save any blocks.

```rust
use pluto_lib::{Identifier, StructurePiece, StructureStart};
let start = StructureStart { position: [8,70,8], pieces: vec![StructurePiece {
    kind: Identifier::parse("example:template_piece").unwrap(),
    configuration: serde_json::json!({"origin":[8,130,8]}),
}] };
assert_ne!(start.position[1], start.pieces[0].configuration["origin"][1].as_i64().unwrap() as i32);
assert_eq!(start.clone(), start);
```

### `ConfiguredStructure`

A cloneable decoded structure algorithm with owned immutable captures. Each invocation uses the caller's `StructureContext`; cloning does not duplicate a world/random stream or cache a native start.

```rust
use pluto_lib::ConfiguredStructure;
let structure = ConfiguredStructure::new(|_| Ok(None));
let _snapshot = structure.clone();
```

### `ConfiguredStructure::new`

`fn new(plan: impl Fn(&mut StructureContext<'_>) -> Result<Option<StructureStart>, String> + Send + Sync + 'static) -> ConfiguredStructure`. Capture already-validated settings. `Ok(None)` rejects the current attempt; `Err` propagates a query/planning failure rather than requesting another implementation. The planner must not place blocks.

```rust
use pluto_lib::ConfiguredStructure;
let enabled = false;
let _structure = ConfiguredStructure::new(move |_| {
    if !enabled { return Ok(None); }
    Err("no enabled planner supplied".into())
});
```

### `ConfiguredStructure::find_generation_point`

`fn find_generation_point(&self, context: &mut StructureContext<'_>) -> Result<Option<StructureStart>, String>`. Execute the decoded planner, preserving its rejection or error. Direct execution is not natural generation, command dispatch, template placement or piece persistence.

```rust
use pluto_lib::{ConfiguredStructure, StructureContext, StructureWorld};
struct EmptyColumn;
impl StructureWorld for EmptyColumn {
    fn top_of_chunk_center(&mut self, _: [i32; 2]) -> Result<Option<[i32; 3]>, String> { Ok(None) }
    fn is_valid_biome(&mut self, _: [i32; 3]) -> Result<bool, String> { panic!("no candidate") }
    fn first_occupied_height(&mut self, _: i32, _: i32) -> Result<i32, String> { panic!("no candidate") }
    fn generation_depth(&self) -> i32 { 384 }
    fn sea_level(&self) -> i32 { 63 }
    fn random_int(&mut self, _: i32) -> Result<i32, String> { panic!("no candidate") }
}
let structure = ConfiguredStructure::new(|context| {
    let Some(_position) = context.world.top_of_chunk_center(context.chunk)? else { return Ok(None); };
    Err("unexpected candidate".into())
});
let mut world = EmptyColumn;
assert_eq!(structure.find_generation_point(&mut StructureContext { world: &mut world, chunk: [0,0] }), Ok(None));
```

### `StructureTypeRegistry`

Namespaced structure-type codecs. Registration is process-local; decoded snapshots retain their captures when a type is replaced. Codecs and replaced-capture destructors run outside registry locks. Explicit native placement is implemented under verification; natural generation remains unconnected. Registration does not verify either path.

```rust
use pluto_lib::{Identifier, StructureTypeRegistry};
let id = Identifier::parse("example:unknown_structure").unwrap();
assert!(StructureTypeRegistry::decode(&id, &serde_json::Value::Null).unwrap().is_none());
```

### `StructureTypeRegistry::register`

`fn register(id: Identifier, codec: impl Fn(&serde_json::Value) -> Result<ConfiguredStructure, String> + Send + Sync + 'static) -> bool`. Register or replace a decoder. False reports a poisoned registry lock. Validate settings before querying a world or consuming structure RNG.

```rust
use pluto_lib::{ConfiguredStructure, Identifier, StructureTypeRegistry};
assert!(StructureTypeRegistry::register(Identifier::parse("example:empty_structure").unwrap(), |_| {
    Ok(ConfiguredStructure::new(|_| Ok(None)))
}));
```

### `StructureTypeRegistry::decode`

`fn decode(id: &Identifier, configuration: &serde_json::Value) -> Result<Option<ConfiguredStructure>, String>`. Unknown types return `Ok(None)`; registered types return their snapshot or a codec error. A registered error is never permission for native fallback. This method does not query or write a world.

```rust
use pluto_lib::{Identifier, StructureTypeRegistry};
let id = Identifier::parse("example:bad_structure").unwrap();
StructureTypeRegistry::register(id.clone(), |_| Err("bad template weights".into()));
assert_eq!(StructureTypeRegistry::decode(&id, &serde_json::Value::Null).err().as_deref(), Some("bad template weights"));
```

### `StructureRotation`

`enum StructureRotation { None, Clockwise90, Clockwise180, Counterclockwise90 }`. Rotation about the template's origin with zero pivot; `Default` is `None`. The native adapter must apply the mirror first.

```rust
use pluto_lib::StructureRotation;
assert_eq!(StructureRotation::default(), StructureRotation::None);
let rotations = [StructureRotation::None, StructureRotation::Clockwise90,
    StructureRotation::Clockwise180, StructureRotation::Counterclockwise90];
assert_eq!(rotations.len(), 4);
```

### `StructureMirror`

`enum StructureMirror { None, LeftRight, FrontBack }`. Minecraft's mirror before rotation. `Default` is `None`.

```rust
use pluto_lib::StructureMirror;
assert_eq!(StructureMirror::default(), StructureMirror::None);
assert_ne!(StructureMirror::LeftRight, StructureMirror::FrontBack);
```

### `StructurePieceTemplate`

`StructurePieceTemplate { template: Identifier, origin: BlockPos, rotation: StructureRotation, mirror: StructureMirror }`. Owned immutable settings for a registered template piece. Native binding must validate the template and bounds before any piece writes. Template entities are ignored; DATA markers are dispatched separately in native order.

```rust
use pluto_lib::{Identifier, StructurePieceTemplate, StructureRotation, StructureMirror};
let template = StructurePieceTemplate {
    template: Identifier::parse("example:buildings/house").unwrap(), origin: [-8, 70, 24],
    rotation: StructureRotation::Clockwise90, mirror: StructureMirror::FrontBack,
};
assert_eq!(template.clone().origin, [-8,70,24]);
```

> **Use with Caution.** Native explicit `/place structure` supports registered template pieces through the typed placement adapter: it validates all piece definitions before writes, clips DATA markers per chunk, and shares the actual world and RNG with configured-feature placement. Mutable state belongs to each live piece. Natural placement remains unconnected; native saved-piece restoration exists but save/reload lifecycle behavior is unverified. Explicit commands do not create cached saved starts.

### `StructurePieceWorld`

Required fallible operations for one piece's real post-processing world. Never substitute default blocks, independent RNG, unrestricted writes or successful feature placement for an unavailable operation.

| Method | Return and contract |
|---|---|
| `block_state(&mut self, pos: BlockPos)` | `Result<BlockState, String>`: actual block at the marker/query position |
| `set_block_state(&mut self, pos: BlockPos, state: &BlockState, flags: i32)` | `Result<bool, String>`: clipped write with the supplied update flags |
| `face_sturdy(&mut self, pos: BlockPos, face: Direction)` | `Result<bool, String>`: native/registered support of that face |
| `can_survive_plant(&mut self, state: &BlockState, pos: BlockPos)` | `Result<bool, String>`: true only for a plant whose native/registered survival test passes |
| `random_int(&mut self, bound: i32)` | `Result<i32, String>`: existing piece RNG; positive bound |
| `random_bool(&mut self)` | `Result<bool, String>`: actual boolean draw, not a replacement integer draw |
| `place_feature(&mut self, feature: &Identifier, pos: BlockPos)` | `Result<bool, String>`: same world/RNG; false for an attempted failure, error for unavailable execution |

```rust
use pluto_lib::{BlockState, Direction, Identifier, StructurePieceWorld};
struct Unavailable;
impl StructurePieceWorld for Unavailable {
    fn block_state(&mut self, _: [i32;3]) -> Result<BlockState,String> { Err("block unavailable".into()) }
    fn set_block_state(&mut self, _: [i32;3], _: &BlockState, _: i32) -> Result<bool,String> { Err("write unavailable".into()) }
    fn face_sturdy(&mut self, _: [i32;3], _: Direction) -> Result<bool,String> { Err("face unavailable".into()) }
    fn can_survive_plant(&mut self, _: &BlockState, _: [i32;3]) -> Result<bool,String> { Err("plant unavailable".into()) }
    fn random_int(&mut self, _: i32) -> Result<i32,String> { Err("random unavailable".into()) }
    fn random_bool(&mut self) -> Result<bool,String> { Err("random unavailable".into()) }
    fn place_feature(&mut self, _: &Identifier, _: [i32;3]) -> Result<bool,String> { Err("feature unavailable".into()) }
}
let mut world = Unavailable;
let stone = BlockState::parse("minecraft:stone").unwrap();
assert!(world.block_state([0,70,0]).is_err());
assert!(world.set_block_state([0,70,0], &stone, 2).is_err());
assert!(world.face_sturdy([0,69,0], Direction::Up).is_err());
assert!(world.can_survive_plant(&stone, [0,70,0]).is_err());
assert!(world.random_int(5).is_err());
assert!(world.random_bool().is_err());
assert!(world.place_feature(&Identifier::parse("example:tree").unwrap(), [0,70,0]).is_err());
```

### `StructureMarkerContext`

`StructureMarkerContext<'a> { world: &'a mut dyn StructurePieceWorld, marker: &'a str, position: BlockPos, state: &'a mut serde_json::Value }`. The host supplies an already transformed/clipped DATA marker and mutable state belonging to this actual piece. It must serialize concurrent access and save state separately from immutable codec settings. Neither context nor world may be retained.

```rust
use pluto_lib::StructureMarkerContext;
fn increment(context: &mut StructureMarkerContext<'_>) -> Result<(), String> {
    if context.marker == "example:success" {
        let count = context.state.as_u64().ok_or("invalid saved counter")?;
        *context.state = serde_json::Value::from(count + 1);
    }
    Ok(())
}
let mut state = serde_json::json!(0);
// Contexts are supplied by the host; the owned state remains ordinary JSON.
state = serde_json::Value::from(state.as_u64().unwrap() + 1);
assert_eq!(state, 1);
```

### `ConfiguredStructurePiece`

An immutable template piece definition with owned initial state and marker behavior. It does not itself own a live piece's changing counter.

```rust
use pluto_lib::{ConfiguredStructurePiece, Identifier, StructurePieceTemplate};
let piece = ConfiguredStructurePiece::new(StructurePieceTemplate {
    template: Identifier::parse("example:house").unwrap(), origin: [0,70,0],
    rotation: Default::default(), mirror: Default::default(),
}, serde_json::Value::Null, |_| Ok(()));
assert!(piece.initial_state().is_null());
```

### `ConfiguredStructurePiece::new`

`fn new(template: StructurePieceTemplate, initial_state: serde_json::Value, marker: impl Fn(&mut StructureMarkerContext<'_>) -> Result<(), String> + Send + Sync + 'static) -> Self`. Captures immutable settings, initial state and a marker callback. `fn template(&self) -> &StructurePieceTemplate` and `fn initial_state(&self) -> &serde_json::Value` borrow those values. `fn handle_data_marker(&self, context: &mut StructureMarkerContext<'_>) -> Result<(), String>` directly invokes the callback. Clones share the callback, not a live piece's mutable state. Errors preserve earlier writes and state changes; this API is not a transaction.

```rust
use pluto_lib::{ConfiguredStructurePiece, Identifier, StructurePieceTemplate, StructureRotation, StructureMirror};
let settings = StructurePieceTemplate { template: Identifier::parse("example:house").unwrap(),
    origin: [0,70,0], rotation: StructureRotation::None, mirror: StructureMirror::None };
let piece = ConfiguredStructurePiece::new(settings.clone(), serde_json::json!({"count":0}), |_| Ok(()));
assert_eq!(piece.template(), &settings);
let mut state = piece.initial_state().clone();
state["count"] = serde_json::json!(1);
assert_eq!(piece.clone().initial_state()["count"], 0);
```

> **Use with Caution.** Direct callback execution does not place a template, create a persisted start or prove native marker integration. A host must bind all pieces first, keep the same RNG, and validate saved state before resuming callbacks.

### `StructurePieceTypeRegistry`

Registry of namespaced template-piece codecs; decoded snapshots outlive registry replacement.

```rust
use pluto_lib::{Identifier, StructurePieceTypeRegistry};
let unknown = Identifier::parse("example:not_registered_piece").unwrap();
assert!(StructurePieceTypeRegistry::decode(&unknown, &serde_json::Value::Null).unwrap().is_none());
```

### `StructurePieceTypeRegistry::register`

`fn register(id: Identifier, codec: impl Fn(&serde_json::Value) -> Result<ConfiguredStructurePiece, String> + Send + Sync + 'static) -> bool`. Register or replace a piece codec; false means the registry lock is poisoned. Decoders and destructors of replaced captures run outside the lock. `fn decode(id: &Identifier, configuration: &serde_json::Value) -> Result<Option<ConfiguredStructurePiece>, String>` returns an immutable snapshot, `None` for unknown kinds, or the registered error. Neither operation writes a world; native dispatch is not connected yet.

```rust
use pluto_lib::{ConfiguredStructurePiece, Identifier, StructurePieceTemplate, StructureRotation, StructureMirror, StructurePieceTypeRegistry};
let id = Identifier::parse("example:registered_piece").unwrap();
assert!(StructurePieceTypeRegistry::decode(&id, &serde_json::Value::Null).unwrap().is_none());
assert!(StructurePieceTypeRegistry::register(id.clone(), |value| {
    if !value.is_null() { return Err("expected null settings".into()); }
    Ok(ConfiguredStructurePiece::new(StructurePieceTemplate {
        template: Identifier::parse("example:house").unwrap(), origin: [0,70,0],
        rotation: StructureRotation::None, mirror: StructureMirror::None,
    }, serde_json::json!({"count":0}), |_| Ok(())))
}));
let snapshot = StructurePieceTypeRegistry::decode(&id, &serde_json::Value::Null).unwrap().unwrap();
assert_eq!(snapshot.initial_state()["count"], 0);
assert!(StructurePieceTypeRegistry::decode(&id, &serde_json::json!(false)).is_err());
```

### `ConfiguredStructurePiece::template`

`fn template(&self) -> &StructurePieceTemplate`. Borrow immutable settings; changing a separate clone does not move this piece.

```rust
use pluto_lib::{ConfiguredStructurePiece, Identifier, StructurePieceTemplate, StructureRotation, StructureMirror};
let settings = StructurePieceTemplate { template: Identifier::parse("example:house").unwrap(),
    origin: [0,70,0], rotation: StructureRotation::None, mirror: StructureMirror::None };
let piece = ConfiguredStructurePiece::new(settings, serde_json::Value::Null, |_| Ok(()));
assert_eq!(piece.template().origin, [0,70,0]);
```

### `ConfiguredStructurePiece::initial_state`

`fn initial_state(&self) -> &serde_json::Value`. Borrow the initial snapshot. Clone it once per actual piece; never mutate a shared global state for all placements.

```rust
use pluto_lib::{ConfiguredStructurePiece, Identifier, StructurePieceTemplate, StructureRotation, StructureMirror};
let piece = ConfiguredStructurePiece::new(StructurePieceTemplate {
    template: Identifier::parse("example:house").unwrap(), origin: [0,70,0],
    rotation: StructureRotation::None, mirror: StructureMirror::None,
}, serde_json::json!({"placed":0}), |_| Ok(()));
let mut state = piece.initial_state().clone();
state["placed"] = serde_json::json!(1);
assert_eq!(piece.initial_state()["placed"], 0);
```

### `ConfiguredStructurePiece::handle_data_marker`

`fn handle_data_marker(&self, context: &mut StructureMarkerContext<'_>) -> Result<(), String>`. Invoke the captured callback directly on the supplied piece's mutable state. Unknown marker policy belongs to that callback. This does not create or place a native template.

```rust
use pluto_lib::{BlockState, Direction, Identifier, ConfiguredStructurePiece, StructureMarkerContext,
    StructurePieceTemplate, StructurePieceWorld, StructureRotation, StructureMirror};
struct Unavailable;
impl StructurePieceWorld for Unavailable {
    fn block_state(&mut self, _: [i32;3]) -> Result<BlockState,String> { Err("block unavailable".into()) }
    fn set_block_state(&mut self, _: [i32;3], _: &BlockState, _: i32) -> Result<bool,String> { Err("write unavailable".into()) }
    fn face_sturdy(&mut self, _: [i32;3], _: Direction) -> Result<bool,String> { Err("face unavailable".into()) }
    fn can_survive_plant(&mut self, _: &BlockState, _: [i32;3]) -> Result<bool,String> { Err("plant unavailable".into()) }
    fn random_int(&mut self, _: i32) -> Result<i32,String> { Err("random unavailable".into()) }
    fn random_bool(&mut self) -> Result<bool,String> { Err("random unavailable".into()) }
    fn place_feature(&mut self, _: &Identifier, _: [i32;3]) -> Result<bool,String> { Err("feature unavailable".into()) }
}
let piece = ConfiguredStructurePiece::new(StructurePieceTemplate {
    template: Identifier::parse("example:house").unwrap(), origin: [0,70,0],
    rotation: StructureRotation::None, mirror: StructureMirror::None,
}, serde_json::json!(0), |context| {
    if context.marker == "example:count" {
        let count = context.state.as_u64().ok_or("bad counter")?;
        *context.state = serde_json::json!(count + 1);
    }
    Ok(())
});
let mut world = Unavailable;
let mut state = piece.initial_state().clone();
let mut context = StructureMarkerContext { world: &mut world, marker: "example:count",
    position: [-8,71,24], state: &mut state };
assert_eq!(context.position, [-8,71,24]);
piece.handle_data_marker(&mut context).unwrap();
assert_eq!(state, 1);
assert_eq!(piece.initial_state(), &serde_json::json!(0));
```

### `StructurePieceTypeRegistry::decode`

`fn decode(id: &Identifier, configuration: &serde_json::Value) -> Result<Option<ConfiguredStructurePiece>, String>`. Clone the decoder outside the registry lock, then decode. Unknown kinds return `Ok(None)`; registered errors stay errors and do not permit native fallback.

```rust
use pluto_lib::{Identifier, StructurePieceTypeRegistry};
let id = Identifier::parse("example:bad_piece").unwrap();
assert!(StructurePieceTypeRegistry::decode(&id, &serde_json::Value::Null).unwrap().is_none());
StructurePieceTypeRegistry::register(id.clone(), |_| Err("invalid template settings".into()));
assert_eq!(StructurePieceTypeRegistry::decode(&id, &serde_json::Value::Null).err().as_deref(),
    Some("invalid template settings"));
```

### `ConfiguredStructurePiece::with_state_validator`

`fn with_state_validator(self, validator: impl Fn(&serde_json::Value) -> Result<(), String> + Send + Sync + 'static) -> Self`. Attach a type-specific saved-state check. Hosts must run it on initial/restored state before template writes and before saving; direct marker calls run it before and after successful callbacks. Without a validator state is opaque JSON, still subject to the host's independent depth/size budgets. Validation errors do not roll back prior writes or mutations.

```rust
use pluto_lib::{ConfiguredStructurePiece, Identifier, StructurePieceTemplate};
let piece = ConfiguredStructurePiece::new(StructurePieceTemplate {
    template: Identifier::parse("example:house").unwrap(), origin: [0,70,0],
    rotation: Default::default(), mirror: Default::default(),
}, serde_json::json!(0), |_| Ok(())).with_state_validator(|state| {
    if state.as_u64().is_some_and(|n| n <= 3) { Ok(()) } else { Err("invalid counter".into()) }
});
assert!(piece.validate_state(piece.initial_state()).is_ok());
assert!(piece.validate_state(&serde_json::json!(-1)).is_err());
```

### `ConfiguredStructurePiece::validate_state`

`fn validate_state(&self, state: &serde_json::Value) -> Result<(), String>`. Validate without querying or writing a world. This does not replace host JSON budgets, registry binding or a real save/reload test.

```rust
use pluto_lib::{ConfiguredStructurePiece, Identifier, StructurePieceTemplate};
let piece = ConfiguredStructurePiece::new(StructurePieceTemplate {
    template: Identifier::parse("example:house").unwrap(), origin: [0,70,0],
    rotation: Default::default(), mirror: Default::default(),
}, serde_json::Value::Null, |_| Ok(()));
assert_eq!(piece.validate_state(&serde_json::json!({"opaque":true})), Ok(()));
```
