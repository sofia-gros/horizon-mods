# Colour providers: the tint of a block's faces

A block model marks the faces that take a tint (grass, leaves) and the game fills in the colour. A colour provider lets your mod answer for its own blocks, and the answer may depend on where the block stands: a colour map laid across the world, a pattern that repeats every 256 blocks. This page first walks through using it, then lists every item in the [Reference](#reference).

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

> **Use with Caution.** The verified path is world-chunk base faces only. A separate resource-facade test does not extend this coverage to another rendering path. A context with `pos: None` is valid for a direct API call, but does not mean the item/hand renderer invokes the provider. Particle tinting and other rendering paths are separate too. Register providers before chunk-mesh dispatch; changing a registration does not itself request a chunk remesh. Keep providers short, thread-safe and independent of live player/world operations, since chunk meshing can run on worker threads.

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

`fn block_color(context: &BlockColorContext) -> Option<u32>`. Call the registered provider for `context.state`'s block. Returns its `0xRRGGBB` answer, or `None` when no provider is registered or the provider declines. This direct lookup works independently of renderer wiring and accepts a manually supplied context, including `pos: None`; it does not establish that a game rendering path is connected. The world-chunk base-face adapter retains the native tint on `None`.

```rust
use pluto_lib::{BlockColorContext, BlockState, ColorProviderRegistry, Identifier};

ColorProviderRegistry::register_block(&[Identifier::new("example", "tinted_moss").unwrap()], |_| Some(0x336633));
let face = BlockColorContext { state: BlockState::parse("example:tinted_moss").unwrap(), pos: None, tint_index: 0 };
assert_eq!(ColorProviderRegistry::block_color(&face), Some(0x336633));
let other = BlockColorContext { state: BlockState::parse("example:plain_stone").unwrap(), pos: None, tint_index: 0 };
assert_eq!(ColorProviderRegistry::block_color(&other), None);
```
