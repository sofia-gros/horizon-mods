// Retextured items: the item-side half of the retexture seam (the block
// half is `pluto_retexture`, on the mesher). A retextured stack names its
// texture in its custom data, but the host bakes an item's model once per
// item, so the swap runs where the stack is still known: the shared
// `pluto-item-color-layers` return seam hands the resolved layers over,
// and each quad layer is rebuilt with the texture on the faces the pure
// table names as the item's retextured slots.
//
// The replacement sprite is the texture item's own particle sprite — for
// a block item the same particle the mesh seam draws the placed block
// with — read off a fresh resolution of the texture item. Resolution
// runs per drawn stack, so the sprite is cached by texture item; the
// swapped models are rebuilt per resolution, like the render state they
// land in.
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, LazyLock, OnceLock, RwLock};
use serde_json::Value;
use sunrise_client::item_render::sprite_name;
use sunrise_assets::item_model::StackView;
use sunrise_assets::item_bake::BakedItemModel;
use sunrise_assets::item_model::DisplayContext;
use sunrise_assets::item_model::ItemEnv;
use sunrise_client::item_render::ItemRenderer;
use sunrise_client::item_render::LayerGeometry;
use sunrise_client::item_render::LayerRender;

// The stack's texture, from its merged components: the item id its
// custom data names, if it names one. Installed by `hooks()`.
type Texture = fn(&BTreeMap<String, Value>) -> Option<String>;
// The pure face test, shared with the mesh seam: (item, sprite, baked
// UVs) -> whether the face belongs to one of the item's retextured slots.
type Matcher = fn(&str, &str, [[f32; 2]; 4]) -> bool;

struct Provider {
    texture: Texture,
    matcher: Matcher,
}

static PROVIDER: OnceLock<Provider> = OnceLock::new();

pub fn set_provider(texture: Texture, matcher: Matcher) -> Result<(), &'static str> {
    PROVIDER
        .set(Provider { texture, matcher })
        .map_err(|_| "retexture item provider already installed")
}

// The particle sprite each texture item resolves to, by item id: a
// texture's sprite cannot change while its models stand, and resolution
// runs per drawn stack per frame.
static PARTICLES: LazyLock<RwLock<HashMap<String, String>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

fn particle_sprite(renderer: &ItemRenderer, texture: &str, env: &dyn ItemEnv) -> Option<String> {
    if let Some(sprite) = PARTICLES.read().ok()?.get(texture) {
        return Some(sprite.clone());
    }
    let mut view = StackView::new(texture, 1);
    // The lookup needs the texture item's model, never its own retexture:
    // removing custom data from the view's patch keeps the nested
    // resolution out of this seam (and out of the tint pass).
    view.patch.insert("minecraft:custom_data".to_owned(), None);
    let state = renderer.resolve(&view, DisplayContext::Gui, None, None, 0, env);
    let sprite = state.layers.iter().find_map(|layer| match &layer.geometry {
        LayerGeometry::Quads(baked) => Some(baked.particle.sprite.clone()),
        LayerGeometry::Special { .. } => None,
    })?;
    if let Ok(mut particles) = PARTICLES.write() {
        particles.insert(texture.to_owned(), sprite.clone());
    }
    Some(sprite)
}

// `baked` rebuilt with `sprite` on the faces the matcher names as
// retextured slots of `item`; None when no face is one, so the layer
// keeps the shared baked model. UVs stay: a slot samples the same
// rectangle of its replacement sprite, as the mesh seam's layer swap
// leaves them.
fn swapped(
    baked: &BakedItemModel,
    item: &str,
    sprite: &str,
    matcher: Matcher,
) -> Option<Arc<BakedItemModel>> {
    if !baked.quads.iter().any(|quad| matcher(item, &quad.material.sprite, quad.uv)) {
        return None;
    }
    let mut model = baked.clone();
    for quad in &mut model.quads {
        if matcher(item, &quad.material.sprite, quad.uv) {
            quad.material.sprite = sprite.to_owned();
        }
    }
    Some(Arc::new(model))
}

pub fn retexture(
    renderer: &ItemRenderer,
    stack: &StackView,
    env: &dyn ItemEnv,
    mut layers: Vec<LayerRender>,
) -> Vec<LayerRender> {
    let Some(provider) = PROVIDER.get() else { return layers };
    let components = renderer.models.components(stack);
    let Some(texture) = (provider.texture)(&components) else { return layers };
    let Some(sprite) = particle_sprite(renderer, &texture, env) else { return layers };
    for layer in &mut layers {
        let LayerGeometry::Quads(baked) = &layer.geometry else { continue };
        let Some(model) = swapped(baked, &stack.item, &sprite, provider.matcher) else { continue };
        layer.geometry = LayerGeometry::Quads(model);
        layer.particle = sprite_name(&sprite).to_owned();
    }
    layers
}
