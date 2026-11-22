//! The original's Tinkers fluid container transfers (Mantle `fill_item` / `empty_item`, SlimeKnights; see LICENSE): a bowl or glass bottle fills with
//! an amount of a fluid and becomes the filled item, a filled item gives the amount back as the empty one. Generated from the pinned original
//! by tools/import_original_data.py; do not edit by hand.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind { Fill, Empty }

/// One transfer: its name, direction, the held item, the fluids (ids in the tconstruct namespace) it takes or gives, the amount and the resulting item.
pub(crate) struct Transfer { pub name: &'static str, pub kind: Kind, pub item: &'static str, pub fluids: &'static [&'static str], pub amount: u32, pub result: &'static str }

pub(crate) const TRANSFERS: &[Transfer] = &[
    Transfer { name: "earth_slime_bottle_fill", kind: Kind::Fill, item: "minecraft:glass_bottle", fluids: &["earth_slime"], amount: 250, result: "tconstruct:earth_slime_bottle" },
    Transfer { name: "ender_slime_bottle_fill", kind: Kind::Fill, item: "minecraft:glass_bottle", fluids: &["ender_slime"], amount: 250, result: "tconstruct:ender_slime_bottle" },
    Transfer { name: "ichor_slime_bottle_fill", kind: Kind::Fill, item: "minecraft:glass_bottle", fluids: &["ichor"], amount: 250, result: "tconstruct:ichor_slime_bottle" },
    Transfer { name: "magma_bottle_fill", kind: Kind::Fill, item: "minecraft:glass_bottle", fluids: &["magma"], amount: 250, result: "tconstruct:magma_bottle" },
    Transfer { name: "meat_soup_empty", kind: Kind::Empty, item: "tconstruct:meat_soup", fluids: &["meat_soup"], amount: 250, result: "minecraft:bowl" },
    Transfer { name: "meat_soup_fill", kind: Kind::Fill, item: "minecraft:bowl", fluids: &["meat_soup"], amount: 250, result: "tconstruct:meat_soup" },
    Transfer { name: "sky_slime_bottle_fill", kind: Kind::Fill, item: "minecraft:glass_bottle", fluids: &["sky_slime"], amount: 250, result: "tconstruct:sky_slime_bottle" },
    Transfer { name: "venom_bottle_fill", kind: Kind::Fill, item: "minecraft:glass_bottle", fluids: &["venom"], amount: 250, result: "tconstruct:venom_bottle" },
];
