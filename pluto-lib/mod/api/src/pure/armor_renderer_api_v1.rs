//! Counterpart of Fabric's `ArmorRenderer` (`fabric-rendering-v1`): picks the texture layers a worn armor stack is drawn with, per stack.
//!
//! An equipment asset names one fixed list of layers per item. A stack whose look depends on its own data (a tool-built armor piece whose
//! texture follows its materials) registers a renderer here instead. The renderer is asked for the layers of one worn piece and answers
//! them in draw order; the game keeps drawing its own glint, trim, light and the equipment asset of any stack that declines.
//!
//! The humanoid layers of the player, other players, mobs and armor stands are connected through one source-declared host seam in
//! `armor_layers`. Worn wings (`wings` layers) are not asked.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use super::base::Identifier;
use super::player_api::StackData;

/// Which equipment layer set of the equipment asset a renderer is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArmorLayerKind {
    /// Helmet, chestplate and boots (`humanoid`).
    Humanoid,
    /// Leggings (`humanoid_leggings`).
    HumanoidLeggings,
}

impl ArmorLayerKind {
    /// The equipment asset's key and texture folder of the kind.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Humanoid => "humanoid",
            Self::HumanoidLeggings => "humanoid_leggings",
        }
    }
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        match key {
            "humanoid" => Some(Self::Humanoid),
            "humanoid_leggings" => Some(Self::HumanoidLeggings),
            _ => None,
        }
    }
}

/// What an armor renderer is asked.
#[derive(Clone, Debug, PartialEq)]
pub struct ArmorRenderContext {
    /// The worn stack, with its custom data and model data (the host's merged components, as for item colours).
    pub stack: StackData,
    /// The layer set being drawn.
    pub kind: ArmorLayerKind,
}

/// One drawn layer of a worn piece.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmorLayer {
    /// The equipment texture, named as an equipment asset names a layer: `namespace:path` for
    /// `assets/<namespace>/textures/entity/equipment/<kind folder>/<path>.png`. Never a `textures/` path or a PNG name.
    pub texture: Identifier,
    /// `0xRRGGBB` multiplied into the layer, or `None` for no tint.
    pub tint: Option<u32>,
}

type Renderer = Arc<dyn Fn(&ArmorRenderContext) -> Option<Vec<ArmorLayer>> + Send + Sync>;
static RENDERERS: RwLock<BTreeMap<String, Renderer>> = RwLock::new(BTreeMap::new());

/// Cheap host preflight: nothing is copied or decoded for an item without a renderer.
pub(crate) fn has_renderer(item: &str) -> bool {
    RENDERERS.read().ok().is_some_and(|all| all.contains_key(item))
}

/// The host's answer for one worn stack: sprite ids as the host's equipment loader forms them, with the layer colour.
/// `None` leaves the equipment asset's own layers; so does a layer that is not a plain equipment texture name.
pub(crate) fn host_layers(stack: StackData, kind: &str) -> Option<Vec<(String, Option<u32>)>> {
    let kind = ArmorLayerKind::from_key(kind)?;
    let layers = ArmorRenderer::layers(&ArmorRenderContext { stack, kind })?;
    let mut out = Vec::with_capacity(layers.len());
    for layer in layers {
        let path = layer.texture.path();
        if path.is_empty() || path.starts_with("textures/") || path.ends_with(".png") {
            eprintln!("PlutoLib: armor layer {} is not an equipment texture name", layer.texture);
            return None;
        }
        let sprite = if layer.texture.namespace() == "minecraft" {
            format!("entity/equipment/{}/{path}", kind.key())
        } else {
            format!("{}:entity/equipment/{}/{path}", layer.texture.namespace(), kind.key())
        };
        out.push((sprite, layer.tint.map(|rgb| rgb & 0xFF_FFFF)));
    }
    Some(out)
}

/// The registry of worn armor renderers (Fabric: `ArmorRenderer`).
pub struct ArmorRenderer;

impl ArmorRenderer {
    /// Registers one renderer for each item (Fabric: `ArmorRenderer.register`). A newer registration replaces the old one.
    /// The renderer answers the layers in draw order, or `None` to leave the item's equipment asset.
    pub fn register(items: &[Identifier], renderer: impl Fn(&ArmorRenderContext) -> Option<Vec<ArmorLayer>> + Send + Sync + 'static) {
        let renderer: Renderer = Arc::new(renderer);
        let replaced = {
            let Ok(mut all) = RENDERERS.write() else { return };
            items.iter().filter_map(|item| all.insert(item.to_string(), renderer.clone())).collect::<Vec<_>>()
        };
        // Dropping a captured user value may itself register another renderer.
        drop(replaced);
    }

    /// Asks the renderer of the stack's item, outside the registry lock. `None` for no registration or a decline.
    pub fn layers(context: &ArmorRenderContext) -> Option<Vec<ArmorLayer>> {
        let renderer = RENDERERS.read().ok()?.get(&context.stack.item).cloned()?;
        renderer(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(item: &str) -> StackData {
        StackData { item: item.into(), count: 1, ..Default::default() }
    }
    fn layer(texture: &str, tint: Option<u32>) -> ArmorLayer {
        ArmorLayer { texture: Identifier::parse(texture).unwrap(), tint }
    }

    #[test]
    fn host_layers_name_sprites_per_kind_and_mask_the_tint() {
        ArmorRenderer::register(&[Identifier::parse("example:suit").unwrap()], |_| {
            Some(vec![layer("example:suit/base", None), layer("minecraft:own", Some(0xFF12_3456))])
        });
        assert!(has_renderer("example:suit") && !has_renderer("example:other"));
        let worn = host_layers(stack("example:suit"), "humanoid_leggings").unwrap();
        assert_eq!(worn, vec![
            ("example:entity/equipment/humanoid_leggings/suit/base".to_owned(), None),
            ("entity/equipment/humanoid_leggings/own".to_owned(), Some(0x12_3456)),
        ]);
        assert_eq!(host_layers(stack("example:other"), "humanoid"), None);
        assert_eq!(host_layers(stack("example:suit"), "wings"), None);
    }

    #[test]
    fn a_decline_or_a_png_path_keeps_the_equipment_asset_and_a_reregistration_replaces() {
        let item = Identifier::parse("example:fussy").unwrap();
        ArmorRenderer::register(&[item.clone()], |context| match context.kind {
            ArmorLayerKind::Humanoid => None,
            ArmorLayerKind::HumanoidLeggings => Some(vec![layer("example:ok", None), layer("example:bad.png", None)]),
        });
        assert_eq!(host_layers(stack("example:fussy"), "humanoid"), None);
        assert_eq!(host_layers(stack("example:fussy"), "humanoid_leggings"), None);
        ArmorRenderer::register(&[item], |_| Some(vec![layer("example:ok", Some(1))]));
        assert_eq!(host_layers(stack("example:fussy"), "humanoid_leggings").unwrap().len(), 1);
    }

    #[test]
    fn the_renderer_sees_the_stack_data_and_may_register_while_running() {
        let item = Identifier::parse("example:chameleon").unwrap();
        let again = item.clone();
        ArmorRenderer::register(&[item], move |context| {
            ArmorRenderer::register(&[again.clone()], |_| None);
            context.stack.custom_model_data.as_ref().and_then(|data| data.strings.first()).map(|name| vec![layer(&format!("example:{name}"), None)])
        });
        let mut piece = stack("example:chameleon");
        piece.custom_model_data = Some(super::super::player_api::CustomModelData { strings: vec!["gold".into()], ..Default::default() });
        let first = host_layers(piece.clone(), "humanoid").unwrap();
        assert_eq!(first[0].0, "example:entity/equipment/humanoid/gold");
        assert_eq!(host_layers(piece, "humanoid"), None);
    }
}
