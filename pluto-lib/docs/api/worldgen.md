# Worldgen: feature types and configuration

A feature type supplies a codec that validates data, then a generator that uses the supplied world and origin. This follows Minecraft/Fabric's registered `Feature` and `ConfiguredFeature` concepts. Rust closures capture the decoded settings; random draws use the supplied `BlockEditor` rather than a newly seeded RNG.

> **Use with Caution.** Direct codec/generator calls have pure tests. Live `BlockEditor::place_feature` dispatch has been checked with an independent real-game consumer: configured state/properties, negative origin, random bounds, invalid configuration, bounded recursion and no native fallthrough after registered failure. Native `/place feature` commands, natural generation workers and custom types referenced inside native feature graphs are not connected. Automatic host dispatch currently runs only through PlutoLib's live `BlockEditor::place_feature`. Registering a type is not evidence that those paths invoke it; see the support matrix. Root placers remain separate work. Decorator codecs can be invoked explicitly by a mod's feature generator; native tree decoding and placement do not dispatch them.

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

> **Use with Caution.** This is an explicit-call API. Native tree decoding/placement, placement-set tracking, leaf-distance/edge-shape finalization and natural-generation dispatch remain the generator/host adapter's responsibility. Failed decorators preserve prior writes; their error is not permission to fall back to a second algorithm. Position lists are not sorted or deduplicated here. An independent live feature fixture checked Tinkers' registered leaf-vine loop: vanilla/configured vine blocks, attachment faces, five-block limit, obstruction, zero probability, invalid configuration and write-error ownership; its five images were inspected. Tinkers' leaf-vine codec syntax-checks its block ID; block/property existence is resolved on write, unlike Java's registry-bound decode. Its full registry/parity integration is unfinished.

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
