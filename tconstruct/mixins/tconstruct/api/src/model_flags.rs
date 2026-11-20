//! Positions in `custom_model_data.flags` that the imported tool item definitions read (SlimeKnights, MIT; see LICENSE).
//! Generated from the original tool models by tools/import_original_data.py; do not edit.
//!
//! The first four are use-time facts the tool item definitions dispatch on while the tool is held in use
//! (`using_item` plus the use animation, the crossbow ammo state). `OVERLAYS` lists the modifiers some tool model draws a
//! sprite for; flag `OVERLAY_BASE + i` is set while the tool has `OVERLAYS[i]` among its crafted modifiers.
#![allow(dead_code)]

pub(crate) const BLOCKING: usize = 0;
pub(crate) const SPEAR: usize = 1;
pub(crate) const ARROW: usize = 2;
pub(crate) const FIREWORK: usize = 3;
pub(crate) const OVERLAY_BASE: usize = 4;

pub(crate) const OVERLAYS: &[&str] = &[
    "tconstruct:ambidextrous",
    "tconstruct:antiaquatic",
    "tconstruct:aqua_affinity",
    "tconstruct:bane_of_sssss",
    "tconstruct:blasting",
    "tconstruct:bounce",
    "tconstruct:bucketing",
    "tconstruct:channeling",
    "tconstruct:collecting",
    "tconstruct:cooling",
    "tconstruct:depth_strider",
    "tconstruct:diamond",
    "tconstruct:double_jump",
    "tconstruct:emerald",
    "tconstruct:experienced",
    "tconstruct:feather_fall",
    "tconstruct:feather_falling",
    "tconstruct:fiery",
    "tconstruct:firestarter",
    "tconstruct:freezing",
    "tconstruct:glowing",
    "tconstruct:grapple",
    "tconstruct:haste",
    "tconstruct:hydraulic",
    "tconstruct:impaling",
    "tconstruct:item_frame",
    "tconstruct:knockback",
    "tconstruct:leaping",
    "tconstruct:lightspeed",
    "tconstruct:long_fall",
    "tconstruct:luck",
    "tconstruct:lure",
    "tconstruct:magnetic",
    "tconstruct:necrotic",
    "tconstruct:netherite",
    "tconstruct:overforced",
    "tconstruct:pierce",
    "tconstruct:power",
    "tconstruct:punch",
    "tconstruct:quick_charge",
    "tconstruct:reach",
    "tconstruct:reinforced",
    "tconstruct:respiration",
    "tconstruct:ricochet",
    "tconstruct:severing",
    "tconstruct:sharpness",
    "tconstruct:shield_strap",
    "tconstruct:silky",
    "tconstruct:sleeves",
    "tconstruct:smite",
    "tconstruct:soulbound",
    "tconstruct:soulspeed",
    "tconstruct:speedy",
    "tconstruct:springy",
    "tconstruct:step_up",
    "tconstruct:strength",
    "tconstruct:swift_sneak",
    "tconstruct:tank",
    "tconstruct:thorns",
    "tconstruct:trueshot",
    "tconstruct:unbreakable",
];
