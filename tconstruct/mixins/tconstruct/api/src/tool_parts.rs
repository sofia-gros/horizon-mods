//! The original tool-part items and the material stat type each one takes, from
//! TinkerToolParts (SlimeKnights, MIT; see LICENSE). Other-mod parts are not included.
use crate::tool_materials::ToolPart;
use pluto_lib::Identifier;
use std::collections::BTreeMap;

/// Part item path -> stat type path, in the original registration order.
const PARTS: &[(&str, &str)] = &[
    ("pick_head", "head"), ("hammer_head", "head"), ("small_axe_head", "head"), ("broad_axe_head", "head"),
    ("small_blade", "head"), ("broad_blade", "head"), ("adze_head", "head"), ("large_plate", "head"),
    ("bow_limb", "limb"), ("bow_grip", "grip"), ("bowstring", "bowstring"),
    ("arrow_head", "arrow_head"), ("arrow_shaft", "arrow_shaft"), ("fletching", "fletching"),
    ("tool_binding", "binding"), ("tough_binding", "binding"), ("tool_handle", "handle"), ("tough_handle", "handle"),
    ("plating_helmet", "plating_helmet"), ("plating_chestplate", "plating_chestplate"),
    ("plating_leggings", "plating_leggings"), ("plating_boots", "plating_boots"),
    ("maille", "maille"), ("shield_core", "shield_core"),
    ("ribcage", "ribcage"), ("shell", "shell"), ("laces", "laces"),
];

pub(crate) fn registry() -> BTreeMap<Identifier, ToolPart> {
    PARTS.iter().map(|(item, stat)| {
        let item = Identifier::new("tconstruct", item).expect("valid part item");
        (item.clone(), ToolPart { item, stat: Identifier::new("tconstruct", stat).expect("valid stat type") })
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_part_uses_a_known_stat_type_and_items_are_unique() {
        let registry = registry();
        assert_eq!(registry.len(), PARTS.len(), "duplicate part item");
        for part in registry.values() {
            assert!(crate::material_stats::is_registered(&part.stat), "unknown stat type {}", part.stat);
        }
    }
}
