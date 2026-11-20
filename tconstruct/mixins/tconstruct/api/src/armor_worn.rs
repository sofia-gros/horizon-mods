//! The worn layers of the tool-built armor: each piece's equipment textures follow the materials of its tool slots, as the original's
//! ArmorModelProvider picks them (MaterialArmorTextureSupplier over the material render info; SlimeKnights, MIT; see LICENSE). The
//! texture table is generated from the original's data (`armor_textures.rs`). The dyed variants need the dyed modifier and are not
//! drawn; trims are the game's own layer; the slime wings and the slimeskull helmet are not asked by the game's worn-armor seam.
use crate::armor_textures::{Layer, BASES, FAMILIES, ITEMS, VARIANTS};
use pluto_lib::{ArmorLayer, ArmorLayerKind, ArmorRenderContext, ArmorRenderer, Identifier};

/// The texture suffix and tint a material gives `base`: its variant, or the plain base when the material has none.
fn material_texture(base: &str, material: &str) -> (&'static str, Option<u32>) {
    VARIANTS.binary_search_by(|(b, m, _, _)| (*b, *m).cmp(&(base, material))).map(|at| (VARIANTS[at].2, VARIANTS[at].3)).unwrap_or(("", None))
}

/// The texture stems (under the layer set's folder) and tints of a worn piece, in draw order, given the material of each tool slot
/// as the stack's model data names them (an empty name is a material without a sprite of its own, which the original renders as
/// its `unknown` material). None for an item that has no humanoid layers here or a layer set that is not its own.
pub(crate) fn layers(item: &str, kind: ArmorLayerKind, materials: &[String]) -> Option<Vec<(String, Option<u32>)>> {
    let (_, family, word) = *ITEMS.iter().find(|(path, _, _)| *path == item)?;
    if word != if kind == ArmorLayerKind::HumanoidLeggings { "leggings" } else { "armor" } {
        return None;
    }
    let plan = FAMILIES.iter().find(|(name, _)| *name == family)?.1;
    let mut out = Vec::new();
    for layer in plan {
        let (prefix, slot) = match layer {
            Layer::Fixed(prefix) => (*prefix, None),
            Layer::Material(prefix, slot) => (*prefix, Some(*slot)),
        };
        let base = format!("{prefix}{word}");
        // The original skips a layer whose base texture does not exist for this layer set.
        if BASES.binary_search(&base.as_str()).is_err() {
            continue;
        }
        let (suffix, tint) = match slot {
            None => ("", None),
            Some(slot) => material_texture(&base, materials.get(slot).map(String::as_str).filter(|name| !name.is_empty()).unwrap_or("unknown")),
        };
        out.push((format!("{base}{suffix}"), tint));
    }
    Some(out)
}

/// Registers the worn-layer renderer of every armor item. Called once from the product's hooks.
pub(crate) fn register() {
    for (item, _, word) in ITEMS {
        if *word == "wings" {
            continue;
        }
        let name = *item;
        ArmorRenderer::register(&[Identifier::new("tconstruct", name).unwrap()], move |context: &ArmorRenderContext| {
            let materials = context.stack.custom_model_data.as_ref().map(|data| data.strings.as_slice()).unwrap_or(&[]);
            let worn = layers(name, context.kind, materials)?;
            worn.into_iter().map(|(stem, tint)| Some(ArmorLayer { texture: Identifier::new("tconstruct", &stem).ok()?, tint })).collect()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(materials: &[&str]) -> Vec<String> {
        materials.iter().map(|name| (*name).to_owned()).collect()
    }
    fn stems(item: &str, kind: ArmorLayerKind, materials: &[&str]) -> Vec<(String, Option<u32>)> {
        layers(item, kind, &names(materials)).unwrap()
    }

    #[test]
    fn travelers_layers_follow_the_cuirass_and_metal_slots() {
        // slot 0 is the metal plating, slot 1 the cuirass (the tool definition's stat order); the base is drawn first.
        let worn = stems("travelers_chestplate", ArmorLayerKind::Humanoid, &["iron", "leather"]);
        assert_eq!(worn.iter().map(|(stem, _)| stem.as_str()).collect::<Vec<_>>(), [
            "travelers/base_armor", "travelers/cuirass_armor_tconstruct_leather", "travelers/metal_armor_tconstruct_iron"]);
        assert!(worn.iter().all(|(_, tint)| tint.is_none()));
        // The leggings have no base texture, so that layer is skipped, and use their own texture set.
        let leggings = stems("travelers_leggings", ArmorLayerKind::HumanoidLeggings, &["iron", "leather"]);
        assert_eq!(leggings.iter().map(|(stem, _)| stem.as_str()).collect::<Vec<_>>(), [
            "travelers/cuirass_leggings_tconstruct_leather", "travelers/metal_leggings_tconstruct_iron"]);
    }

    #[test]
    fn a_material_without_a_baked_texture_tints_its_base_by_its_vertex_colour() {
        // Wood has no baked plate texture: the grey plating base takes the material's colour, as the original does.
        let worn = stems("plate_helmet", ArmorLayerKind::Humanoid, &["wood", "iron"]);
        assert_eq!(worn[0], ("plate/plating_armor".to_owned(), Some(0x876627)));
        assert_eq!(worn[1], ("plate/maille_armor_tconstruct_iron".to_owned(), None));
    }

    #[test]
    fn slime_pieces_put_the_skin_slot_under_the_ribcage_overlay() {
        // slot 1 is the skin (`slime/armor`), slot 0 the overlay material.
        let worn = stems("slime_boots", ArmorLayerKind::Humanoid, &["bone", "earthslime"]);
        assert_eq!(worn.iter().map(|(stem, _)| stem.as_str()).collect::<Vec<_>>()[0], "slime/armor_tconstruct_earthslime");
        assert!(worn[1].0.starts_with("slime/overlay_armor"));
    }

    #[test]
    fn an_unnamed_material_is_the_unknown_material_and_a_missing_slot_too() {
        let named = stems("plate_boots", ArmorLayerKind::Humanoid, &["", ""]);
        let missing = stems("plate_boots", ArmorLayerKind::Humanoid, &[]);
        assert_eq!(named, missing);
        assert_eq!(named[0].1, material_texture("plate/plating_armor", "unknown").1);
    }

    #[test]
    fn items_without_humanoid_layers_or_of_another_layer_set_answer_nothing() {
        assert_eq!(layers("slime_wings", ArmorLayerKind::Humanoid, &names(&["blood"])), None);
        assert_eq!(layers("slime_helmet", ArmorLayerKind::Humanoid, &[]), None);
        assert_eq!(layers("travelers_leggings", ArmorLayerKind::Humanoid, &[]), None);
        assert_eq!(layers("plate_boots", ArmorLayerKind::HumanoidLeggings, &[]), None);
    }

    #[test]
    fn the_generated_table_is_sorted_for_its_lookups() {
        assert!(BASES.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(VARIANTS.windows(2).all(|pair| (pair[0].0, pair[0].1) < (pair[1].0, pair[1].1)));
        assert!(VARIANTS.iter().all(|(base, ..)| BASES.binary_search(base).is_ok()));
    }

    #[test]
    fn the_registered_renderer_reads_the_stacks_model_strings() {
        register();
        let mut stack = pluto_lib::StackData { item: "tconstruct:plate_leggings".into(), count: 1, ..Default::default() };
        stack.custom_model_data = Some(pluto_lib::CustomModelData { strings: names(&["iron", "leather"]), ..Default::default() });
        let asked = ArmorRenderer::layers(&ArmorRenderContext { stack: stack.clone(), kind: ArmorLayerKind::HumanoidLeggings }).unwrap();
        assert_eq!(asked.iter().map(|layer| layer.texture.to_string()).collect::<Vec<_>>(), [
            "tconstruct:plate/plating_leggings_tconstruct_iron", "tconstruct:plate/maille_leggings_tconstruct_leather"]);
        assert_eq!(ArmorRenderer::layers(&ArmorRenderContext { stack, kind: ArmorLayerKind::Humanoid }), None);
    }
}
