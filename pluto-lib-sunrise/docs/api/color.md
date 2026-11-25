# Colour providers: block and item tints

A model marks the faces that take a tint (grass, leaves) and the game fills in the colour. A block provider can depend on where the block stands; an item provider receives stack data and the model tint index. World-block rendering is connected; native item rendering remains a documented stub. This page first walks through using the providers, then lists every public item in the [Reference](#reference).

> **Use with Caution.** World-chunk base-face dispatch has been built and checked in a real client: a provider renders magenta leaves while `None` preserves the native green, with real state properties, position and tint index. The updated pinned Mixins accepts the first-tint `MODIFY_VARIABLE` seam; the old call-site wrap is not used. Item/hand, particle, connected-texture overlay tint evaluation and other block-rendering paths remain unconnected (see the support matrix).

## Basic Example

A leaf block that turns greener towards the east:

```rust
use pluto_lib::{ColorProviderRegistry, Identifier};

ColorProviderRegistry::register_block(&[Identifier::new("example", "meadow_leaves").unwrap()], |face| {
    let east = face.pos?[0].rem_euclid(256) as u32;
    Some(0x2E0000 | ((0x60 + east / 2) << 8) | 0x20)
});
```

The provider gets a `BlockColorContext`. Answer `Some(0xRRGGBB)` to supply a tint, or `None` to retain the game's own tint. The example registers a provider; registration or a direct lookup is not evidence that a renderer called it.

## World Rendering and Other Paths

The world-chunk adapter resolves the block's real state and properties through that mesher's asset registry and supplies its world position and model tint index. It replaces only the first native tint value, before native model-colour modulation and lighting, skips negative tint indices and retains native tint when a provider declines. Provider dispatch and native fallback have been checked in game. A provider can also be registered and queried directly; limit it to the model layers it owns. This example illustrates the general API, not Tinkers' colour-map algorithm:

```rust
use pluto_lib::{ColorProviderRegistry, Identifier};

ColorProviderRegistry::register_block(&[Identifier::new("example", "patterned_leaves").unwrap()], |face| {
    if face.tint_index != 0 {
        return None;
    }
    let pos = face.pos?;
    let blue = if face.state.property("persistent") == Some("true") { 0x80 } else { 0x40 };
    Some(if pos[0].rem_euclid(2) == 0 { 0x336600 | blue } else { 0x669900 | blue })
});
```

The Tinkers colour consumer is implemented against the original Java and reads its colour maps through `ResourceManagerHelper::read_client_image`, replacing owned snapshots on client-resource notifications (see [resources](resources.md)). Connecting a tint hook or loading an image alone does not establish parity with the original sampling rules. The focused game gate places all 47 original block registrations, loads five controlled colour maps, changes them through actual F3+T, and checks malformed-Ichor-map retention. Initial, reloaded and retained plant images have been inspected; exact world-position folding and leaf/vine offsets also have independent pure tests.

The consumer accepts only 256×256 images with a complete four-byte RGBA pixel buffer. This is an intentional safety difference for malformed resources: the original Java can index outside a short colour map, while the port rejects wrong dimensions or buffer length and keeps the previous map (the initial zero-filled map if none has loaded). This does not claim identical behaviour for malformed packs.

> **Use with Caution.** The verified path is world-chunk base faces only. A separate resource-facade test does not extend this coverage to another rendering path. A context with `pos: None` is valid for a direct API call, but does not mean the item/hand renderer invokes the provider. Common terrain-particle tint now has a source-declared adapter at `Game::particle_sprite`, reusing that game's registry, the supplied world position and tint index zero. It retains native tint on decline/missing state and keeps ordinary grass-block dirt crumbs untinted, as confirmed in the original TerrainParticle bytecode. The actual helper passes a structural-fixture test for state/position/registry isolation and fallback; native particle construction, lighting, attachment and rendering remain unverified. Independent falling-dust and other particle routes are not covered. Register providers before chunk-mesh dispatch; changing a registration does not itself request a chunk remesh. Keep providers short, thread-safe and independent of live player/world operations, since chunk meshing can run on worker threads.

## Item Colours

Item providers receive an owned `StackData` and the model tint index. Use separate registrations when block and item behaviour differ, or explicitly forward a block item to its positionless block lookup. Providers run outside the registry lock, so an alias observes the current block provider rather than a cached colour.

```rust
use pluto_lib::{ColorProviderRegistry, Identifier, ItemColorContext, StackData};

let item = Identifier::parse("example:dyed_leaf").unwrap();
ColorProviderRegistry::register_item(&[item], |face| {
    (face.tint_index == 0).then_some(0x123456)
});
let face = ItemColorContext {
    stack: StackData { item: "example:dyed_leaf".into(), count: 1, ..Default::default() },
    tint_index: 0,
};
assert_eq!(ColorProviderRegistry::item_color(&face), Some(0x123456));
```

> **Use with Caution.** Item registration and direct dispatch are pure-tested, including full reduced-stack data and reentrant replacement. A native baked-layer adapter is source-declared at the shared `resolve_with_rule` RETURN boundary, including CIT/GUI/hand callers. It replaces only the completed state's layers, retaining display, animation, oversized-GUI policy and glint/HDR fields. Individual planning passes against the pinned official Mixins; full attachment, types and runtime remain unvalidated under the native attachment gate. This remains a renderer stub, not a rendered-colour claim. Tinkers registers the original 42 block-item aliases; potted ferns remain block-only. Its aliases query block providers with no world position and forward the tint index. The source bridge reads the native resolver's already merged components, preserves unregistered/declined tints and special geometry, writes opaque ARGB only for registered quad tint indices, and never mutates shared baked models. One test executes that bridge and its injection helper with structural fixtures; it does not test native model baking or prototype/patch merging. Indices above 4095 deliberately retain native tint to bound malformed-model allocations; this safety limit differs from an unbounded original tint array.

## Reference

### `BlockColorContext`

What a block colour provider is asked. `state` is the block's state including its properties, `pos` the world position when one is supplied, and `tint_index` the model's `tintindex`. `None` represents a lookup without a position; automatic item/hand lookups are not connected by the world-mesh adapter.

```rust
use pluto_lib::{BlockColorContext, BlockState};

let face = BlockColorContext { state: BlockState::parse("example:meadow_leaves").unwrap(), pos: Some([3, 70, 9]), tint_index: 0 };
assert_eq!(face.pos, Some([3, 70, 9]));
assert_eq!(face.tint_index, 0);
assert_eq!(face.state.block.path(), "meadow_leaves");
```

### `ItemColorContext`

`ItemColorContext { stack: StackData, tint_index: i32 }` carries the stack being drawn, including count, damage, durability override and custom data, plus the model face's tint index. It has no fabricated world position.

```rust
use pluto_lib::{ItemColorContext, StackData};
let face = ItemColorContext { stack: StackData { item: "example:leaf".into(), count: 2, ..Default::default() }, tint_index: 1 };
assert_eq!(face.stack.count, 2);
assert_eq!(face.tint_index, 1);
```

### `ColorProviderRegistry::register_item`

`fn register_item(items: &[Identifier], provider: impl Fn(&ItemColorContext) -> Option<u32> + Send + Sync + 'static)` registers one provider for each item. Later registrations replace earlier ones; callback execution and replaced callback destruction occur outside the lock. **Stub:** native item/hand renderers do not invoke it yet.

```rust
use pluto_lib::{ColorProviderRegistry, Identifier};
ColorProviderRegistry::register_item(&[Identifier::parse("example:berries").unwrap()], |_| Some(0xFF3300));
```

### `ColorProviderRegistry::item_color`

`fn item_color(context: &ItemColorContext) -> Option<u32>` directly queries the stack item's provider. `Some(0xRRGGBB)` supplies its colour; `None` means no registration or an explicit decline. This lookup is functional independently of the native renderer stub.

```rust
use pluto_lib::{ColorProviderRegistry, Identifier, ItemColorContext, StackData};
ColorProviderRegistry::register_item(&[Identifier::parse("example:plain_item").unwrap()], |_| None);
let face = ItemColorContext { stack: StackData { item: "example:plain_item".into(), count: 1, ..Default::default() }, tint_index: 0 };
assert_eq!(ColorProviderRegistry::item_color(&face), None);
```

### `ColorProviderRegistry`

The registry of colour providers (Fabric: `ColorProviderRegistry`).

```rust
use pluto_lib::ColorProviderRegistry;

let _ = ColorProviderRegistry::block_color;
```

### `ColorProviderRegistry::register_block`

`fn register_block(blocks: &[Identifier], provider: impl Fn(&BlockColorContext) -> Option<u32> + Send + Sync + 'static)`. Gives `blocks` a provider (Fabric: `ColorProviderRegistry.BLOCK.register`). It answers the colour as `0xRRGGBB`, or `None` to leave the face to the game's tint. A block registered again takes the newer provider.

```rust
use pluto_lib::{ColorProviderRegistry, Identifier};

ColorProviderRegistry::register_block(
    &[Identifier::new("example", "fern").unwrap(), Identifier::new("example", "tall_fern").unwrap()],
    |_| Some(0x4F9F2F),
);
```

### `ColorProviderRegistry::block_color`

`fn block_color(context: &BlockColorContext) -> Option<u32>`. Call the registered provider for `context.state`'s block. Returns its `0xRRGGBB` answer, or `None` when no provider is registered or the provider declines. This direct lookup works independently of renderer wiring and accepts a manually supplied context, including `pos: None`; it does not establish that a game rendering path is connected. The world-chunk base-face adapter retains the native tint on `None`.

```rust
use pluto_lib::{BlockColorContext, BlockState, ColorProviderRegistry, Identifier};

ColorProviderRegistry::register_block(&[Identifier::new("example", "tinted_moss").unwrap()], |_| Some(0x336633));
let face = BlockColorContext { state: BlockState::parse("example:tinted_moss").unwrap(), pos: None, tint_index: 0 };
assert_eq!(ColorProviderRegistry::block_color(&face), Some(0x336633));
let other = BlockColorContext { state: BlockState::parse("example:plain_stone").unwrap(), pos: None, tint_index: 0 };
assert_eq!(ColorProviderRegistry::block_color(&other), None);
```
