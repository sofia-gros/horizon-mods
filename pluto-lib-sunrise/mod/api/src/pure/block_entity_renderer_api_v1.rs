//! Drawing a block entity (Minecraft: `BlockEntityRenderer`; Fabric: `BlockEntityRendererFactories.register`). The renderer runs on the
//! client once a frame for each block entity of the block in view and adds geometry under a pose stack that starts at the block's lower
//! north-west corner, in blocks. It sees the data the server sent (`BlockEntityType::update_tag` / `sync_data`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::base::{Identifier, NbtCompound};
use super::block_entity_api_v1::{BlockPos, ItemSlot};
#[path = "model_part.rs"]
mod model_part;
pub use model_part::{ModelPart, ModelPolygon, ModelVertex, PartPose};
#[path = "model_layer.rs"]
mod model_layer;
pub use model_layer::{CubeDefinition, PartDefinition, LayerDefinition, EntityModelLayerRegistry};
pub(crate) use model_layer::{definitions as model_layer_definitions, definitions_for as model_layer_definitions_for};

/// The model transform used when drawing an item (Minecraft: `ItemDisplayContext`).
/// This selects the item's resource-model transform, not the surrounding canvas pose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemDisplayContext {
    None,
    ThirdPersonLeftHand,
    ThirdPersonRightHand,
    FirstPersonLeftHand,
    FirstPersonRightHand,
    Head,
    Gui,
    Ground,
    Fixed,
    OnShelf,
}

/// The surface a block entity renderer draws on. PlutoLib passes one to the renderer; a test can implement it to check what a renderer
/// draws. Positions are in blocks under the current pose; a `tint` is `0xRRGGBB`.
pub trait BlockEntityCanvas {
    /// A property of the exact state being drawn, not a second world lookup.
    /// None if absent or unavailable on a custom canvas.
    fn state_property(&self, _name: &str) -> Option<String> { None }
    /// Read-only transient data of this client entity, distinct from server NBT.
    /// None if instance storage/transport is unsupported or temporarily busy.
    /// Native snapshot forwarding exists in source, not yet native-validated.
    fn client_data(&self) -> Option<&NbtCompound> { None }
    /// Current render interpolation fraction. None on unsupported canvases.
    fn partial_tick(&self) -> Option<f32> { None }
    /// Current level game time. This is not a block entity's animation counter.
    fn game_time(&self) -> Option<i64> { None }
    /// Saves the pose (position, rotation, scale) to come back to with [`pop`](Self::pop).
    fn push(&mut self);
    /// Comes back to the pose of the matching [`push`](Self::push).
    fn pop(&mut self);
    /// Moves the pose by `by` blocks.
    fn translate(&mut self, by: [f32; 3]);
    /// Rotates the pose by `degrees` about `axis` through `pivot`.
    fn rotate_about(&mut self, axis: [f32; 3], degrees: f32, pivot: [f32; 3]);
    /// Scales the pose.
    fn scale(&mut self, by: [f32; 3]);
    /// Draws one quad with the four `corners` and the part `uv` (`[u0, v0, u1, v1]`, in texels of 16 to a side) of the texture `sprite`
    /// (`mymod:block/casing`); false if there is no such sprite.
    fn quad(&mut self, sprite: &str, corners: [[f32; 3]; 4], uv: [f32; 4], tint: u32) -> bool;
    /// Draws a box from `from` to `to` with `sprite` on all six faces; false if there is no such sprite.
    fn cube(&mut self, from: [f32; 3], to: [f32; 3], sprite: &str, tint: u32) -> bool;
    /// Draws a block (`minecraft:glass`) with the given block state properties; false if there is no such block.
    fn block(&mut self, block: &str, properties: &[(&str, &str)]) -> bool;
    /// Draws an item stack as it looks fixed in a frame; false if it cannot be drawn.
    fn item(&mut self, item: &ItemSlot) -> bool;
    /// Draws a stack with the chosen item-model display transform. Custom canvases
    /// that only implement `item` support Fixed; other contexts return false rather
    /// than silently drawing the wrong transform. The native canvas handles all contexts.
    fn item_with_context(&mut self, item: &ItemSlot, context: ItemDisplayContext) -> bool {
        context == ItemDisplayContext::Fixed && self.item(item)
    }
    /// Draws a model (`mymod:block/gear`) the mod registered; false if there is none.
    fn model(&mut self, model: &str, tint: u32) -> bool;
    /// Bakes an owned part tree from a native registered or vanilla model layer,
    /// e.g. `minecraft:piglin_head#main`. None when unavailable or unsupported.
    /// Cache the returned tree in the renderer rather than baking every frame.
    fn bake_layer(&mut self, _layer: &str) -> Option<ModelPart> { None }
    /// Draws a part tree under this canvas's current pose, with an entity-texture
    /// sprite and RGB tint. Part translations/vertices are in texels, rotations
    /// in radians; visibility and skip_draw retain their distinct meanings.
    /// Custom canvases without model-part support return false without drawing.
    fn model_part(&mut self, _part: &ModelPart, _sprite: &str, _tint: u32) -> bool { false }
    /// Requests geometry caching for static rendering. PlutoLib coalesces hints
    /// and checks them after the callback; a registered client ticker or an
    /// unavailable type registry suppresses the request. Generation/token
    /// adapters for existing native entries are source-wired but unvalidated.
    fn keep(&mut self);
}

/// What a renderer receives for one block entity.
pub struct BlockEntityRender<'a> {
    /// The block entity's position.
    pub pos: BlockPos,
    /// The block's id.
    pub block: &'a str,
    /// What the server sent of the block entity (its update tag).
    pub data: &'a NbtCompound,
    /// The block entity's item slots, as many as its type declared (an empty slot is `None`); empty when the type declared none.
    /// The slots live outside the saved data, so a renderer that draws the contents (a melter's melting items, a table's cast)
    /// reads them here.
    pub items: &'a [Option<ItemSlot>],
    /// Where to draw.
    pub canvas: &'a mut dyn BlockEntityCanvas,
}

type Renderer = Arc<dyn Fn(&mut BlockEntityRender<'_>) + Send + Sync>;

static RENDERERS: Mutex<Option<HashMap<String, Renderer>>> = Mutex::new(None);
pub(crate) fn registry_available() -> bool { !RENDERERS.is_poisoned() }

/// Registers block entity renderers (Fabric: `BlockEntityRendererFactories.register`).
pub struct BlockEntityRendererRegistry;

impl BlockEntityRendererRegistry {
    /// Draws every block entity of `block` with `renderer`. Registering the same block again replaces the earlier renderer.
    pub fn register(block: Identifier, renderer: impl Fn(&mut BlockEntityRender<'_>) + Send + Sync + 'static) {
        if let Ok(mut all) = RENDERERS.lock() {
            all.get_or_insert_with(HashMap::new).insert(block.to_string(), Arc::new(renderer));
            super::render_cache::changed();
        }
    }

    /// Whether `block` has a renderer.
    pub fn has(block: &str) -> bool {
        RENDERERS.lock().ok().and_then(|all| all.as_ref().map(|m| m.contains_key(block))).unwrap_or(false)
    }
}

/// Defer keep hints until the complete callback has finished: a callback may
/// register a client ticker after asking to keep its geometry. All optional
/// canvas operations must forward too, rather than falling back to trait stubs.
struct CacheCanvas<'a> {
    inner: &'a mut dyn BlockEntityCanvas,
    keep_requested: bool,
}
impl BlockEntityCanvas for CacheCanvas<'_> {
    fn state_property(&self, name: &str) -> Option<String> { self.inner.state_property(name) }
    fn client_data(&self) -> Option<&NbtCompound> { self.inner.client_data() }
    fn partial_tick(&self) -> Option<f32> { self.inner.partial_tick() }
    fn game_time(&self) -> Option<i64> { self.inner.game_time() }
    fn push(&mut self) { self.inner.push(); }
    fn pop(&mut self) { self.inner.pop(); }
    fn translate(&mut self, by: [f32; 3]) { self.inner.translate(by); }
    fn rotate_about(&mut self, axis: [f32; 3], degrees: f32, pivot: [f32; 3]) { self.inner.rotate_about(axis, degrees, pivot); }
    fn scale(&mut self, by: [f32; 3]) { self.inner.scale(by); }
    fn quad(&mut self, sprite: &str, corners: [[f32; 3]; 4], uv: [f32; 4], tint: u32) -> bool { self.inner.quad(sprite, corners, uv, tint) }
    fn cube(&mut self, from: [f32; 3], to: [f32; 3], sprite: &str, tint: u32) -> bool { self.inner.cube(from, to, sprite, tint) }
    fn block(&mut self, block: &str, properties: &[(&str, &str)]) -> bool { self.inner.block(block, properties) }
    fn item(&mut self, item: &ItemSlot) -> bool { self.inner.item(item) }
    fn item_with_context(&mut self, item: &ItemSlot, context: ItemDisplayContext) -> bool { self.inner.item_with_context(item, context) }
    fn model(&mut self, model: &str, tint: u32) -> bool { self.inner.model(model, tint) }
    fn bake_layer(&mut self, layer: &str) -> Option<ModelPart> { self.inner.bake_layer(layer) }
    fn model_part(&mut self, part: &ModelPart, sprite: &str, tint: u32) -> bool { self.inner.model_part(part, sprite, tint) }
    fn keep(&mut self) { self.keep_requested = true; }
}

/// Draws the block entity with the block's renderer; false if the block has none.
pub(crate) fn render(block: &str, pos: BlockPos, data: &NbtCompound, items: &[Option<ItemSlot>], canvas: &mut dyn BlockEntityCanvas) -> bool {
    let renderer = RENDERERS.lock().ok().and_then(|all| all.as_ref().and_then(|m| m.get(block).cloned()));
    match renderer {
        Some(renderer) => {
            let mut canvas = CacheCanvas { inner: canvas, keep_requested: false };
            renderer(&mut BlockEntityRender { pos, block, data, items, canvas: &mut canvas });
            if canvas.keep_requested && super::block_entity_api_v1::client_ticks(block) == Some(false) {
                canvas.inner.keep();
            }
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::base::{Nbt, NbtCompoundExt};

    #[derive(Default)]
    struct Recorder(Vec<String>, Option<NbtCompound>, Option<f32>, Option<i64>);

    impl BlockEntityCanvas for Recorder {
        fn client_data(&self) -> Option<&NbtCompound> { self.1.as_ref() }
        fn partial_tick(&self) -> Option<f32> { self.2 }
        fn game_time(&self) -> Option<i64> { self.3 }
        fn state_property(&self, _: &str) -> Option<String> { self.2.map(|_| "north".into()) }
        fn push(&mut self) {
            self.0.push("push".into());
        }
        fn pop(&mut self) {
            self.0.push("pop".into());
        }
        fn translate(&mut self, by: [f32; 3]) {
            self.0.push(format!("translate {by:?}"));
        }
        fn rotate_about(&mut self, _: [f32; 3], degrees: f32, _: [f32; 3]) {
            self.0.push(format!("rotate {degrees}"));
        }
        fn scale(&mut self, _: [f32; 3]) {}
        fn quad(&mut self, sprite: &str, _: [[f32; 3]; 4], _: [f32; 4], _: u32) -> bool {
            self.0.push(format!("quad {sprite}"));
            true
        }
        fn cube(&mut self, _: [f32; 3], to: [f32; 3], sprite: &str, _: u32) -> bool {
            self.0.push(format!("cube {sprite} {to:?}"));
            true
        }
        fn block(&mut self, block: &str, _: &[(&str, &str)]) -> bool {
            self.0.push(format!("block {block}"));
            true
        }
        fn item(&mut self, item: &ItemSlot) -> bool {
            self.0.push(format!("item {}", item.item));
            true
        }
        fn item_with_context(&mut self, item: &ItemSlot, context: ItemDisplayContext) -> bool {
            if self.2.is_some() && context == ItemDisplayContext::Head {
                self.0.push("head-item".into()); true
            } else { context == ItemDisplayContext::Fixed && self.item(item) }
        }
        fn model(&mut self, _: &str, _: u32) -> bool {
            if self.2.is_some() { self.0.push("model".into()); true } else { false }
        }
        fn bake_layer(&mut self, _: &str) -> Option<ModelPart> {
            self.2.map(|_| ModelPart::new(Vec::new(), Vec::new(), PartPose::ZERO))
        }
        fn model_part(&mut self, _: &ModelPart, _: &str, _: u32) -> bool {
            if self.2.is_some() { self.0.push("model-part".into()); true } else { false }
        }
        fn keep(&mut self) {
            self.0.push("keep".into());
        }
    }

    #[test]
    fn a_registered_renderer_draws_from_the_data_the_server_sent_and_other_blocks_draw_nothing() {
        BlockEntityRendererRegistry::register(Identifier::parse("test:tank").unwrap(), |r| {
            let level = r.data.get_int("level", 0) as f32 / 1000.0;
            r.canvas.push();
            r.canvas.translate([0.0, 0.25, 0.0]);
            r.canvas.cube([0.1, 0.0, 0.1], [0.9, level, 0.9], "test:block/liquid", 0xFFFFFF);
            r.canvas.pop();
            // the contents are drawn from the slots the renderer is handed, not from the saved data
            if let Some(Some(stack)) = r.items.first() {
                r.canvas.item(stack);
            }
            r.canvas.keep();
        });
        assert!(BlockEntityRendererRegistry::has("test:tank"));
        assert!(!BlockEntityRendererRegistry::has("test:other"));
        let items = [Some(ItemSlot::new(Identifier::parse("minecraft:stick").unwrap(), 1)), None];
        let mut data = NbtCompound::new();
        data.insert("level".into(), Nbt::Int(500));
        let mut canvas = Recorder::default();
        assert!(render("test:tank", [1, 2, 3], &data, &items, &mut canvas));
        assert_eq!(canvas.0, ["push", "translate [0.0, 0.25, 0.0]", "cube test:block/liquid [0.9, 0.5, 0.9]", "pop", "item minecraft:stick", "keep"]);
        let mut nothing = Recorder::default();
        assert!(!render("test:other", [0; 3], &data, &items, &mut nothing));
        assert!(nothing.0.is_empty());
    }

    #[test]
    fn keep_is_decided_after_registration_changes_and_client_ticks_stay_frame_dependent() {
        use crate::pure::block_entity_api_v1::{self, BlockEntityType, BlockEntityTypeRegistry};
        use crate::pure::block_api_v1::MemoryBlockView;
        use std::sync::atomic::{AtomicBool, Ordering};
        let block = "test:cache_policy";
        let late = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&late);
        BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::parse(block).unwrap()));
        BlockEntityRendererRegistry::register(Identifier::parse(block).unwrap(), move |r| {
            r.canvas.keep();
            r.canvas.keep(); // coalesced and checked only after this callback
            let count = r.canvas.client_data().map(|d| d.get_int("ticks", 0)).unwrap_or(0) as f32;
            r.canvas.cube([0.0; 3], [1.0, count, 1.0], "test:client", 0xFFFFFF);
            if flag.swap(false, Ordering::SeqCst) {
                BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::parse(block).unwrap()).client_ticker(|tick| {
                    let count = tick.client_data.get_int("ticks", 0) as i32;
                    tick.client_data.insert("ticks".into(), Nbt::Int(count + 1));
                }));
            }
        });
        let saved = NbtCompound::new();
        let mut canvas = Recorder::default();
        assert!(render(block, [0; 3], &saved, &[], &mut canvas));
        assert_eq!(canvas.0.iter().filter(|line| *line == "keep").count(), 1);
        late.store(true, Ordering::SeqCst);
        canvas.0.clear();
        assert!(render(block, [0; 3], &saved, &[], &mut canvas));
        assert!(!canvas.0.iter().any(|line| line == "keep")); // registered AFTER keep request
        let mut local = NbtCompound::new();
        for expected in [1.0, 2.0] {
            assert!(block_entity_api_v1::client_tick(block, [0; 3], &saved, &mut local, &MemoryBlockView::default(), true));
            canvas.0.clear();
            canvas.1 = Some(local.clone());
            assert!(render(block, [0; 3], &saved, &[], &mut canvas));
            assert_eq!(canvas.0, [format!("cube test:client {:?}", [1.0, expected, 1.0])]);
        }
        assert!(saved.is_empty());
        BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::parse(block).unwrap()));
        canvas.0.clear();
        assert!(render(block, [0; 3], &saved, &[], &mut canvas));
        assert_eq!(canvas.0.iter().filter(|line| *line == "keep").count(), 1);
    }

    #[test]
    fn cache_policy_canvas_preserves_optional_frame_model_and_item_operations() {
        let mut inner = Recorder::default();
        inner.1 = Some(NbtCompound::from([("ticks".into(), Nbt::Int(5))]));
        inner.2 = Some(0.25);
        inner.3 = Some(42);
        {
            let mut canvas = CacheCanvas { inner: &mut inner, keep_requested: false };
            assert_eq!(canvas.client_data().unwrap().get_int("ticks", 0), 5);
            assert_eq!(canvas.partial_tick(), Some(0.25));
            assert_eq!(canvas.game_time(), Some(42));
            assert_eq!(canvas.state_property("facing").as_deref(), Some("north"));
            let part = canvas.bake_layer("test:layer#main").unwrap();
            assert!(canvas.model_part(&part, "test:entity", 0xFFFFFF));
            assert!(canvas.model("test:model", 0xFFFFFF));
            assert!(canvas.item_with_context(&ItemSlot::new(Identifier::parse("minecraft:stick").unwrap(), 1), ItemDisplayContext::Head));
            canvas.keep();
            assert!(canvas.keep_requested);
        }
        assert_eq!(inner.0, ["model-part", "model", "head-item"]); // no premature backend keep
    }

    #[test]
    fn poisoned_renderer_registry_refuses_existing_cache_generations() {
        // Poisoning is permanent; isolate it from the other registry tests.
        if std::env::var_os("PLUTO_RENDER_POISON_TEST_CHILD").is_none() || !std::env::args().any(|arg| arg == "--exact") {
            let name = concat!(module_path!(), "::poisoned_renderer_registry_refuses_existing_cache_generations");
            let filter = name.split_once("::").unwrap().1;
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", filter, "--test-threads=1"])
                .env("PLUTO_RENDER_POISON_TEST_CHILD", "1").output().unwrap();
            assert!(output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
                "{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
            return;
        }
        let block = Identifier::parse("test:poisoned_renderer").unwrap();
        BlockEntityRendererRegistry::register(block, |r| r.canvas.keep());
        assert!(super::super::render_cache::generation().is_some());
        assert!(std::panic::catch_unwind(|| {
            let _guard = RENDERERS.lock().unwrap();
            panic!("poison renderer registry");
        }).is_err());
        assert_eq!(super::super::render_cache::generation(), None);
        let mut canvas = Recorder::default();
        assert!(!render("test:poisoned_renderer", [0; 3], &NbtCompound::new(), &[], &mut canvas));
        assert!(canvas.0.is_empty());
    }

    #[test]
    fn legacy_canvas_keeps_fixed_items_but_never_substitutes_them_for_other_contexts() {
        let stack = ItemSlot::new(Identifier::parse("test:head").unwrap(), 1);
        let mut recorder = Recorder::default();
        let canvas: &mut dyn BlockEntityCanvas = &mut recorder;
        assert!(canvas.item_with_context(&stack, ItemDisplayContext::Fixed));
        for context in [ItemDisplayContext::None, ItemDisplayContext::ThirdPersonLeftHand,
            ItemDisplayContext::ThirdPersonRightHand, ItemDisplayContext::FirstPersonLeftHand,
            ItemDisplayContext::FirstPersonRightHand, ItemDisplayContext::Head,
            ItemDisplayContext::Gui, ItemDisplayContext::Ground, ItemDisplayContext::OnShelf] {
            assert!(!canvas.item_with_context(&stack, context));
        }
        // No unimplemented context was drawn using the fixed-frame transform.
        assert_eq!(recorder.0, ["item test:head"]);
        assert_eq!(recorder.state_property("rotation"), None);
        assert_eq!(recorder.partial_tick(), None);
        assert_eq!(recorder.game_time(), None);
        assert!(recorder.client_data().is_none());
        assert!(recorder.bake_layer("test:head#main").is_none());
        let part = ModelPart::new(Vec::new(), Vec::new(), PartPose::ZERO);
        assert!(!recorder.model_part(&part, "test:entity/head", 0xFFFFFF));
        assert_eq!(recorder.0, ["item test:head"]);
    }
}
