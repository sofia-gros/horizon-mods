//! Vanilla damage-type tags the Tinkers damage tags and protection modules name, flattened.
//! Generated from the pinned vanilla data by tools/import_original_data.py; do not edit by hand.

pub(crate) const TAGS: &[(&str, &[&str])] = &[
    ("bypasses_enchantments", &["minecraft:sonic_boom"]),
    ("bypasses_invulnerability", &["minecraft:out_of_world", "minecraft:generic_kill"]),
    ("is_explosion", &["minecraft:fireworks", "minecraft:explosion", "minecraft:player_explosion", "minecraft:bad_respawn_point"]),
    ("is_fall", &["minecraft:fall", "minecraft:ender_pearl", "minecraft:stalagmite"]),
    ("is_fire", &["minecraft:in_fire", "minecraft:campfire", "minecraft:on_fire", "minecraft:lava", "minecraft:hot_floor", "minecraft:sulfur_cube_hot", "minecraft:unattributed_fireball", "minecraft:fireball"]),
    ("is_lightning", &["minecraft:lightning_bolt"]),
    ("is_projectile", &["minecraft:arrow", "minecraft:trident", "minecraft:mob_projectile", "minecraft:unattributed_fireball", "minecraft:fireball", "minecraft:wither_skull", "minecraft:thrown", "minecraft:wind_charge"]),
    ("witch_resistant_to", &["minecraft:magic", "minecraft:indirect_magic", "minecraft:sonic_boom", "minecraft:thorns"]),
];
