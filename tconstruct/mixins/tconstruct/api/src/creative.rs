//! The Tool Parts creative tab: one stack per part and visible material, as
//! IMaterialItem.addVariants lists them (SlimeKnights, MIT; see LICENSE).
//! The list is the shipped material set baked into creative_parts.rs, because a PlutoLib tab is
//! fixed when the mod starts, before materials load; a datapack's extra materials are not listed.
//! The Tools tab holds the armor and shield stacks (ToolBuildHandler.addVariants over the shipped
//! materials, from creative_tools.rs); the other tools are not offered yet. A stack carries only
//! its material list: its stats need the loaded registries, so they are rebuilt when the stack is used.
use crate::{creative_parts::PARTS, creative_tools::ARMOR};
use pluto_lib::{Identifier, ItemGroup, ItemGroupRegistry, ItemStack};

fn part(item: &str, material: &str) -> ItemStack {
    ItemStack::new(Identifier::new("tconstruct", item).expect("valid part id"), 1)
        .with_components(&format!("{{\"minecraft:custom_data\":{{Material:\"tconstruct:{material}\"}},\"minecraft:custom_model_data\":{{strings:[\"{material}\"]}}}}"))
}

/// The tab: icon is the first part of the first material, entries follow the original order.
pub(crate) fn tool_parts_group() -> ItemGroup {
    let icon = part("pick_head", "wood");
    let entries = PARTS.iter().flat_map(|(item, materials)| materials.iter().map(move |material| part(item, material)));
    ItemGroup::new(Identifier::new("tconstruct", "tool_parts").expect("valid group id"), "Tinkers' Construct Tool Parts", icon).entries(entries)
}

/// A tool stack made of the given materials, one per material slot, uninitialized like ToolStack.createTool before the registries load.
fn tool(item: &str, materials: &[&str]) -> ItemStack {
    let list = materials.iter().map(|material| format!("\"tconstruct:{material}\"")).collect::<Vec<_>>().join(",");
    // The item models select their sprites on custom_model_data.strings, one material path per slot, as tool_materials::sync_model keeps them.
    let sprites = materials.iter().map(|material| format!("\"{material}\"")).collect::<Vec<_>>().join(",");
    ItemStack::new(Identifier::new("tconstruct", item).expect("valid tool id"), 1)
        .with_components(&format!("{{\"minecraft:custom_data\":{{tic_materials:[{list}]}},\"minecraft:custom_model_data\":{{strings:[{sprites}]}}}}"))
}

/// The Tools tab: armor and shields in the original order, each tool once per visible material.
pub(crate) fn tools_group() -> ItemGroup {
    let icon = ItemStack::new(Identifier::new("tconstruct", "pickaxe").expect("valid tool id"), 1);
    let entries = ARMOR.iter().flat_map(|(item, variants)| variants.iter().map(move |materials| tool(item, materials)));
    ItemGroup::new(Identifier::new("tconstruct", "tools").expect("valid group id"), "Tinkers' Construct Tools", icon).entries(entries)
}

pub(crate) fn register() {
    ItemGroupRegistry::register(tools_group());
    ItemGroupRegistry::register(tool_parts_group());
}

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
        assert_eq!(first.to_snbt(), "{count:1,id:\"tconstruct:pick_head\",components:{\"minecraft:custom_data\":{Material:\"tconstruct:wood\"},\"minecraft:custom_model_data\":{strings:[\"wood\"]}}}");
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

    #[test]
    fn armor_tab_lists_each_tool_per_material_with_slots_filled_like_create_single_material() {
        let group = tools_group();
        let expected: usize = ARMOR.iter().map(|(_, variants)| variants.len()).sum();
        assert_eq!(group.entries.len(), expected);
        let count = |item: &str| ARMOR.iter().find(|(name, _)| *name == item).map(|(_, v)| v.len()).unwrap();
        assert_eq!(group.entries[0].to_snbt(), "{count:1,id:\"tconstruct:travelers_helmet\",components:{\"minecraft:custom_data\":{tic_materials:[\"tconstruct:copper\",\"tconstruct:leather\"]},\"minecraft:custom_model_data\":{strings:[\"copper\",\"leather\"]}}}");
        // Wings have one slot; plate takes plating and maille, so a plating-only material gets the first maille material.
        assert!(ARMOR.iter().find(|(name, _)| *name == "slime_wings").unwrap().1.iter().all(|m| m.len() == 1));
        assert!(count("plate_helmet") == count("plate_chestplate") && count("slime_helmet") > count("slime_chestplate"));
        // Every material of the parts tab's plating list makes a plate helmet entry, in the same order.
        let plating: Vec<&str> = PARTS.iter().find(|(item, _)| *item == "helmet_plating").unwrap().1.to_vec();
        let helmet = ARMOR.iter().find(|(name, _)| *name == "plate_helmet").unwrap().1;
        assert!(plating.iter().all(|material| helmet.iter().any(|slots| slots[0] == *material)));
        // The list starts with the armor of the original order: travelers, plate, then the slimesuit and wings last.
        let order: Vec<&str> = ARMOR.iter().map(|(name, _)| *name).collect();
        assert_eq!(order.first(), Some(&"travelers_helmet"));
        assert_eq!(order.last(), Some(&"slime_wings"));
    }
}
