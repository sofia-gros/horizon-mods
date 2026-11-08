# Colour providers: the tint of a block's faces

A block model marks the faces that take a tint (grass, leaves) and the game fills in the colour. A colour provider lets your mod answer for its own blocks, and the answer may depend on where the block stands: a colour map laid across the world, a pattern that repeats every 256 blocks. This page first walks through using it, then lists every item in the [Reference](#reference).

> **Use with Caution.** Stub. The pinned Horizon Mixins cannot apply the injection that would ask the providers while chunks are meshed (`E_UNSUPPORTED_CONTROL_FLOW`, see the support matrix), so providers are accepted and kept but the game does not ask them yet. Faces keep the game's own tint until it can.

## Basic Example

A leaf block that turns greener towards the east:

```rust
use pluto_lib::{ColorProviderRegistry, Identifier};

ColorProviderRegistry::register_block(&[Identifier::new("example", "meadow_leaves").unwrap()], |face| {
    let east = face.pos?[0].rem_euclid(256) as u32;
    Some(0x2E0000 | ((0x60 + east / 2) << 8) | 0x20)
});
```

The provider gets a `BlockColorContext`. Answer `None` and the face keeps the game's tint; `pos` is `None` when the block is drawn without a place, such as in a hand.

## Reference

### `BlockColorContext`

What a block colour provider is asked. `state` is the block's state, `pos` where it stands (None when drawn without a place) and `tint_index` the model's `tintindex` of the face being drawn.

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

`fn block_color(context: &BlockColorContext) -> Option<u32>`. The colour the provider of `context.state`'s block answers, if the block has one and it answers. The game calls this per face once it can; a test can call it to check a provider.

```rust
use pluto_lib::{BlockColorContext, BlockState, ColorProviderRegistry, Identifier};

ColorProviderRegistry::register_block(&[Identifier::new("example", "tinted_moss").unwrap()], |_| Some(0x336633));
let face = BlockColorContext { state: BlockState::parse("example:tinted_moss").unwrap(), pos: None, tint_index: 0 };
assert_eq!(ColorProviderRegistry::block_color(&face), Some(0x336633));
let other = BlockColorContext { state: BlockState::parse("example:plain_stone").unwrap(), pos: None, tint_index: 0 };
assert_eq!(ColorProviderRegistry::block_color(&other), None);
```
