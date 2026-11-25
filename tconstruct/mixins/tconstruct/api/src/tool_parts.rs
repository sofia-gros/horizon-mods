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
    // registerEnum(types, "plating") names these <type>_plating, unlike the plating stat types.
    ("helmet_plating", "plating_helmet"), ("chestplate_plating", "plating_chestplate"),
    ("leggings_plating", "plating_leggings"), ("boots_plating", "plating_boots"),
    ("maille", "maille"), ("shield_core", "shield_core"),
    ("ribcage", "ribcage"), ("shell", "shell"), ("laces", "laces"),
];

/// Max stack size of each non-armor tool item, from TinkerTools.java (UNSTACKABLE_PROPS is 1).
const TOOL_STACKS: &[(&str, i32)] = &[
    ("pickaxe", 1), ("sledge_hammer", 1), ("vein_hammer", 1), ("mattock", 1), ("pickadze", 1), ("excavator", 1),
    ("hand_axe", 1), ("broad_axe", 1), ("kama", 1), ("scythe", 1), ("dagger", 2), ("sword", 1), ("cleaver", 1),
    ("crossbow", 1), ("longbow", 1), ("fishing_rod", 1), ("javelin", 1), ("arrow", 64), ("shuriken", 16), ("throwing_axe", 16),
    ("flint_and_brick", 1), ("sky_staff", 1), ("earth_staff", 1), ("ichor_staff", 1), ("ender_staff", 1),
    ("melting_pan", 1), ("war_pick", 1), ("battlesign", 1), ("swasher", 1), ("minotaur_axe", 1), ("travelers_shield", 1), ("plate_shield", 1),
    // Armor and wings: unstackable like every worn item.
    ("plate_helmet", 1), ("slime_helmet", 1), ("travelers_helmet", 1), ("plate_chestplate", 1), ("slimy_chestplate", 1), ("travelers_chestplate", 1), ("plate_leggings", 1), ("slime_leggings", 1), ("travelers_leggings", 1), ("plate_boots", 1), ("slime_boots", 1), ("travelers_boots", 1), ("slime_wings", 1),
];

/// The stack limit of a tool item; anything else is treated as unstackable like the base item properties.
pub(crate) fn tool_stack_size(item: &Identifier) -> i32 {
    if item.namespace() != "tconstruct" { return 1; }
    TOOL_STACKS.iter().find(|(path, _)| *path == item.path()).map_or(1, |(_, size)| *size)
}

/// Every tool item the original registers (`TinkerTools`), in registration order.
pub(crate) fn tool_items() -> impl Iterator<Item = Identifier> {
    TOOL_STACKS.iter().map(|(path, _)| Identifier::new("tconstruct", path).expect("valid tool item"))
}

pub(crate) fn registry() -> BTreeMap<Identifier, ToolPart> {
    PARTS.iter().map(|(item, stat)| {
        let item = Identifier::new("tconstruct", item).expect("valid part item");
        (item.clone(), ToolPart { item, stat: Identifier::new("tconstruct", stat).expect("valid stat type") })
    }).collect()
}

/// Every material item, which is what the part builder and the casting blocks make parts of (IMaterialItem): the tool parts, and the repair kit
/// (RepairKitItem has its own stat type but is not a ToolPart, so it is not in `registry`, which the tool station reads for the parts of a tool).
pub(crate) fn material_items() -> BTreeMap<Identifier, ToolPart> {
    let mut items = registry();
    let kit = Identifier::new("tconstruct", "repair_kit").expect("valid item");
    items.insert(kit.clone(), ToolPart { item: kit, stat: Identifier::new("tconstruct", "repair_kit").expect("valid stat type") });
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_stack_sizes_follow_the_original_properties() {
        let size = |path: &str| tool_stack_size(&Identifier::new("tconstruct", path).unwrap());
        assert_eq!((size("pickaxe"), size("dagger"), size("arrow"), size("shuriken"), size("unknown")), (1, 2, 64, 16, 1));
        assert_eq!(tool_stack_size(&Identifier::parse("minecraft:arrow").unwrap()), 1);
        assert_eq!(TOOL_STACKS.len(), 45);
    }
    #[test]
    fn every_part_uses_a_known_stat_type_and_items_are_unique() {
        let registry = registry();
        assert_eq!(registry.len(), PARTS.len(), "duplicate part item");
        assert_eq!(material_items().len(), PARTS.len() + 1, "the repair kit is a material item but not a tool part");
        for part in material_items().values() {
            assert!(crate::material_stats::is_registered(&part.stat), "unknown stat type {}", part.stat);
        }
    }
}
