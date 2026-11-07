// EXCLUDED candidate: no manifest, startup registration or ordinary content change.
// This maps only the four PR716 flow-routing facts at the final d82 cutoff.
// It MUST NOT be installed as complete fluid behavior: all four still require
// native travel/current/fall/burn, fog/overlay, path and source/flow/save proof.
use pluto_api::fluid::{Declaration, Definition, RenderDefinition, RenderOrientation, SlopeDistance};

pub fn flow_fixture_declarations() -> Result<[Declaration; 4], String> {
    fn one(name: &str, emission: u8) -> Result<Declaration, String> {
        let mineral = name.strip_prefix("molten_").ok_or("expected molten family")?;
        let definition = Definition {
            id: format!("tconstruct:{name}"),
            // PR731 renamed the colliding original PR716 block identities.
            block: format!("tconstruct:{name}_fluid"),
            bucket: format!("tconstruct:{name}_bucket"),
            texture: format!("tconstruct:fluid/molten/ore/{mineral}/still"),
            tick_delay: 30, drop_off: 3, light: emission,
        };
        let render = RenderDefinition {
            flowing_texture: format!("tconstruct:fluid/molten/ore/{mineral}/flowing"),
            tint: 0xFF_FFFF, orientation: RenderOrientation::Bottom,
        };
        Ok(Declaration::new(definition, render)?.with_slope_distance(SlopeDistance::new(3)?))
    }
    Ok([one("molten_iron", 12)?, one("molten_gold", 12)?,
        one("molten_copper", 12)?, one("molten_cobalt", 8)?])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn four_flow_candidates_keep_final_ids_and_explicit_source_routing() {
        let entries = flow_fixture_declarations().unwrap();
        for (entry, (name, light)) in entries.iter().zip([
            ("molten_iron",12), ("molten_gold",12), ("molten_copper",12), ("molten_cobalt",8),
        ]) {
            let d = entry.definition();
            assert_eq!(d.id, format!("tconstruct:{name}"));
            assert_eq!(d.block, format!("tconstruct:{name}_fluid"));
            assert_eq!(d.bucket, format!("tconstruct:{name}_bucket"));
            assert_eq!((d.tick_delay,d.drop_off,d.light), (30,3,light));
            assert_eq!(entry.slope_distance().unwrap().get(), 3);
            assert_eq!(entry.render().orientation, RenderOrientation::Bottom);
            assert!(entry.render_material_model().unwrap().is_some());
        }
        // Declaration generation is not installation or registry activation.
        for entry in entries { assert!(pluto_api::fluid::get(&entry.definition().id).is_none()); }
    }
}
