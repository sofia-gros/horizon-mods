//! Drawing a block entity (Minecraft: `BlockEntityRenderer`; Fabric: `BlockEntityRendererFactories.register`). The renderer runs on the
//! client once a frame for each block entity of the block in view and adds geometry under a pose stack that starts at the block's lower
//! north-west corner, in blocks. It sees the data the server sent (`BlockEntityType::update_tag` / `sync_data`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::base::{Identifier, NbtCompound};
use super::block_entity_api_v1::{BlockPos, ItemSlot};

/// The surface a block entity renderer draws on. PlutoLib passes one to the renderer; a test can implement it to check what a renderer
/// draws. Positions are in blocks under the current pose; a `tint` is `0xRRGGBB`.
pub trait BlockEntityCanvas {
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
    /// Draws a model (`mymod:block/gear`) the mod registered; false if there is none.
    fn model(&mut self, model: &str, tint: u32) -> bool;
    /// Keeps what was drawn until the block entity, its light or the resource packs change, so idle machines cost nothing per frame.
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
    /// Where to draw.
    pub canvas: &'a mut dyn BlockEntityCanvas,
}

type Renderer = Arc<dyn Fn(&mut BlockEntityRender<'_>) + Send + Sync>;

static RENDERERS: Mutex<Option<HashMap<String, Renderer>>> = Mutex::new(None);

/// Registers block entity renderers (Fabric: `BlockEntityRendererFactories.register`).
pub struct BlockEntityRendererRegistry;

impl BlockEntityRendererRegistry {
    /// Draws every block entity of `block` with `renderer`. Registering the same block again replaces the earlier renderer.
    pub fn register(block: Identifier, renderer: impl Fn(&mut BlockEntityRender<'_>) + Send + Sync + 'static) {
        if let Ok(mut all) = RENDERERS.lock() {
            all.get_or_insert_with(HashMap::new).insert(block.to_string(), Arc::new(renderer));
        }
    }

    /// Whether `block` has a renderer.
    pub fn has(block: &str) -> bool {
        RENDERERS.lock().ok().and_then(|all| all.as_ref().map(|m| m.contains_key(block))).unwrap_or(false)
    }
}

/// Draws the block entity with the block's renderer; false if the block has none.
pub(crate) fn render(block: &str, pos: BlockPos, data: &NbtCompound, canvas: &mut dyn BlockEntityCanvas) -> bool {
    let renderer = RENDERERS.lock().ok().and_then(|all| all.as_ref().and_then(|m| m.get(block).cloned()));
    match renderer {
        Some(renderer) => {
            renderer(&mut BlockEntityRender { pos, block, data, canvas });
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
    struct Recorder(Vec<String>);

    impl BlockEntityCanvas for Recorder {
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
        fn model(&mut self, _: &str, _: u32) -> bool {
            false
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
            r.canvas.item(&ItemSlot::new(Identifier::parse("minecraft:stick").unwrap(), 1));
            r.canvas.keep();
        });
        assert!(BlockEntityRendererRegistry::has("test:tank"));
        assert!(!BlockEntityRendererRegistry::has("test:other"));
        let mut data = NbtCompound::new();
        data.insert("level".into(), Nbt::Int(500));
        let mut canvas = Recorder::default();
        assert!(render("test:tank", [1, 2, 3], &data, &mut canvas));
        assert_eq!(canvas.0, ["push", "translate [0.0, 0.25, 0.0]", "cube test:block/liquid [0.9, 0.5, 0.9]", "pop", "item minecraft:stick", "keep"]);
        let mut nothing = Recorder::default();
        assert!(!render("test:other", [0; 3], &data, &mut nothing));
        assert!(nothing.0.is_empty());
    }
}
