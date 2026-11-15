//! The Tool Parts creative tab: one stack per part and visible material, as
//! IMaterialItem.addVariants lists them (SlimeKnights, MIT; see LICENSE).
//! The list is the shipped material set baked into creative_parts.rs, because a PlutoLib tab is
//! fixed when the mod starts, before materials load; a datapack's extra materials are not listed.
//! Tool stacks are not offered yet: a creative tool needs its rebuilt stats, which need the
//! loaded registries.
use crate::creative_parts::PARTS;
use pluto_lib::{Identifier, ItemGroup, ItemGroupRegistry, ItemStack};

fn part(item: &str, material: &str) -> ItemStack {
    ItemStack::new(Identifier::new("tconstruct", item).expect("valid part id"), 1)
        .with_components(&format!("{{\"minecraft:custom_data\":{{Material:\"tconstruct:{material}\"}}}}"))
}

/// The tab: icon is the first part of the first material, entries follow the original order.
pub(crate) fn tool_parts_group() -> ItemGroup {
    let icon = part("pick_head", "wood");
    let entries = PARTS.iter().flat_map(|(item, materials)| materials.iter().map(move |material| part(item, material)));
    ItemGroup::new(Identifier::new("tconstruct", "tool_parts").expect("valid group id"), "Tinkers' Construct Tool Parts", icon).entries(entries)
}

pub(crate) fn register() { ItemGroupRegistry::register(tool_parts_group()); }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_part_lists_each_material_once_in_original_order_and_stacks_carry_the_material() {
        let group = tool_parts_group();
        let expected: usize = PARTS.iter().map(|(_, materials)| materials.len()).sum();
        assert_eq!(group.entries.len(), expected);
        assert!(expected > 500, "the shipped data gives a large catalog");
        let first = &group.entries[0];
        assert_eq!(first.item.to_string(), "tconstruct:pick_head");
        assert_eq!(first.to_snbt(), "{count:1,id:\"tconstruct:pick_head\",components:{\"minecraft:custom_data\":{Material:\"tconstruct:wood\"}}}");
        // No part repeats a material.
        for (item, materials) in PARTS {
            let mut seen = std::collections::BTreeSet::new();
            assert!(materials.iter().all(|material| seen.insert(*material)), "{item} repeats a material");
        }
        // Parts of one stat type share a list, and a stat type only some materials have gives a shorter one.
        let materials = |name: &str| PARTS.iter().find(|(item, _)| *item == name).map(|(_, m)| *m).unwrap();
        assert_eq!(materials("pick_head"), materials("hammer_head"));
        assert!(materials("helmet_plating").len() < materials("pick_head").len(), "plating is not on every material");
        assert!(materials("ribcage").len() < materials("pick_head").len(), "slimesuit repair parts are rarer");
    }
}
