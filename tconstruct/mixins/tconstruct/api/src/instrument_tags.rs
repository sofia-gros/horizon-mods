//! The pinned vanilla instrument registry and the instrument tags the shipped data names, flattened.
//! Generated from the pinned vanilla data and the original's generated resources by tools/import_original_data.py; do not edit by hand.

pub(crate) const REGISTRY: &[&str] = &["minecraft:admire_goat_horn", "minecraft:call_goat_horn", "minecraft:dream_goat_horn", "minecraft:feel_goat_horn", "minecraft:ponder_goat_horn", "minecraft:seek_goat_horn", "minecraft:sing_goat_horn", "minecraft:yearn_goat_horn"];

pub(crate) const TAGS: &[(&str, &[&str])] = &[
    ("minecraft:goat_horns", &["minecraft:ponder_goat_horn", "minecraft:sing_goat_horn", "minecraft:seek_goat_horn", "minecraft:feel_goat_horn", "minecraft:admire_goat_horn", "minecraft:call_goat_horn", "minecraft:yearn_goat_horn", "minecraft:dream_goat_horn"]),
    ("minecraft:regular_goat_horns", &["minecraft:ponder_goat_horn", "minecraft:sing_goat_horn", "minecraft:seek_goat_horn", "minecraft:feel_goat_horn"]),
    ("minecraft:screaming_goat_horns", &["minecraft:admire_goat_horn", "minecraft:call_goat_horn", "minecraft:yearn_goat_horn", "minecraft:dream_goat_horn"]),
    ("tconstruct:variant_horns", &["minecraft:ponder_goat_horn", "minecraft:sing_goat_horn", "minecraft:seek_goat_horn", "minecraft:feel_goat_horn", "minecraft:admire_goat_horn", "minecraft:call_goat_horn", "minecraft:yearn_goat_horn", "minecraft:dream_goat_horn"]),
];
