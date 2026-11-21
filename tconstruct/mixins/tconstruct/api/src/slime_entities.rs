//! TinkerWorld's slime mobs (SlimeKnights, MIT; see LICENSE): the slime family of the original is five entity
//! types — the vanilla `minecraft:slime` and `minecraft:magma_cube` plus the three types TinkerWorld
//! registers, `sky_slime`, `ender_slime` and `terracube` (the membership of the original's `forge:slimes`
//! entity type tag). The three registered types are declared here on PlutoLib's derived-type registry over
//! the vanilla slime implementation, with the entity-class queries and class attribute defaults their Java
//! classes override. Everything else their classes do is recorded below with the reach it waits on; nothing
//! is approximated.
//!
//! Registration (TinkerWorld, all three): category monster (inherited from the slime factory), builder box
//! 2.04 x 2.04 — the per-size scaling (0.255 x size) is the slime class's own, kept by the factory —
//! velocity updates on, tracking ranges 20 (sky), 32 (ender) and 8 (terracube), recorded as the registry's
//! declaration metadata exactly as PlutoLib's derived-slime examples record them; the host consumes no
//! tracking policy yet. Attributes are the factory's: the original registers the plain monster attribute
//! set, whose values agree with the slime factory's on every shared key. Each type's spawn egg
//! (`<name>_spawn_egg`, declared by the importer on the vanilla slime egg) carries the original's colours:
//! sky 0x47eff5/0xacfff4, ender 0x6300b0/0xd37cff, terracube 0xafb9d6/0xa1a7b1 — recorded in `TYPES`; the
//! 26.3 egg form has no tint layers (every vanilla egg has its own texture, and the original ships none for
//! its eggs), so the eggs render unmodelled for now. Each type's ambient particle is the particle type of
//! its own name (`tconstruct:sky_slime` etc., a plain particle type in the original); PlutoLib has no
//! entity-particle surface for derived types, so the particle types are not registered.
//!
//! Class behaviour (ArmoredSlimeEntity, shared by all three, over the vanilla Slime class):
//! - the three transient attribute modifiers registered here (armor, armor toughness and knockback
//!   resistance, each amount 3, multiply-total — a fourfold total): they are in the Java constructor, so
//!   they apply to every instance, metal or not.
//! - a synchronized `metal` flag (saved as the `metal` boolean): a metal slime's armor base value is its
//!   size, restored in `setSize`/`setMetal`. NOT WIRED: a derived type has no persistent or synchronized
//!   state of its own, and the load-bases callback context carries the slime size but no read of the
//!   entity's saved data, so the flag can neither be stored nor read back. The knock-on reductions are the
//!   armor base restoration, the loot tables' `metal` NBT condition (the tables ship verbatim; the
//!   condition can never hold until the flag exists) and the split inheritance below.
//! - spawn equipment (`finalizeSpawn`): `canPickUpLoot` at 0.55 x the difficulty instance's special
//!   multiplier, the equipment roll of the subclass, and on 31 October a 0.25 chance of a carved pumpkin
//!   (90%) or jack o'lantern (10%) in an empty head slot with head drop chance 0. NOT WIRED: PlutoLib's
//!   spawn finalization info carries dimension, position and reason but no difficulty, so none of the
//!   multiplier-scaled rolls can be evaluated, and there is no surface for the pickup flag. The rolls are
//!   recorded as data instead: sky and ender (TravelersPlateSlimeEntity) roll 0.15 for metal, then at
//!   0.15 x multiplier build a tool helmet — plate helmet at 0.35 x multiplier, else travelers helmet —
//!   from the plating material (`steel` for sky, `knightmetal` for ender, in `TYPES`) plus one ancient
//!   material, fill each free defense slot at 0.5 x multiplier per attempt with a random modifier of the
//!   `tconstruct:slime_defense` tag, and at 0.5 x multiplier add a random `tconstruct:gems` modifier to a
//!   free upgrade slot; terracube at 0.15 x multiplier rolls a vanilla helmet of quality nextInt(3) plus
//!   three independent 0.25 increments (quality 5 is a turtle helmet) and enchants it as spawned armor.
//! - death splitting: the slime factory splits a dying slime of size > 1 into 2 + nextInt(3) children of
//!   half size of the same type, as the original's classes inherit; the original's own bookkeeping on top —
//!   the children inherit persistence, name, no-AI, invulnerability and the metal flag, one random child
//!   receives the head stack and each other child copies it at 0.25 when the head drop chance allows —
//!   has no split callback surface and is not wired. The head stack's custom death drop (drop chance
//!   capped at 0.25 for size > 1, looting-scaled) likewise waits on that surface.
//!
//! SkySlimeEntity: jump power sqrt(size) x block jump factor / 2 (registered). Its `causeFallDamage`
//! bounces instead of hurting — falling more than 2 blocks reverses the vertical motion at -0.9, scales
//! the horizontal motion by 1/0.95 and plays `tconstruct:slimy_bounce`; sneaking suppresses the bounce and
//! takes a fifth of the fall. NOT WIRED: `causeFallDamage` is not the fall-damage calculation query (the
//! class does not override that), and PlutoLib has no cause-fall surface, so no fall override is registered
//! for the sky slime rather than a wrong one.
//!
//! EnderSlimeEntity: hurting a living target teleports the target to a random nearby position, and being
//! hurt teleports the slime itself (the original's ender teleport helper and event). NOT WIRED: the living
//! hurt events exist, but PlutoLib has no entity teleport action to answer them with.
//!
//! TerracubeEntity: jump power 0.5 x block jump factor, attack damage = attack attribute + 2, fall damage
//! calculation 0 (all registered).
//!
//! Spawning: the original registers spawn placements (OR operation) — the vanilla slime gets a predicate on
//! the `tconstruct:slime_spawn/earth` block tag, sky and ender get on-ground placements with the
//! motion-blocking-no-leaves heightmap and a predicate on `tconstruct:slime_spawn/sky` /
//! `tconstruct:slime_spawn/ender`, terracube gets the same placement with its own rule (never on peaceful,
//! always from a spawner, in water, else a valid spawn block in the dark). The block tags ship as data, but
//! PlutoLib has no spawn-placement registration surface, so the placements are recorded here only. Natural
//! spawning itself is data: the importer ships the original's three biome modifiers
//! (`spawn_overworld_slime`: sky slime, weight 100, group 2-4, overworld; `spawn_terracube`: weight 10,
//! group 2-4, overworld; `spawn_end_slime`: ender slime, weight 10, group 2-4, the end). The terracube
//! modifier's biome predicate (overworld and not `forge:no_default_monsters`) is widened to the overworld
//! tag, the same reduction the geode modifiers carry: no biome-predicate surface exists to express the
//! exclusion. The slime island structures' spawn overrides name exactly these three types (plus the
//! vanilla slime and magma cube), which is what makes the islands' forward references resolve.
use pluto_lib::{AttackDamageContext, AttributeModifierOperation, CalculateFallDamageContext, EntityTypeRegistry, Identifier, JumpPowerContext, LivingEntityBehavior, LivingEntityBehaviors, TransientAttributeModifier};

/// One of the original's registered slime types, with the registration values of TinkerWorld and the data
/// of the behaviour that is recorded rather than wired (see the module documentation).
pub(crate) struct SlimeType {
    /// The entity type's path under `tconstruct:`; its spawn egg is `<name>_spawn_egg` and its particle
    /// type shares the name.
    pub name: &'static str,
    /// The original's tracking range, as registered.
    pub tracking: u32,
    /// The spawn egg's two colours, primary and secondary.
    pub egg: (u32, u32),
    /// The fixed plating material of the spawn-equipment helmet build, for the travelers-plate slimes.
    pub plating: Option<&'static str>,
    /// The block tag of the spawn placement predicate, for the slimes that place by one.
    pub spawn_tag: Option<&'static str>,
}

pub(crate) const TYPES: &[SlimeType] = &[
    SlimeType { name: "sky_slime", tracking: 20, egg: (0x47eff5, 0xacfff4), plating: Some("steel"), spawn_tag: Some("slime_spawn/sky") },
    SlimeType { name: "ender_slime", tracking: 32, egg: (0x6300b0, 0xd37cff), plating: Some("knightmetal"), spawn_tag: Some("slime_spawn/ender") },
    SlimeType { name: "terracube", tracking: 8, egg: (0xafb9d6, 0xa1a7b1), plating: None, spawn_tag: None },
];

/// ArmoredSlimeEntity's constructor modifiers: armor, armor toughness and knockback resistance, each
/// amount 3 multiply-total under the original's modifier names (including its `resistence` spelling).
fn class_modifiers() -> Vec<TransientAttributeModifier> {
    [("minecraft:armor", "small_armor_bonus"), ("minecraft:armor_toughness", "small_toughness_bonus"), ("minecraft:knockback_resistance", "small_resistence_bonus")]
        .iter()
        .map(|(attribute, name)| TransientAttributeModifier {
            attribute: Identifier::parse(attribute).expect("attribute id"),
            id: Identifier::new("tconstruct", name).expect("modifier id"),
            amount: 3.0,
            operation: AttributeModifierOperation::AddMultipliedTotal,
        })
        .collect()
}

/// SkySlimeEntity.getJumpPower: sqrt(size) * block jump factor / 2. Without a slime size there is no
/// override (the context carries one only for native slime implementations).
fn sky_jump(context: &JumpPowerContext) -> Option<f32> {
    context.slime_size.map(|size| (size as f32).sqrt() * context.block_jump_factor / 2.0)
}

/// TerracubeEntity.getJumpPower: 0.5 * block jump factor.
fn terracube_jump(context: &JumpPowerContext) -> Option<f32> {
    Some(0.5 * context.block_jump_factor)
}

/// TerracubeEntity.getAttackDamage: the attack attribute + 2.
fn terracube_attack(context: &AttackDamageContext) -> Option<f32> {
    Some(context.attribute as f32 + 2.0)
}

/// TerracubeEntity.calculateFallDamage: 0.
fn terracube_fall(_: &CalculateFallDamageContext) -> Option<i32> {
    Some(0)
}

pub(crate) fn register() {
    let factory = Identifier::parse("minecraft:slime").expect("factory id");
    for slime in TYPES {
        let id = Identifier::new("tconstruct", slime.name).expect("entity type id");
        EntityTypeRegistry::register_derived(id.clone(), factory.clone(), 2.04, 2.04, slime.tracking, true)
            .unwrap_or_else(|error| panic!("registering tconstruct:{}: {error}", slime.name));
        let mut behavior = LivingEntityBehavior::new(id);
        for modifier in class_modifiers() {
            behavior = behavior.transient_attribute_modifier(modifier);
        }
        behavior = match slime.name {
            "sky_slime" => behavior.jump_power(sky_jump),
            "terracube" => behavior.jump_power(terracube_jump).attack_damage(terracube_attack).calculate_fall_damage(terracube_fall),
            _ => behavior,
        };
        LivingEntityBehaviors::register(behavior)
            .unwrap_or_else(|error| panic!("registering the behaviour of tconstruct:{}: {error}", slime.name));
    }
}
