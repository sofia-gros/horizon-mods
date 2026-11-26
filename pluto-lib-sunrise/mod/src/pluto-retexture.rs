// Retexture mesh seam: the position-to-texture cache the client block-entity
// tick pass feeds, and the face-layer swap the mesher's `face` calls through
// the `pluto-retexture-layer` expression injection.
//
// A retextured block's texture lives in its block entity's saved data under
// `texture` (Mantle's `RetexturedHelper.TAG_TEXTURE`, the item id of the
// material). The mesher runs on worker threads without the level, so the game
// thread copies each loaded block entity's texture into this cache once a
// tick (see `pluto_client_block_entity_tick`), and the mesh seam reads it
// back by position. Entries are keyed by position only; the seam always
// re-checks the CURRENT block's model against the pure table first, so a
// stale entry for a position whose block changed can never swap a face it
// should not: the block must be retexturable AND the face must sample one of
// its retextured slots. A level replacement clears the cache outright.
use sunrise_client_render::models::catalog::BakedFace;
use sunrise_client_render::mesher::Mesher;
use sunrise_core::position::BlockPos;
use sunrise_world::registry::BlockStateId;
use sunrise_client_render::*;
use std::collections::HashMap;
use std::sync::{LazyLock, OnceLock, RwLock};

// The face test, from the pure side: (block, sprite, baked UVs) -> whether
// the face belongs to one of the block's retextured slots. Installed by
// `hooks()`; the seam leaves every face alone until it is installed.
static MATCHER: OnceLock<fn(&str, &str, [[f32; 2]; 4]) -> bool> = OnceLock::new();

pub fn set_matcher(matcher: fn(&str, &str, [[f32; 2]; 4]) -> bool) {
    let _ = MATCHER.set(matcher);
}

// The texture item id each retextured block entity currently names, by world
// position. Written by the client block-entity tick pass, read by the mesh
// seam on worker threads.
static TEXTURES: LazyLock<RwLock<HashMap<[i32; 3], String>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn texture_at(pos: [i32; 3]) -> Option<String> {
    TEXTURES.read().ok()?.get(&pos).cloned()
}

// Record (or clear, with `None`) the texture a block entity at `pos` names.
// A write happens only when the record actually changes, so the steady
// per-tick refresh costs the tick pass a shared read per entity and nothing
// more; the mesher's reads are never blocked by an unchanged world.
pub fn refresh(pos: [i32; 3], texture: Option<&str>) {
    if texture_at(pos).as_deref() == texture {
        return;
    }
    let Ok(mut textures) = TEXTURES.write() else { return };
    match texture {
        Some(texture) => {
            textures.insert(pos, texture.to_owned());
        }
        None => {
            textures.remove(&pos);
        }
    }
}

// Drop every cached texture: the level was replaced or unloaded.
pub fn clear() {
    if let Ok(mut textures) = TEXTURES.write() {
        textures.clear();
    }
}

// The layer the `face.layer as u16` expression in `Mesher::face` should draw
// with: the texture block's particle layer when this face is one of the
// block's retextured slots and the block entity at `world` names a texture,
// else the face's own layer. A texture that names no block, or a block whose
// model has no particle sprite, leaves the face alone.
pub fn layer(value: u16, mesher: &Mesher<'_>, id: u32, world: BlockPos, face: &BakedFace) -> u16 {
    let Some(matcher) = MATCHER.get() else { return value };
    let Some(state) = mesher.assets.registry.state(BlockStateId(id)) else { return value };
    if !matcher(&state.name, &face.quad.material.sprite, face.quad.uv) {
        return value;
    }
    let Some(texture) = texture_at([world.x, world.y, world.z]) else { return value };
    let Some(texture_state) = mesher.assets.registry.default_state(&texture) else {
        return value;
    };
    let Some(layer) = mesher.render.model_catalog.particle(texture_state) else {
        return value;
    };
    u16::try_from(layer).unwrap_or(value)
}
