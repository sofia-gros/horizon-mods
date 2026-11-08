# Drawing block entities

A block entity renderer runs on the client once a frame for each block entity of its block that is in view (within 64 blocks and in the frustum) and adds geometry. It draws under a pose stack that starts at the block's lower north-west corner, in blocks, and sees the data the server sent of the block entity (see `BlockEntityType::update_tag` and `sync_data` in `block-entity.md`; without them the data is empty on the client).

## Basic Example

Draw a glowing lamp on top of a block entity's block:

```rust
use pluto_lib::{BlockEntityRendererRegistry, Identifier};

BlockEntityRendererRegistry::register(Identifier::new("example", "lamp").unwrap(), |render| {
    render.canvas.cube([0.3, 1.0, 0.3], [0.7, 1.4, 0.7], "example:block/lamp", 0xFFEEAA);
    render.canvas.keep();
});
```

The renderer runs on the client once a frame for each block entity of the block in view (within 64 blocks). Positions are in blocks from the block's lower north-west corner.

## Moving and Turning

`push` and `pop` save and restore the pose, so a part can be moved and turned without disturbing the next:

```rust
use pluto_lib::{BlockEntityRendererRegistry, Identifier};

BlockEntityRendererRegistry::register(Identifier::new("example", "fan").unwrap(), |render| {
    render.canvas.push();
    render.canvas.translate([0.5, 0.5, 0.5]);
    render.canvas.rotate_about([0.0, 1.0, 0.0], 45.0, [0.0, 0.0, 0.0]);
    render.canvas.cube([-0.4, -0.05, -0.05], [0.4, 0.05, 0.05], "example:block/blade", 0xFFFFFF);
    render.canvas.pop();
});
```

## Drawing What the Server Knows

The renderer sees the block entity's *update tag*, the data the server chose to send (`sync_data` or `update_tag` on the block entity type). A tank that shows its level needs the level in the tag:

```rust
use pluto_lib::{BlockEntityRendererRegistry, Identifier, NbtCompoundExt};

BlockEntityRendererRegistry::register(Identifier::new("example", "gauge").unwrap(), |render| {
    let level = render.data.get_int("level", 0) as f32 / 1000.0;
    render.canvas.cube([0.2, 0.0, 0.2], [0.8, level, 0.8], "example:block/liquid", 0x3366FF);
});
```

> **Use with Caution.** The renderer runs every frame for every block entity in view. Call `keep()` when the picture only changes with the block entity's data, so an idle machine costs nothing per frame.

## Reference

### `BlockEntityCanvas`

The surface a renderer draws on; PlutoLib passes one to the renderer, and a test can implement it to check what a renderer draws.

- Pose: `push()` saves the pose and `pop()` comes back to it; `translate(by)`, `rotate_about(axis, degrees, pivot)` and `scale(by)` change it. Positions are in blocks.
- Geometry: `quad(sprite, corners, uv, tint) -> bool`, `cube(from, to, sprite, tint) -> bool` (one sprite on all six faces), `block(block, properties) -> bool`, `item(slot) -> bool` (an item stack as it looks fixed in a frame), `model(model, tint) -> bool`. They answer false when the sprite, block, item or model does not exist. A `tint` is `0xRRGGBB`.
- `keep()` keeps what was drawn until the block entity, its light or the resource packs change, so an idle machine costs nothing per frame.

```rust
use pluto_lib::{BlockEntityCanvas, ItemSlot};

/// A canvas that only counts what is drawn.
#[derive(Default)]
struct Counting(u32);
impl BlockEntityCanvas for Counting {
    fn push(&mut self) {}
    fn pop(&mut self) {}
    fn translate(&mut self, _: [f32; 3]) {}
    fn rotate_about(&mut self, _: [f32; 3], _: f32, _: [f32; 3]) {}
    fn scale(&mut self, _: [f32; 3]) {}
    fn quad(&mut self, _: &str, _: [[f32; 3]; 4], _: [f32; 4], _: u32) -> bool { self.0 += 1; true }
    fn cube(&mut self, _: [f32; 3], _: [f32; 3], _: &str, _: u32) -> bool { self.0 += 1; true }
    fn block(&mut self, _: &str, _: &[(&str, &str)]) -> bool { self.0 += 1; true }
    fn item(&mut self, _: &ItemSlot) -> bool { self.0 += 1; true }
    fn model(&mut self, _: &str, _: u32) -> bool { false }
    fn keep(&mut self) {}
}
let mut canvas = Counting::default();
canvas.cube([0.0; 3], [1.0; 3], "example:block/glass", 0xFFFFFF);
assert_eq!(canvas.0, 1);
assert!(!canvas.model("example:block/gear", 0xFFFFFF));
```

### `BlockEntityRender`

What a renderer receives for one block entity. `pos` is its position, `block` the block's id, `data` what the server sent of it, `canvas` where to draw.

```rust
use pluto_lib::{BlockEntityCanvas, BlockEntityRender, ItemSlot, NbtCompound};

struct Quiet;
impl BlockEntityCanvas for Quiet {
    fn push(&mut self) {}
    fn pop(&mut self) {}
    fn translate(&mut self, _: [f32; 3]) {}
    fn rotate_about(&mut self, _: [f32; 3], _: f32, _: [f32; 3]) {}
    fn scale(&mut self, _: [f32; 3]) {}
    fn quad(&mut self, _: &str, _: [[f32; 3]; 4], _: [f32; 4], _: u32) -> bool { true }
    fn cube(&mut self, _: [f32; 3], _: [f32; 3], _: &str, _: u32) -> bool { true }
    fn block(&mut self, _: &str, _: &[(&str, &str)]) -> bool { true }
    fn item(&mut self, _: &ItemSlot) -> bool { true }
    fn model(&mut self, _: &str, _: u32) -> bool { true }
    fn keep(&mut self) {}
}
fn draw_level(render: &mut BlockEntityRender<'_>) {
    let _above = render.pos[1] + 1;
    if render.block == "example:tank" && render.data.contains_key("level") {
        render.canvas.cube([0.1, 0.0, 0.1], [0.9, 0.5, 0.9], "example:block/liquid", 0x3366FF);
    }
}
let mut data = NbtCompound::new();
data.insert("level".to_owned(), pluto_lib::Nbt::Int(500));
draw_level(&mut BlockEntityRender { pos: [0, 64, 0], block: "example:tank", data: &data, canvas: &mut Quiet });
```

### `BlockEntityRendererRegistry`

Registers renderers (Fabric: `BlockEntityRendererFactories.register`).

```rust
use pluto_lib::{BlockEntityRendererRegistry, Identifier};

BlockEntityRendererRegistry::register(Identifier::new("example", "lamp").unwrap(), |render| {
    render.canvas.cube([0.25, 0.0, 0.25], [0.75, 0.5, 0.75], "example:block/lamp", 0xFFEEAA);
    render.canvas.keep();
});
```

### `BlockEntityRendererRegistry::register`

`fn register(block: Identifier, renderer: impl Fn(&mut BlockEntityRender<'_>) + Send + Sync + 'static)`. Draws every block entity of `block` with `renderer`. Registering the same block again replaces the earlier renderer. Keep the renderer short: it runs for every block entity in view, every frame, unless it calls `keep()`.

```rust
use pluto_lib::{BlockEntityRendererRegistry, Identifier, NbtCompoundExt};

BlockEntityRendererRegistry::register(Identifier::new("example", "gauge").unwrap(), |render| {
    let level = render.data.get_int("level", 0) as f32 / 1000.0;
    render.canvas.push();
    render.canvas.translate([0.0, 1.0, 0.0]);
    render.canvas.scale([1.0, level.max(0.01), 1.0]);
    render.canvas.cube([0.4, 0.0, 0.4], [0.6, 1.0, 0.6], "example:block/bar", 0xFF3030);
    render.canvas.pop();
});
```

### `BlockEntityRendererRegistry::has`

`fn has(block: &str) -> bool`. Whether `block` (an id such as `example:lamp`) has a renderer.

```rust
use pluto_lib::{BlockEntityRendererRegistry, Identifier};

assert!(!BlockEntityRendererRegistry::has("example:undrawn"));
BlockEntityRendererRegistry::register(Identifier::new("example", "undrawn").unwrap(), |_| {});
assert!(BlockEntityRendererRegistry::has("example:undrawn"));
```
