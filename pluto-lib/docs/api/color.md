# Colour providers: the tint of a block's faces

A block model marks the faces that take a tint (grass, leaves) and the game fills in the colour. A colour provider lets your mod answer for its own blocks, and the answer may depend on where the block stands: a colour map laid across the world, a pattern that repeats every 256 blocks. This page first walks through using it, then lists every item in the [Reference](#reference).

> **Use with Caution.** World-render dispatch remains unimplemented. Two seams were measured on the pinned Mixins: direct injection into `Mesher::face` was rejected for destructured parameters, and wrapping its call from `mesh_reusing` was rejected with `E_UNSUPPORTED_CONTROL_FLOW` (closures, async, try/`?`, unsafe and const blocks are unsupported injection contexts). These failures do not prove every alternative is impossible, but neither attempt connects rendering. Item/hand, particle and other block-rendering paths remain unconnected too (see the support matrix).

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

The proposed world-chunk adapter would pass the block's real state and properties, world position and model tint index, resolving state names through the asset registry belonging to that mesher rather than a global startup snapshot. It would skip negative tint indices and retain native tint when a provider declines. The two attempted injection seams were rejected, so this dispatch is not implemented. A provider can still be registered and queried directly; limit it to the model layers it owns. This pattern example illustrates the general API, not a working renderer hookup or Tinkers' colour-map algorithm:

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

The Tinkers colour consumer is implemented against the original Java and reads its colour maps through `ResourceManagerHelper::read_client_image`, replacing owned snapshots on client-resource notifications (see [resources](resources.md)). Connecting a tint hook or loading an image alone does not establish parity with the original sampling rules. The consumer, resource-pack overrides and reload behaviour remain unverified in game.

The consumer accepts only 256×256 images with a complete four-byte RGBA pixel buffer. This is an intentional safety difference for malformed resources: the original Java can index outside a short colour map, while the port rejects wrong dimensions or buffer length and keeps the previous map (the initial zero-filled map if none has loaded). This does not claim identical behaviour for malformed packs.

> **Use with Caution.** The renderer connection is not implemented or game-validated. Testing the resource facade separately does not establish that block tint providers reach world rendering. A context with `pos: None` is valid for a direct API call, but does not mean the item/hand renderer invokes the provider. Particle tinting and other rendering paths are separate too. Register providers before any future chunk-mesh dispatch; changing a registration does not itself request a chunk remesh. Keep providers short, thread-safe and independent of live player/world operations, since chunk meshing can run on worker threads.

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

`fn block_color(context: &BlockColorContext) -> Option<u32>`. Call the registered provider for `context.state`'s block. Returns its `0xRRGGBB` answer, or `None` when no provider is registered or the provider declines. This direct lookup works independently of renderer wiring and accepts a manually supplied context, including `pos: None`; it does not establish that a game rendering path is connected. The world-mesh adapter, when connected, is responsible for retaining the native tint on `None`.

```rust
use pluto_lib::{BlockColorContext, BlockState, ColorProviderRegistry, Identifier};

ColorProviderRegistry::register_block(&[Identifier::new("example", "tinted_moss").unwrap()], |_| Some(0x336633));
let face = BlockColorContext { state: BlockState::parse("example:tinted_moss").unwrap(), pos: None, tint_index: 0 };
assert_eq!(ColorProviderRegistry::block_color(&face), Some(0x336633));
let other = BlockColorContext { state: BlockState::parse("example:plain_stone").unwrap(), pos: None, tint_index: 0 };
assert_eq!(ColorProviderRegistry::block_color(&other), None);
```
