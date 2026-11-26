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

## Reading Render State and Time

A renderer can read properties from the exact block state submitted for this draw, rather than looking up the world again. It can also read the current render partial tick and level game time. These queries return `None` on custom canvases that do not implement them; missing state is not guessed as a default orientation.

```rust
use pluto_lib::{BlockEntityRendererRegistry, Identifier};
BlockEntityRendererRegistry::register(Identifier::new("example", "oriented_display").unwrap(), |render| {
    if render.canvas.state_property("facing").as_deref() == Some("north") {
        render.canvas.translate([0.5, 0.25, 0.75]);
    }
    let _frame = (render.canvas.partial_tick(), render.canvas.game_time());
});
```

> **Use with Caution.** Native forwarding is source-implemented but not native-compiled/game-verified. `game_time` is a level clock, not a block entity's animation counter: powered skull animation pauses/resumes and cannot be replaced with that clock. These methods add neither a client ticker nor synchronization or reload-safe model caching.

## Registering a Model Layer

Register model layers before the native startup collection, using Fabric's `EntityModelLayerRegistry` naming. Each provider returns named top-level parts under an implicit identity root, plus texture dimensions. Definitions retain box deformation, mirrored UVs, part poses and child ordering; native builders perform the actual baking.

```rust
use pluto_lib::{CubeDefinition, EntityModelLayerRegistry, LayerDefinition, PartDefinition};
EntityModelLayerRegistry::register_model_layer("example:head#main", || {
    let head = PartDefinition {
        cubes: vec![CubeDefinition::new([-4.0, -8.0, -4.0], [8.0; 3], [0, 0])],
        ..PartDefinition::default()
    };
    LayerDefinition::create(vec![("head".into(), head)], 32, 16)
}).unwrap();
```

> **Use with Caution.** Duplicate, vanilla-namespace and late registrations fail without replacing earlier entries. Providers run outside the registry lock, but collection has frozen registration by then. All definitions are validated before the collected batch reaches native registration; zero texture dimensions, duplicate sibling names, non-finite poses/boxes and negative box sizes reject that batch with a diagnostic. Native forwarding appends the validated batch through the pinned host's `LayerDefinitionsEvent`; the full PlutoLib API typecheck passes against the adapted host, but runtime collection/baking and game verification remain pending. Registering a layer does not create a block, an item, a skull renderer or an animation ticker.

## Baking and Animating Parts

A model layer bakes into an owned `ModelPart` tree. Animate its named children, then draw the tree under the canvas pose. `ModelPart` positions are in texels (16 per block) and rotations are radians; these are different units from the canvas's block translations and degree rotations.

```rust
use pluto_lib::{BlockEntityRendererRegistry, Identifier};
BlockEntityRendererRegistry::register(Identifier::new("example", "piglin_display").unwrap(), |render| {
    // For a real renderer, retain the baked tree instead of rebaking each frame.
    if let Some(mut root) = render.canvas.bake_layer("minecraft:piglin_head#main") {
        root.reset_all_poses();
        if let Some(head) = root.get_child_mut("head") {
            head.pose.rotation[1] = 0.5;
        }
        render.canvas.model_part(&root, "entity/piglin/piglin", 0xFFFFFF);
    }
});
```

> **Use with Caution.** Native baking/drawing is wired in source but not yet native-compiled or game-verified under the native attachment gate. Custom canvases return `None`/`false` by default. Baking itself does not register custom layers (use `EntityModelLayerRegistry` before startup), select a skull type, tick client animations or attach a model to armor. The current native adapter copies the tree into native part data at draw time; cache baked trees, and do not assume large animated models are performance-tested. Only RGB tint is exposed here, not a new alpha/overlay contract.

## Choosing an Item Transform

An item worn on a head must use its `Head` display transform, not the `Fixed` transform used by an item frame. `item_with_context` selects the item's resource-model transform inside the canvas's current pose; `item` remains the fixed-frame shorthand.

```rust
use pluto_lib::{BlockEntityRendererRegistry, Identifier, ItemDisplayContext, ItemSlot};

BlockEntityRendererRegistry::register(Identifier::new("example", "display_head").unwrap(), |render| {
    let stack = ItemSlot::new(Identifier::new("minecraft", "carved_pumpkin").unwrap(), 1);
    render.canvas.item_with_context(&stack, ItemDisplayContext::Head);
});
```

> **Use with Caution.** The native forwarding code is implemented in source, including stack damage, maximum damage and custom data, but its next native compilation and game check are pending the Phase native attachment gate. This is not a completed skull renderer: head-layer placement, mesh/texture selection, lighting, tint and animation still need their own integration. Existing custom canvases support only `Fixed` unless they override this method; other contexts return `false`, never a silently substituted fixed-frame draw.

## Drawing What the Server Knows

The renderer sees the block entity's *update tag*, the data the server chose to send (`sync_data` or `update_tag` on the block entity type). A tank that shows its level needs the level in the tag:

```rust
use pluto_lib::{BlockEntityRendererRegistry, Identifier, NbtCompoundExt};

BlockEntityRendererRegistry::register(Identifier::new("example", "gauge").unwrap(), |render| {
    let level = render.data.get_int("level", 0) as f32 / 1000.0;
    render.canvas.cube([0.2, 0.0, 0.2], [0.8, level, 0.8], "example:block/liquid", 0x3366FF);
});
```

> **Use with Caution.** The renderer runs every frame for every block entity in view. Request `keep()` only when the picture depends on saved data, light and resources, not partial ticks or transient animation. PlutoLib coalesces requests and evaluates them after the callback. A registered client ticker (including one registered during that callback) suppresses the hint; an unavailable type registry also refuses caching. Source-declared lookup/revision hooks now track registration generations and committed client data/reset tokens without modifying saved NBT. Cache entries capture the generation before drawing, not after a callback changes registration. Actual owner/constructor/function plan review, native compilation and cache-lifecycle gameplay checks remain pending; this is not verified native invalidation.

### Animated renderers

A client ticker makes rendering frame-dependent. Read this entity's transient data and real interpolation value, and draw without requesting a geometry cache. Optional canvas operations continue to forward through the cache policy wrapper.

```rust
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, BlockEntityRendererRegistry,
    Identifier, Nbt, NbtCompoundExt};
let block = Identifier::parse("example:animated_display").unwrap();
BlockEntityTypeRegistry::register(BlockEntityType::new(block.clone()).client_ticker(|tick| {
    let count = tick.client_data.get_int("ticks", 0) as i32;
    tick.client_data.insert("ticks".into(), Nbt::Int(count.wrapping_add(1)));
}));
BlockEntityRendererRegistry::register(block, |render| {
    let Some(local) = render.canvas.client_data() else { return };
    let count = local.get_int("ticks", 0) as f32;
    let Some(partial) = render.canvas.partial_tick() else { return };
    render.canvas.push();
    render.canvas.rotate_about([0.0, 1.0, 0.0], (count + partial) * 5.0, [0.5; 3]);
    render.canvas.cube([0.2; 3], [0.8; 3], "example:block/display", 0xFFFFFF);
    render.canvas.pop();
    // No keep(): the pose depends on client ticks and interpolation.
});
```

> **Use with Caution.** This example declares callbacks; native tick dispatch and level-transition resets are source-wired but not native-validated. Pure tests verify keep suppression, including late registration inside a renderer, not native cached-entry invalidation or live animation.

## Custom Item and Worn Skulls

A block-entity renderer alone does not select a skull model for inventory, hand or worn-head rendering. `SkullModelRegistry` associates a declared custom item with a model layer, texture sprite and inherited animation family:

```rust
use pluto_lib::{EntityModelLayerRegistry, LayerDefinition, PartDefinition,
    CubeDefinition, Identifier, SkullModelAnimation, SkullModelRegistry};
EntityModelLayerRegistry::register_model_layer("example:skull#main", || {
    let head = PartDefinition { cubes: vec![CubeDefinition::new(
        [-4.0, -8.0, -4.0], [8.0; 3], [0, 0])], ..Default::default() };
    LayerDefinition::create(vec![("head".into(), head)], 64, 32)
}).unwrap();
SkullModelRegistry::register(Identifier::parse("example:skull").unwrap(),
    "example:skull#main", Identifier::parse("example:entity/skull").unwrap(),
    SkullModelAnimation::None).unwrap();
```

The client item definition uses the registered item ID as its custom `minecraft:head` kind, inside a `minecraft:special` model with the target version's skull base and transformation. Worn skull classification uses the same ID independently of item-model overrides. Block-entity renderer registration remains separate.

> **Use with Caution.** Registration, owned definition transport and the actual decoder/parts/bounds/submit/worn helpers are pure-tested with trace host types. Source adapters extend the native special renderer, but native codec behavior, geometry conversion/baking, combined attachment approval, resource reload and rendered pixels remain unverified. Registration creates neither items nor textures. Register before first collection; duplicate, vanilla-item and late registrations reject. Native layers come from the loader's shared entity-model document when provided, otherwise from the current resource root; the document path does not reread the file. Both source loading routes extend custom geometry. The shared-document adapter passes the full fixed-pin plan and diagnostic host/API typechecks; native model baking, reload behavior and rendered pixels remain unverified. Mod layers must exist in EntityModelLayerRegistry. Each ItemRenderer owns fresh model trees, while sprites are looked up for the actual draw. A failed geometry batch publishes no custom prefix; claimed custom models with missing geometry/textures do not draw a vanilla-player substitute. Vanilla kinds and unrelated special renderers retain native fallback.

## Worn Armor Layers

An equipment asset (`assets/<namespace>/equipment/<id>.json`) names one fixed list of layers for every stack of an item. A piece whose look follows its own data (a tool-built piece that takes its textures from its materials) registers an `ArmorRenderer` for its item. The renderer receives the worn stack and the layer set (`humanoid` or `humanoid_leggings`) and answers the layers in draw order. The item still needs an equipment asset (its `equippable.asset_id`); that asset's list is what draws when the renderer declines.

```rust
use pluto_lib::{ArmorLayer, ArmorRenderer, Identifier};

ArmorRenderer::register(&[Identifier::parse("example:suit").unwrap()], |context| {
    let material = context.stack.custom_model_data.as_ref()?.strings.first()?.clone();
    Some(vec![
        ArmorLayer { texture: Identifier::parse("example:suit/base").unwrap(), tint: None },
        ArmorLayer { texture: Identifier::parse(&format!("example:suit/{material}")).unwrap(), tint: Some(0xCC9966) },
    ])
});
```

A layer texture is `namespace:path` for `assets/<namespace>/textures/entity/equipment/<humanoid or humanoid_leggings>/<path>.png`; the game packs every texture under a content namespace's `textures/entity` folder, so no equipment asset has to list it. The glint, trim, OptiFine armour rules and emissive overlays keep working over the answered list.

> **Use with Caution.** Registration, the answer-to-sprite translation and the host-layer decline rules are pure-tested. The worn-armour resolvers of the player, other players, mobs and armor stands are source-declared direct edits (`pluto-armor-worn-layers`, `pluto-armor-stand-worn-layers`) at their layer-list lookup with a bridge module in the item renderer; none of it is native-compiled or seen in a game. Worn wings are not asked. A stack carrying `minecraft:dyed_color` replaces the `tint` of its tinted layers (the game's dyeable-layer rule); untinted layers stay white. The renderer runs every frame for each worn piece: keep it short and allocation-light.

## Reference

> **Use with Caution.** The current native submit route draws `Mod` block entities only. Native-recognized custom names such as `*_head`/`*_skull` become native skull entities before the mod factory is consulted; registering a renderer here alone cannot make those reach this route. The general registered-custom-only factory-priority policy is pure-tested; predicate/hook/guard declarations exist in source, but the guard deliberately rejects unset actual plan fingerprints and native compilation remains unverified. Failed claimed factories do not delegate back to native name classification. The head render core's pure pose tests do not establish native rendering.

### `BlockEntityCanvas`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

The surface a renderer draws on; PlutoLib passes one to the renderer, and a test can implement it to check what a renderer draws.

- Pose: `push()` saves the pose and `pop()` comes back to it; `translate(by)`, `rotate_about(axis, degrees, pivot)` and `scale(by)` change it. Positions are in blocks.
- Geometry: `quad(sprite, corners, uv, tint) -> bool`, `cube(from, to, sprite, tint) -> bool` (one sprite on all six faces), `block(block, properties) -> bool`, `item(slot) -> bool` (an item stack as it looks fixed in a frame), `model(model, tint) -> bool`. They answer false when the sprite, block, item or model does not exist. A `tint` is `0xRRGGBB`.
- `keep()` requests a static geometry cache. Repeated calls are coalesced after the callback, and ignored when the block has a client ticker or the type registry is unavailable. Time-dependent rendering must not request it. A poisoned type or renderer registry makes the registration generation unavailable, so existing entries must not be reused. Native entry-generation and revision adapters exist in source, but real plan/constructor validation and native cache lifecycle checks remain unfinished.

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

### State and frame queries

`BlockEntityCanvas::state_property(&self, name: &str) -> Option<String>` reads a property from the exact submitted state. Unknown properties and unsupported canvases return `None`. The returned string is owned.

`BlockEntityCanvas::partial_tick(&self) -> Option<f32>` reads the native frame interpolation fraction (0–1). `BlockEntityCanvas::game_time(&self) -> Option<i64>` reads the level game time. Both return `None` when unavailable, not fabricated zeroes. These are read-only queries and do not advance any clock or animation.

```rust
use pluto_lib::BlockEntityCanvas;
fn frame_state(canvas: &dyn BlockEntityCanvas) -> (Option<String>, Option<f32>, Option<i64>) {
    (canvas.state_property("rotation"), canvas.partial_tick(), canvas.game_time())
}
```

### Client transient data

`BlockEntityCanvas::client_data(&self) -> Option<&NbtCompound>` reads the transient state of the exact client block entity being drawn. It is not the server's saved or synchronized `BlockEntityRender::data`. Unsupported or busy canvases return `None`; do not substitute server NBT or a global clock. The native source adapter now forwards a committed snapshot from the exact entity's declared transient field, but its state/constructor plan and native compilation are unverified. Client tick dispatch and lifetime/gameplay verification remain incomplete; source forwarding is not working animation.

```rust
use pluto_lib::{BlockEntityCanvas, NbtCompoundExt};
fn local_animation(canvas: &dyn BlockEntityCanvas) -> Option<i64> {
    let data = canvas.client_data()?;
    Some(data.get_int("animation_ticks", 0))
}
```

### `CubeDefinition`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

An unbaked box with public `origin`, `dimensions`, `texture_offset`, `deformation` and `mirror` fields. Geometry/deformation uses texels; the texture offset is pixels. `CubeDefinition::new` starts undeformed and unmirrored. Negative dimensions and non-finite geometry reject at startup collection; zero-sized planes are permitted.

```rust
use pluto_lib::CubeDefinition;
let mut hat = CubeDefinition::new([-4.0, -8.0, -4.0], [8.0; 3], [0, 16]);
hat.deformation = [0.25; 3];
hat.mirror = true;
assert_eq!(hat.texture_offset, [0, 16]);
```

### `CubeDefinition::new`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`new(origin: [f32; 3], dimensions: [f32; 3], texture_offset: [i32; 2]) -> CubeDefinition` constructs a box with zero deformation and `mirror=false`.

```rust
use pluto_lib::CubeDefinition;
let cube = CubeDefinition::new([0.0; 3], [4.0, 8.0, 2.0], [0, 0]);
assert_eq!(cube.deformation, [0.0; 3]);
assert!(!cube.mirror);
```

### `PartDefinition`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

An unbaked part with public `pose: PartPose`, `cubes: Vec<CubeDefinition>` and ordered `children: Vec<(String, PartDefinition)>`. `Default` has the identity pose and no geometry/children. Names must be unique within each sibling list at collection.

```rust
use pluto_lib::{PartDefinition, PartPose};
let part = PartDefinition { pose: PartPose { translation: [0.0, 4.0, 0.0], ..PartPose::ZERO }, ..PartDefinition::default() };
assert!(part.children.is_empty());
```

### `PartDefinition::add_or_replace_child`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`add_or_replace_child(&mut self, name: &str, cubes: Vec<CubeDefinition>, pose: PartPose) -> &mut PartDefinition` appends a new child or replaces an existing child's boxes/pose, preserving its children and its position in draw order.

```rust
use pluto_lib::{PartDefinition, PartPose};
let mut root = PartDefinition::default();
root.add_or_replace_child("head", vec![], PartPose::ZERO)
    .add_or_replace_child("hat", vec![], PartPose::ZERO);
let head = root.add_or_replace_child("head", vec![], PartPose::ZERO);
assert_eq!(head.children[0].0, "hat");
assert_eq!(root.children.len(), 1);
```

### `LayerDefinition`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`LayerDefinition { parts: Vec<(String, PartDefinition)>, texture_size: [u32; 2] }` defines a layer with an implicit identity root and its ordered named parts. Both texture dimensions must be positive. Baking normalizes box UVs using these dimensions, not the canvas quad helper's UV units.

```rust
use pluto_lib::LayerDefinition;
let definition = LayerDefinition { parts: vec![], texture_size: [64, 32] };
assert_eq!(definition.texture_size, [64, 32]);
```

### `LayerDefinition::create`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`create(parts: Vec<(String, PartDefinition)>, width: u32, height: u32) -> LayerDefinition` owns the parts and dimensions; validation occurs when providers are collected at startup.

```rust
use pluto_lib::LayerDefinition;
let definition = LayerDefinition::create(vec![], 32, 16);
assert_eq!(definition.texture_size, [32, 16]);
```

### `SkullModelAnimation`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: absent](https://img.shields.io/badge/Sunrise-absent-lightgrey)

`None`, `Piglin` and `Dragon` select the inherited rigid-head, piglin-ear or vanilla-dragon-jaw pose algorithm. Animation comes from the requesting special model or worn-head caller, not a global clock. A custom armor wrapper is not implied.

```rust
use pluto_lib::SkullModelAnimation;
let families = [SkullModelAnimation::None, SkullModelAnimation::Piglin, SkullModelAnimation::Dragon];
assert_ne!(families[0], families[1]);
```

### `SkullModelRegistry`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

Associates custom item IDs with skull model metadata before client startup. It is independent of BlockEntityRendererRegistry and does not declare an item or a model layer.

```rust
use pluto_lib::{Identifier, SkullModelAnimation, SkullModelRegistry};
SkullModelRegistry::register(Identifier::parse("example:bone_head").unwrap(),
    "minecraft:skeleton_skull#main", Identifier::parse("minecraft:entity/skeleton/skeleton").unwrap(),
    SkullModelAnimation::None).unwrap();
```

### `SkullModelRegistry::register`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn register(item: Identifier, layer: &str, texture: Identifier, animation: SkullModelAnimation) -> Result<(), String>`. The custom item ID doubles as the special-model kind; `layer` has `namespace:path#layer` form and names a native layer or a registered mod layer. `texture` is the full sprite ID, not a `textures/` path or `.png` filename. Native item membership, model attachment and pixels are not validated by registration. Duplicate/late/vanilla-item IDs and invalid layer/texture forms reject without replacing an entry. For a registered custom special model, an optional JSON `texture` follows the native entity-relative convention, preserving custom namespaces.

```rust
use pluto_lib::{Identifier, SkullModelAnimation, SkullModelRegistry};
let item = Identifier::parse("example:ears").unwrap();
let texture = Identifier::parse("example:entity/ears").unwrap();
SkullModelRegistry::register(item.clone(), "example:ears#main", texture.clone(), SkullModelAnimation::Piglin).unwrap();
assert!(SkullModelRegistry::register(item, "example:other#main", texture, SkullModelAnimation::None).is_err());
```

### `EntityModelLayerRegistry`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

The Fabric-style registry for native model layers. Register providers during mod initialization, before the one-time native collection. It does not replace vanilla layers or other mods' layers.

```rust
use pluto_lib::{EntityModelLayerRegistry, LayerDefinition};
EntityModelLayerRegistry::register_model_layer("example:empty#main", || LayerDefinition::create(vec![], 16, 16)).unwrap();
```

### `EntityModelLayerRegistry::register_model_layer`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`register_model_layer(id: &str, provider: impl Fn() -> LayerDefinition + Send + Sync + 'static) -> Result<(), String>` registers a provider. `id` is `namespace:path#layer`, with a non-vanilla identifier and a nonempty layer containing neither `#` nor `:`. Duplicate, invalid, poisoned-registry and post-collection registrations return an error without mutation. Providers are snapshotted in ID order and invoked without holding the registry lock; invalid provider output rejects the whole PlutoLib batch before native output is extended.

```rust
use pluto_lib::{EntityModelLayerRegistry, LayerDefinition};
assert!(EntityModelLayerRegistry::register_model_layer("minecraft:head#main", || LayerDefinition::create(vec![], 32, 16)).is_err());
EntityModelLayerRegistry::register_model_layer("example:layer#main", || LayerDefinition::create(vec![], 32, 16)).unwrap();
assert!(EntityModelLayerRegistry::register_model_layer("example:layer#main", || LayerDefinition::create(vec![], 64, 32)).is_err());
```

### `PartPose`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`PartPose { translation: [f32; 3], rotation: [f32; 3], scale: [f32; 3] }` describes a part relative to its parent. Translation is in texels; X/Y/Z rotation components are radians, applied Z then Y then X, before scaling. `ZERO` and `Default` have zero translation/rotation and unit scale.

```rust
use pluto_lib::PartPose;
let pose = PartPose { translation: [0.0, 4.0, -8.0], rotation: [0.2, 0.0, 0.0], scale: [1.0; 3] };
assert_eq!(PartPose::default(), PartPose::ZERO);
assert_eq!(pose.translation[1], 4.0);
```

### `ModelVertex`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`ModelVertex { pos: [f32; 3], uv: [f32; 2] }` is a model-space vertex in texels with normalized texture coordinates. UVs are not the canvas quad helper's 0–16 texture coordinates.

```rust
use pluto_lib::ModelVertex;
let vertex = ModelVertex { pos: [-4.0, -8.0, -4.0], uv: [0.25, 0.5] };
assert_eq!(vertex.uv, [0.25, 0.5]);
```

### `ModelPolygon`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`ModelPolygon { vertices: [ModelVertex; 4], normal: [f32; 3] }` is one ordered face. The adapter preserves vertex order, UVs and model-space normal.

```rust
use pluto_lib::{ModelPolygon, ModelVertex};
let vertices = [
    ModelVertex { pos: [0.0, 0.0, 0.0], uv: [0.0, 0.0] },
    ModelVertex { pos: [16.0, 0.0, 0.0], uv: [1.0, 0.0] },
    ModelVertex { pos: [16.0, 16.0, 0.0], uv: [1.0, 1.0] },
    ModelVertex { pos: [0.0, 16.0, 0.0], uv: [0.0, 1.0] },
];
let face = ModelPolygon { vertices, normal: [0.0, 0.0, 1.0] };
assert_eq!(face.vertices[2].pos, [16.0, 16.0, 0.0]);
```

### `ModelPart`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`ModelPart::new(polygons: Vec<ModelPolygon>, children: Vec<(String, ModelPart)>, pose: PartPose)` takes ownership of its faces and ordered named children and remembers the initial pose. Public fields `pose`, `visible`, `skip_draw`, `polygons` and `children` are editable. `visible=false` hides a whole subtree; `skip_draw=true` hides only that part's faces. Cloning makes an independent tree.

`get_child(name: &str) -> Option<&ModelPart>` and `get_child_mut(name: &str) -> Option<&mut ModelPart>` search immediate children, returning the first matching name or `None`. `reset_pose()` restores only this part's pose; `reset_all_poses()` also restores every descendant. Neither changes visibility, geometry or names.

```rust
use pluto_lib::{ModelPart, PartPose};
let jaw = ModelPart::new(vec![], vec![], PartPose { translation: [0.0, 4.0, -8.0], ..PartPose::ZERO });
let mut head = ModelPart::new(vec![], vec![("jaw".into(), jaw)], PartPose::ZERO);
let other = head.clone();
head.get_child_mut("jaw").unwrap().pose.rotation[0] = 0.4;
head.pose.rotation[1] = 0.7;
head.reset_pose();
assert_eq!(head.get_child("jaw").unwrap().pose.rotation[0], 0.4);
assert_eq!(other.get_child("jaw").unwrap().pose.rotation[0], 0.0);
head.reset_all_poses();
assert_eq!(head.get_child("jaw").unwrap().pose.rotation[0], 0.0);
assert!(head.get_child("missing").is_none());
```

### `PartPose::ZERO`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`ZERO: PartPose` is zero translation/rotation and unit scale, also returned by `Default`.

```rust
use pluto_lib::PartPose;
assert_eq!(PartPose::ZERO.scale, [1.0; 3]);
```

### `ModelPart::new`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`new(polygons: Vec<ModelPolygon>, children: Vec<(String, ModelPart)>, pose: PartPose) -> ModelPart` owns its ordered geometry/children and remembers `pose` for reset. It starts visible and does not skip its own drawing.

```rust
use pluto_lib::{ModelPart, PartPose};
let part = ModelPart::new(vec![], vec![], PartPose::ZERO);
assert!(part.visible && !part.skip_draw);
```

### `ModelPart::get_child`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`get_child(&self, name: &str) -> Option<&ModelPart>` finds the first immediate child of that name, not a recursive descendant; missing names return `None`.

```rust
use pluto_lib::{ModelPart, PartPose};
let part = ModelPart::new(vec![], vec![], PartPose::ZERO);
assert!(part.get_child("jaw").is_none());
```

### `ModelPart::get_child_mut`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`get_child_mut(&mut self, name: &str) -> Option<&mut ModelPart>` mutably borrows the first matching immediate child for animation or editing.

```rust
use pluto_lib::{ModelPart, PartPose};
let child = ModelPart::new(vec![], vec![], PartPose::ZERO);
let mut root = ModelPart::new(vec![], vec![("jaw".into(), child)], PartPose::ZERO);
root.get_child_mut("jaw").unwrap().pose.rotation[0] = 0.2;
assert_eq!(root.get_child("jaw").unwrap().pose.rotation[0], 0.2);
```

### `ModelPart::reset_pose`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`reset_pose(&mut self)` restores this part's constructor/baked pose, not its children's poses or its visibility/geometry.

```rust
use pluto_lib::{ModelPart, PartPose};
let mut part = ModelPart::new(vec![], vec![], PartPose::ZERO);
part.pose.rotation[1] = 1.0;
part.visible = false;
part.reset_pose();
assert_eq!(part.pose, PartPose::ZERO);
assert!(!part.visible);
```

### `ModelPart::reset_all_poses`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`reset_all_poses(&mut self)` restores this part and every descendant to their own initial poses, retaining all visibility flags, geometry and names.

```rust
use pluto_lib::{ModelPart, PartPose};
let child = ModelPart::new(vec![], vec![], PartPose::ZERO);
let mut root = ModelPart::new(vec![], vec![("jaw".into(), child)], PartPose::ZERO);
root.get_child_mut("jaw").unwrap().pose.rotation[0] = 0.4;
root.reset_all_poses();
assert_eq!(root.get_child("jaw").unwrap().pose, PartPose::ZERO);
```

### Baking and drawing a part tree

`BlockEntityCanvas::bake_layer(&mut self, layer: &str) -> Option<ModelPart>` takes an id such as `minecraft:piglin_head#main`. It returns a new owned tree for an available vanilla or already native-registered layer, otherwise `None`. It does not itself register a layer. Bake once per renderer/resource snapshot and invalidate the cache on resource reload.

`BlockEntityCanvas::model_part(&mut self, part: &ModelPart, sprite: &str, tint: u32) -> bool` draws that tree using an entity-texture sprite and `0xRRGGBB` tint under the current canvas pose and lighting. It returns `false` if unsupported or the sprite is missing; no fallback cube or item is drawn. Custom canvases override these methods to support them.

```rust
use pluto_lib::BlockEntityCanvas;
fn draw_skull(canvas: &mut dyn BlockEntityCanvas) -> bool {
    let Some(root) = canvas.bake_layer("minecraft:skeleton_skull#main") else { return false };
    canvas.model_part(&root, "entity/skeleton/skeleton", 0xFFFFFF)
}
```

### `ItemDisplayContext`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

A copyable enum selecting the resource-model transform. Its ten forms are `None`, `ThirdPersonLeftHand`, `ThirdPersonRightHand`, `FirstPersonLeftHand`, `FirstPersonRightHand`, `Head`, `Gui`, `Ground`, `Fixed` and `OnShelf`. `None` selects the model's no-display-context form; it does not suppress drawing. This type does not position an item on an entity or open a screen.

```rust
use pluto_lib::ItemDisplayContext;
let contexts = [
    ItemDisplayContext::None,
    ItemDisplayContext::ThirdPersonLeftHand,
    ItemDisplayContext::ThirdPersonRightHand,
    ItemDisplayContext::FirstPersonLeftHand,
    ItemDisplayContext::FirstPersonRightHand,
    ItemDisplayContext::Head,
    ItemDisplayContext::Gui,
    ItemDisplayContext::Ground,
    ItemDisplayContext::Fixed,
    ItemDisplayContext::OnShelf,
];
assert_eq!(contexts.len(), 10);
```

### Drawing an item with a selected context

`item_with_context(&mut self, item: &ItemSlot, context: ItemDisplayContext) -> bool` draws a stack using the chosen display transform under the current canvas pose. The native source adapter preserves the slot's count, damage, maximum damage and custom data. The result is the backend's draw result; a missing item/model or an unsupported context returns `false`. Custom canvases inherit a default that calls `item` only for `Fixed` and rejects every other context.

```rust
use pluto_lib::{BlockEntityCanvas, ItemDisplayContext, ItemSlot};
fn draw_on_head(canvas: &mut dyn BlockEntityCanvas, stack: &ItemSlot) -> bool {
    canvas.item_with_context(stack, ItemDisplayContext::Head)
}
```

### `BlockEntityRender`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

What a renderer receives for one block entity. `pos` is its position, `block` the block's id, `data` what the server sent of it, `items` its item slots (as many as its type declared, an empty slot `None`; the slots live outside the sent data, so a renderer that draws the contents reads them here), `canvas` where to draw.

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
draw_level(&mut BlockEntityRender { pos: [0, 64, 0], block: "example:tank", data: &data, items: &[], canvas: &mut Quiet });
```

### `BlockEntityRendererRegistry`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

Registers renderers (Fabric: `BlockEntityRendererFactories.register`).

```rust
use pluto_lib::{BlockEntityRendererRegistry, Identifier};

BlockEntityRendererRegistry::register(Identifier::new("example", "lamp").unwrap(), |render| {
    render.canvas.cube([0.25, 0.0, 0.25], [0.75, 0.5, 0.75], "example:block/lamp", 0xFFEEAA);
    render.canvas.keep();
});
```

### `BlockEntityRendererRegistry::register`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn register(block: Identifier, renderer: impl Fn(&mut BlockEntityRender<'_>) + Send + Sync + 'static)`. Draws every block entity of `block` with `renderer`. Registering the same block again replaces the earlier renderer. Keep the renderer short: it runs for every block entity in view, every frame, unless a static `keep()` request is accepted. Client-ticker renderers never forward that hint; hint eligibility is checked after the callback.

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

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn has(block: &str) -> bool`. Whether `block` (an id such as `example:lamp`) has a renderer.

```rust
use pluto_lib::{BlockEntityRendererRegistry, Identifier};

assert!(!BlockEntityRendererRegistry::has("example:undrawn"));
BlockEntityRendererRegistry::register(Identifier::new("example", "undrawn").unwrap(), |_| {});
assert!(BlockEntityRendererRegistry::has("example:undrawn"));
```

### `ArmorLayerKind`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

The layer set a renderer is asked for: `Humanoid` (helmet, chestplate, boots) or `HumanoidLeggings`. The texture folder under `textures/entity/equipment` has the same name.

```rust
use pluto_lib::ArmorLayerKind;
assert_ne!(ArmorLayerKind::Humanoid, ArmorLayerKind::HumanoidLeggings);
```

### `ArmorRenderContext`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

What a renderer is asked: `stack` is the worn stack as the item renderer sees it (custom data, model data, damage) and `kind` the layer set being drawn.

```rust
use pluto_lib::{ArmorLayerKind, ArmorRenderContext, StackData};
let context = ArmorRenderContext { stack: StackData { item: "example:suit".into(), count: 1, ..Default::default() }, kind: ArmorLayerKind::Humanoid };
assert_eq!(context.stack.item, "example:suit");
assert_eq!(context.kind, ArmorLayerKind::Humanoid);
```

### `ArmorLayer`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

One drawn layer: `texture` names the equipment texture as an equipment asset does (`namespace:path`, no `textures/` and no `.png`; such a name makes the game keep the equipment asset's own layers) and `tint` is `Some(0xRRGGBB)` to multiply into it or `None`.

```rust
use pluto_lib::{ArmorLayer, Identifier};
let layer = ArmorLayer { texture: Identifier::parse("example:suit/base").unwrap(), tint: Some(0x336699) };
assert_eq!(layer.texture.path(), "suit/base");
assert_eq!(layer.tint, Some(0x336699));
```

### `ArmorRenderer`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

The Fabric-style registry of worn armor renderers (`ArmorRenderer.register`). It does not declare an item, an equipment asset or a texture.

```rust
use pluto_lib::{ArmorRenderer, Identifier};
ArmorRenderer::register(&[Identifier::parse("example:plain_suit").unwrap()], |_| None);
```

### `ArmorRenderer::register`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn register(items: &[Identifier], renderer: impl Fn(&ArmorRenderContext) -> Option<Vec<ArmorLayer>> + Send + Sync + 'static)` gives each item the renderer. A later registration replaces an earlier one. The renderer returns the layers in draw order or `None` to leave the equipment asset; it may register renderers itself.

```rust
use pluto_lib::{ArmorLayer, ArmorRenderer, Identifier};
let item = Identifier::parse("example:cloak").unwrap();
ArmorRenderer::register(&[item], |_| Some(vec![ArmorLayer { texture: Identifier::parse("example:cloak/plain").unwrap(), tint: None }]));
```

### `ArmorRenderer::layers`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn layers(context: &ArmorRenderContext) -> Option<Vec<ArmorLayer>>` asks the stack item's renderer outside the registry lock: `None` for no registration or a decline. The game's adapter calls the same lookup.

```rust
use pluto_lib::{ArmorLayer, ArmorLayerKind, ArmorRenderContext, ArmorRenderer, Identifier, StackData};
ArmorRenderer::register(&[Identifier::parse("example:mantle").unwrap()], |context| {
    (context.kind == ArmorLayerKind::Humanoid).then(|| vec![ArmorLayer { texture: Identifier::parse("example:mantle/top").unwrap(), tint: None }])
});
let ask = |kind| ArmorRenderContext { stack: StackData { item: "example:mantle".into(), count: 1, ..Default::default() }, kind };
assert_eq!(ArmorRenderer::layers(&ask(ArmorLayerKind::Humanoid)).unwrap().len(), 1);
assert_eq!(ArmorRenderer::layers(&ask(ArmorLayerKind::HumanoidLeggings)), None);
```
