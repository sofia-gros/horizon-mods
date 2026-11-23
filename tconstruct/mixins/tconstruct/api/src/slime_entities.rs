//! TinkerWorld's slime mobs (SlimeKnights, MIT; see LICENSE): the slime family of the original is five entity
//! types — the vanilla `minecraft:slime` and `minecraft:magma_cube` plus the three types TinkerWorld
//! registers, `sky_slime`, `ender_slime` and `terracube` (the membership of the original's `forge:slimes`
//! entity type tag). The three registered types are declared here on PlutoLib's derived-type registry over
//! the vanilla slime implementation, with their entity-class behaviour wired through PlutoLib's living
//! behaviours, spawn finalization, split callback and placement registry.
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
//! its eggs), so the eggs render unmodelled for now. Each type's particle is the particle type of its own
//! name (`tconstruct:sky_slime` etc.): a plain particle type in the original whose client factory is a
//! breaking-item particle of the type's own slime ball (the terracube's is a clay ball) — exactly the
//! item-crumb shape PlutoLib's entity particle override replaces a landing particle with. `crate::particles`
//! declares the three types with those factories and points each type's override at its own type, the
//! host-side shape of the slime classes' `getParticleType()`.
//!
//! Class behaviour (ArmoredSlimeEntity, shared by all three, over the vanilla Slime class):
//! - the three transient attribute modifiers registered here (armor, armor toughness and knockback
//!   resistance, each amount 3, multiply-total — a fourfold total): they are in the Java constructor, so
//!   they apply to every instance, metal or not.
//! - a synchronized `metal` flag (saved as the `metal` boolean), stored in the `tconstruct:metal`
//!   attachment: a metal slime's armor base value is its size, restored on load by the load-bases
//!   callback (the original maintains it in `setSize`/`setMetal`). The loot tables' `metal` NBT
//!   condition reads the same flag: the game's attachment store seeds from the entity's saved data, so
//!   the shipped tables' condition sees it as the original's synched data. Split children inherit the
//!   flag through the split callback below.
//! - spawn equipment (`finalizeSpawn`, wired as the `tconstruct:slime_finalization` spawn callback):
//!   `canPickUpLoot` at 0.55 x the difficulty instance's special multiplier (carried on the spawn
//!   info), the equipment roll of the subclass, and on 31 October a 0.25 chance of a carved pumpkin
//!   (90%) or jack o'lantern (10%) in an empty head slot with head drop chance 0. All rolls are drawn
//!   during preparation from the mob's own stream, as the original draws them in `finalizeSpawn`;
//!   the completion applies them after the inherited initialization, before the mob joins. Sky and
//!   ender (TravelersPlateSlimeEntity) roll 0.15 for metal, then at 0.15 x multiplier build a tool
//!   helmet — plate helmet at 0.35 x multiplier, else travelers helmet — from the plating material
//!   (`steel` for sky, `knightmetal` for ender, in `TYPES`) plus one ancient material (any tier,
//!   hidden allowed, never in `tconstruct:exclude_from_loot`), fill each free defense slot at
//!   0.5 x multiplier per attempt with a random modifier of the `tconstruct:slime_defense` tag, and
//!   at 0.5 x multiplier add a random `tconstruct:gems` modifier to a free upgrade slot; the helmet
//!   build runs only when the material/modifier registries are loaded, and never fabricates a stack
//!   otherwise. Terracube at 0.15 x multiplier rolls a vanilla helmet of quality nextInt(3) plus
//!   three independent 0.25 increments (quality 5 is a turtle helmet) and enchants it as spawned
//!   armor at (int)(5 + multiplier x nextInt(18)); the backend runs the game's own enchantment
//!   selection at that level. The Halloween date is read from the system clock in UTC; the original
//!   reads the local date, which differs only within hours of the day boundary.
//! - death splitting: the game splits a dying slime of size > 1 into 2 + nextInt(3) children of half
//!   size of the same type itself (the original's classes inherit the same split); the original's own
//!   bookkeeping on top is wired as the `tconstruct:slime_inheritance` split callback — the children
//!   inherit name, no-AI, invulnerability and the metal flag, one random child receives the head
//!   stack and each other child copies it at 0.25 when the parent's head drop chance is below 1.
//!   Persistence, size and position are the game's own split work, as in the original. The head
//!   stack's custom death drop needs no consumer code: the game's equipment drop loop is the
//!   original's `dropCustomDeathLoot` shape already (looting-scaled roll, the same durability
//!   formula, the slot cleared), and the one class rule on top — the chance capped at 0.25 for
//!   size > 1 — is registered as the living behaviour's equipment drop chance for all three types.
//!   The looting of the killing blow reaches the roll through the drop chance context, so a
//!   recorded chance of exactly 0 still drops at looting × 1%, as the original's roll does.
//!
//! SkySlimeEntity: jump power sqrt(size) x block jump factor / 2 (registered). Its `causeFallDamage`
//! is registered as the living behaviour's cause-fall override: falling more than 2 blocks bounces —
//! the vertical motion reverses at -0.9, the horizontal motion scales by 1/0.95, the fall distance
//! clears and `tconstruct:slimy_bounce` plays — and any shorter fall is cancelled outright, as the
//! original returns false for every non-suppressed fall. A suppressing (sneaking) fall would take
//! the inherited application at a fifth of the multiplier in the original; mobs never sneak, and
//! the behaviour declines that case, keeping the full inherited application. The original also
//! re-applies the stored bounce to the vertical motion once in `move()`; the override sets the
//! motion and clears the on-ground flag at the landing itself, which the combined build's smoke
//! must confirm still leaves the ground.
//!
//! EnderSlimeEntity: hurting a living target teleports the target to a random nearby position, and
//! being hurt teleports the slime itself (the original's ender teleport helper, diameter 16). Wired
//! through the living damage event plus the entity tick: the damage event records the pending
//! teleports, and the slime's next tick applies them through the entity reach, so a teleport lands
//! at most a tick after the hit rather than inside it. The application follows the helper: up to
//! 16 candidate positions, a candidate refused when either block the body would occupy is solid
//! or liquid (read through the world reach, where the original tests the entity's own collision
//! box), the `tconstruct:slime_teleport` sound at the origin and the destination, and a passenger
//! leaves its vehicle before it moves. Players are teleported too: the reach queues them through
//! the player teleport transport, and the game's own server teleport applies the request at the
//! level's rider synchronisation, so a player's teleport also lands at most a tick after the hit.
//! The vertical destination clamps to the dimension's own build range.
//! The hurt-side trigger approximates the original's health-decreased check with the damage event's
//! post-absorption amount, so a fully absorbed hit inside the invulnerability window can teleport
//! where the original would not.
//!
//! TerracubeEntity: jump power 0.5 x block jump factor, attack damage = attack attribute + 2, fall damage
//! calculation 0 (all registered).
//!
//! Spawning: the original registers spawn placements (OR operation) — the vanilla slime gets a predicate on
//! the `tconstruct:slime_spawn/earth` block tag, sky and ender get on-ground placements with the
//! motion-blocking-no-leaves heightmap and a predicate on `tconstruct:slime_spawn/sky` /
//! `tconstruct:slime_spawn/ender`, terracube gets the same placement with its own rule (never on peaceful,
//! always from a spawner, in water, else a valid spawn block in the dark). All four tag predicates
//! (the derived slimes' and the vanilla slime's) are registered on the placement registry's OR side
//! (the tag predicates carry the shipped tags' current expansion, pinned against the tag JSONs by the
//! domain tests): PlutoLib's permit-add pass proposes its own candidates after the game's natural
//! pass and spawns the ones a predicate accepts, so the slimes spawn on their tag blocks even where
//! the inherited species rules turn the position away — the original's OR. (The registry's replacing
//! consultation at natural finalize time remains for predicates that replace a type's rules
//! outright, terracube's among them: its rule is the entity's own `canSpawnHere`, not a
//! SpawnPlacements registration.) Natural
//! spawning itself is data: the importer ships the original's three biome modifiers
//! (`spawn_overworld_slime`: sky slime, weight 100, group 2-4, overworld; `spawn_terracube`: weight 10,
//! group 2-4, overworld; `spawn_end_slime`: ender slime, weight 10, group 2-4, the end). The terracube
//! modifier's biome predicate (overworld and not `forge:no_default_monsters`) is widened to the overworld
//! tag, the same reduction the geode modifiers carry: no biome-predicate surface exists to express the
//! exclusion. The slime island structures' spawn overrides name exactly these three types (plus the
//! vanilla slime and magma cube), which is what makes the islands' forward references resolve.
use pluto_lib::{AttachmentRegistry, AttachmentType, AttackDamageContext, AttributeBaseValue, AttributeModifierOperation, CalculateFallDamageContext, CauseFallDamageContext, DamageModification, EntityInfo, EntityReach, EntityTypeRegistry, EquipmentDropChanceContext, FallOutcome, Identifier, JumpPowerContext, LivingEntityBehavior, LivingEntityBehaviors, LivingEntityLoadContext, MobSpawnAction, MobSpawnCallback, MobSpawnCompletion, MobSpawnPreparation, MobSplitCallback, MobSplitCallbacks, MobSplitInheritance, Nbt, NbtCompound, ServerEntityEvents, ServerLivingEntityEvents, SpawnPlacementContext, SpawnPlacementRegistry, StackData, TransientAttributeModifier, World};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock};

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
    /// The current expansion of the spawn placement block tag (the crystal blocks plus the slimy grass
    /// variants), for the slimes that place by one; pinned against the shipped tag JSONs by the domain tests.
    pub spawn_blocks: &'static [&'static str],
}

pub(crate) const TYPES: &[SlimeType] = &[
    SlimeType { name: "sky_slime", tracking: 20, egg: (0x47eff5, 0xacfff4), plating: Some("steel"), spawn_tag: Some("slime_spawn/sky"),
        spawn_blocks: &["tconstruct:sky_slime_crystal_block", "tconstruct:budding_sky_slime_crystal",
            "tconstruct:sky_earth_slime_grass", "tconstruct:sky_sky_slime_grass", "tconstruct:sky_ichor_slime_grass",
            "tconstruct:sky_ender_slime_grass", "tconstruct:sky_vanilla_slime_grass"] },
    SlimeType { name: "ender_slime", tracking: 32, egg: (0x6300b0, 0xd37cff), plating: Some("knightmetal"), spawn_tag: Some("slime_spawn/ender"),
        spawn_blocks: &["tconstruct:ender_slime_crystal_block", "tconstruct:budding_ender_slime_crystal",
            "tconstruct:ender_earth_slime_grass", "tconstruct:ender_sky_slime_grass", "tconstruct:ender_ichor_slime_grass",
            "tconstruct:ender_ender_slime_grass", "tconstruct:ender_vanilla_slime_grass"] },
    SlimeType { name: "terracube", tracking: 8, egg: (0xafb9d6, 0xa1a7b1), plating: None, spawn_tag: None, spawn_blocks: &[] },
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

/// ArmoredSlimeEntity's synchronized `metal` flag, as the synced attachment the loot tables and the
/// load-bases callback read (saved as the `metal` boolean, as the original's synched data is).
fn metal_attachment() -> &'static AttachmentType<bool> {
    static METAL: OnceLock<AttachmentType<bool>> = OnceLock::new();
    METAL.get_or_init(|| AttachmentRegistry::create_synced(Identifier::parse("tconstruct:metal").expect("attachment id"), || false))
}

/// ArmoredSlimeEntity.setSize/setMetal: the armor base value is the slime's size while metal, else 0.
fn armor_base(context: &LivingEntityLoadContext) -> Result<Vec<AttributeBaseValue>, String> {
    let value = if context.entity.get_attached(metal_attachment()) { context.slime_size.unwrap_or(0) as f64 } else { 0.0 };
    Ok(vec![AttributeBaseValue { attribute: Identifier::parse("minecraft:armor").expect("attribute id"), value }])
}

/// ArmoredSlimeEntity.dropCustomDeathLoot: the head slot's drop chance is capped at 0.25 above size 1.
pub(crate) fn head_drop_chance(context: &EquipmentDropChanceContext) -> Option<f32> {
    (context.slot == "head" && context.slime_size.is_some_and(|size| size > 1) && context.chance > 0.25).then_some(0.25)
}

/// SkySlimeEntity.causeFallDamage: a fall of more than 2 bounces with no damage; a shorter fall is
/// cancelled too (the original returns false for every non-suppressed fall). A suppressing fall
/// declines, keeping the inherited application (mobs never suppress; see the module documentation).
pub(crate) fn sky_fall(context: &CauseFallDamageContext) -> Option<FallOutcome> {
    if context.suppressing_bounce { return None; }
    if context.distance > 2.0 {
        Some(FallOutcome::Bounce {
            velocity: [context.velocity[0] / 0.95, -context.velocity[1] * 0.9, context.velocity[2] / 0.95],
            sound: Some(Identifier::parse("tconstruct:slimy_bounce").expect("sound id")),
        })
    } else {
        Some(FallOutcome::Cancel)
    }
}

/// ToolDataNBT.getFreeSlots, as commands.rs computes it: persistent plus volatile counts for the type.
fn free_slots(nbt: &NbtCompound, name: &str) -> i32 {
    let count = |tag: &str| match nbt.get(tag) {
        Some(Nbt::Compound(compound)) => crate::modifier_entries::nbt_int(compound.get(name)),
        _ => 0,
    };
    count(crate::tool_stack::TAG_PERSISTENT) + count(crate::tool_stack::TAG_VOLATILE)
}

/// ToolDataNBT.addSlots on the persistent data, as commands.rs computes it: ignores 0, removes the key
/// at a 0 total.
fn add_slot_count(persistent: &mut NbtCompound, name: &str, count: i32) {
    if count == 0 { return; }
    let total = crate::modifier_entries::nbt_int(persistent.get(name)).wrapping_add(count);
    if total == 0 { persistent.remove(name); } else { persistent.insert(name.to_owned(), Nbt::Int(total)); }
}

/// RandomMaterial.ancient: any tier, hidden allowed, never a material in `tconstruct:exclude_from_loot`,
/// with stats for the requested type; a uniform pick, falling back to the first material with the stat.
pub(crate) fn ancient_material(materials: &crate::materials::Definitions, stat: &Identifier, view: &mut dyn MobSpawnPreparation) -> Option<Identifier> {
    let excluded = materials.tags.get(&Identifier::parse("tconstruct:exclude_from_loot").expect("tag id"));
    let choices: Vec<&Identifier> = materials.materials.keys()
        .filter(|id| !excluded.is_some_and(|list| list.contains(id)) && materials.get_stats(id, stat).is_some())
        .collect();
    if choices.is_empty() { return materials.first_with_stat(stat).map(|material| material.id.clone()); }
    let index = view.next_int(choices.len() as i32).ok()?;
    Some(choices[index as usize].clone())
}

/// The modifier rolls of TravelersPlateSlimeEntity.populateDefaultEquipmentSlots on a built helmet:
/// each free defense slot is attempted at 0.5 x multiplier (consume the slot, add a random
/// `tconstruct:slime_defense` modifier), then one free upgrade slot at 0.5 x multiplier takes a
/// random `tconstruct:gems` modifier.
fn add_slime_modifiers(view: &mut dyn MobSpawnPreparation, tool: &mut NbtCompound, env: &crate::tool_stack::Environment, mult: f32) {
    let modifiers = env.modifiers;
    let mut upgrades = crate::tool_stack::upgrades(tool);
    let mut changed = false;
    if let Some(choices) = modifiers.tags.get(&Identifier::parse("tconstruct:slime_defense").expect("tag id")) {
        if !choices.is_empty() {
            for _ in 0..free_slots(tool, "defense") {
                if view.next_float() > 0.5 * mult { break; }
                let pick = choices[view.next_int(choices.len() as i32).unwrap_or(0) as usize].clone();
                if let Ok(next) = upgrades.with_modifier(pick, 1) {
                    upgrades = next;
                    changed = true;
                    crate::tool_stack::with_persistent(tool, |persistent| add_slot_count(persistent, "defense", -1));
                }
            }
        }
    }
    if free_slots(tool, "upgrades") > 0 && view.next_float() < 0.5 * mult {
        if let Some(choices) = modifiers.tags.get(&Identifier::parse("tconstruct:gems").expect("tag id")) {
            if !choices.is_empty() {
                let pick = choices[view.next_int(choices.len() as i32).unwrap_or(0) as usize].clone();
                if let Ok(next) = upgrades.with_modifier(pick, 1) {
                    upgrades = next;
                    changed = true;
                    crate::tool_stack::with_persistent(tool, |persistent| add_slot_count(persistent, "upgrades", -1));
                }
            }
        }
    }
    if changed {
        tool.insert(crate::tool_stack::TAG_UPGRADES.into(), upgrades.to_nbt());
        let _ = crate::tool_stack::rebuild_stats(tool, env);
    }
}

/// Builds the travelers-plate helmet ToolStack for a spawn: the definition's stat types pair with the
/// plating material, one ancient material, then the first material with each remaining stat. Runs only
/// when the material/modifier registries are loaded; returns None rather than fabricating a stack.
fn build_slime_helmet(view: &mut dyn MobSpawnPreparation, item: &Identifier, plating: &str, mult: f32) -> Option<StackData> {
    let materials = crate::common::register().read().ok()?;
    if !materials.server_loaded { return None; }
    let modifiers = crate::common::server_modifiers()?.read().ok()?;
    if !modifiers.definitions_loaded { return None; }
    let definitions = crate::tool_definitions::register().read().ok()?;
    let definition = definitions.data.get(item)?;
    let stat_types = definition.material_stat_types()?;
    let tags = crate::common::item_snapshot();
    let env = crate::tool_stack::Environment {
        definition: Some(definition), has_materials: true, materials: &materials, modifiers: &modifiers,
        item_tags_loaded: tags.is_some(), item: Some(item), item_tags: tags.as_deref(),
    };
    let mut variants = Vec::new();
    for (index, (stat, _scale)) in stat_types.iter().enumerate() {
        let chosen = match index {
            0 => Identifier::parse(&format!("tconstruct:{plating}")).ok()?,
            1 => ancient_material(&materials, stat, view)?,
            _ => materials.first_with_stat(stat).map(|material| material.id.clone())?,
        };
        variants.push(crate::tool_materials::MaterialVariant::parse(&chosen.to_string())?);
    }
    let mut tool = crate::tool_build::create_tool(&env, &crate::tool_materials::MaterialList::from_variants(variants), false).ok()?;
    add_slime_modifiers(view, &mut tool, &env, mult);
    Some(StackData { item: item.to_string(), count: 1, custom_data: tool, ..Default::default() })
}

/// TravelersPlateSlimeEntity.populateDefaultEquipmentSlots' rolls: 0.15 for metal, then at
/// 0.15 x multiplier a helmet (plate at 0.35 x multiplier, else travelers). Returns the metal roll
/// and the built helmet stack.
fn travelers_plate_rolls(view: &mut dyn MobSpawnPreparation, kind: &str, mult: f32) -> (bool, Option<StackData>) {
    let metal = view.next_float() < 0.15;
    if view.next_float() >= 0.15 * mult { return (metal, None); }
    let (item, plating) = if view.next_float() < 0.35 * mult {
        ("tconstruct:plate_helmet", if kind == "tconstruct:sky_slime" { "steel" } else { "knightmetal" })
    } else {
        ("tconstruct:travelers_helmet", if kind == "tconstruct:sky_slime" { "steel" } else { "knightmetal" })
    };
    let item = Identifier::parse(item).expect("helmet item id");
    (metal, build_slime_helmet(view, &item, plating, mult))
}

/// TerracubeEntity.populateDefaultEquipmentSlots' rolls: at 0.15 x multiplier a vanilla helmet of
/// quality nextInt(3) plus three independent 0.25 increments (5 is a turtle helmet), enchanted as
/// spawned armor at (int)(5 + multiplier x nextInt(18)). Returns the stack and the enchant level.
pub(crate) fn terracube_rolls(view: &mut dyn MobSpawnPreparation, mult: f32) -> Option<(StackData, i32)> {
    if view.next_float() >= 0.15 * mult { return None; }
    let mut quality = view.next_int(3).ok()?;
    for _ in 0..3 { if view.next_float() < 0.25 { quality += 1; } }
    let item = match quality {
        5 => "minecraft:turtle_helmet",
        0 => "minecraft:leather_helmet",
        1 => "minecraft:golden_helmet",
        2 => "minecraft:chainmail_helmet",
        3 => "minecraft:iron_helmet",
        _ => "minecraft:diamond_helmet",
    };
    let level = 5 + (mult * view.next_int(18).ok()? as f32) as i32;
    Some((StackData { item: item.into(), count: 1, ..Default::default() }, level))
}

/// Whether today is 31 October (the original checks the local date; the system clock is read in UTC).
fn is_halloween() -> bool {
    let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |elapsed| (elapsed.as_secs() / 86_400) as i64);
    let (_, month, day) = civil_from_days(days);
    month == 10 && day == 31
}

/// Days since the Unix epoch to a civil date (Howard Hinnant's algorithm).
pub(crate) fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// ArmoredSlimeEntity.finalizeSpawn and the subclass equipment rolls, as the spawn callback: every
/// roll is drawn here during preparation from the mob's own stream; the returned action applies the
/// outcomes after the inherited initialization, before the mob joins.
pub(crate) fn finalize_spawn(view: &mut dyn MobSpawnPreparation) -> Result<Option<MobSpawnAction>, String> {
    let kind = view.info().entity_type.to_string();
    if !matches!(kind.as_str(), "tconstruct:sky_slime" | "tconstruct:ender_slime" | "tconstruct:terracube") { return Ok(None); }
    let mult = view.info().special_multiplier;
    // ArmoredSlimeEntity.finalizeSpawn: the pickup roll comes first.
    let can_pick_up = view.next_float() < 0.55 * mult;
    let mut metal = false;
    let mut helmet: Option<StackData> = None;
    let mut enchant_level: Option<i32> = None;
    match kind.as_str() {
        "tconstruct:sky_slime" | "tconstruct:ender_slime" => {
            let (rolled_metal, built) = travelers_plate_rolls(view, &kind, mult);
            metal = rolled_metal;
            helmet = built;
        }
        _ => {
            if let Some((stack, level)) = terracube_rolls(view, mult) {
                helmet = Some(stack);
                enchant_level = Some(level);
            }
        }
    }
    // The Halloween pumpkin, in an empty head slot (no subclass helmet was chosen), drop chance 0.
    let mut halloween: Option<StackData> = None;
    if helmet.is_none() && is_halloween() && view.next_float() < 0.25 {
        let item = if view.next_float() < 0.1 { "minecraft:jack_o_lantern" } else { "minecraft:carved_pumpkin" };
        halloween = Some(StackData { item: item.into(), count: 1, ..Default::default() });
    }
    Ok(Some(Box::new(move |completion: &mut dyn MobSpawnCompletion| {
        completion.set_can_pick_up_loot(can_pick_up)?;
        if metal { completion.set_attached("tconstruct:metal", "true", true)?; }
        if let Some(stack) = &helmet { completion.set_equipment("head", stack.clone())?; }
        if let Some(level) = enchant_level { completion.enchant_equipment("head", level)?; }
        if let Some(stack) = &halloween {
            completion.set_equipment("head", stack.clone())?;
            completion.set_drop_chance("head", 0.0)?;
        }
        Ok(())
    })))
}

/// ArmoredSlimeEntity.remove's bookkeeping, as the split callback: the children inherit name, no-AI,
/// invulnerability and the metal flag; one random child receives the head stack, and each other child
/// copies it at 0.25 when the parent's head drop chance is below 1. Persistence, size and position are
/// the game's own split work, as in the original.
pub(crate) struct SlimeSplit;
impl MobSplitCallback for SlimeSplit {
    fn inherit(&self, view: &mut MobSplitInheritance<'_>) -> Result<(), String> {
        if !matches!(view.context.parent.kind.as_str(), "tconstruct:sky_slime" | "tconstruct:ender_slime" | "tconstruct:terracube") { return Ok(()); }
        let metal = view.context.parent.get_attached(metal_attachment());
        let helmet_index = if view.head.is_some() { view.random.next_int(view.children.len() as i32)? } else { -1 };
        for (index, child) in view.children.iter_mut().enumerate() {
            if metal { child.info().set_attached(metal_attachment(), true); }
            child.set_custom_name(view.custom_name.clone())?;
            child.set_no_ai(view.no_ai)?;
            child.set_invulnerable(view.invulnerable)?;
            if let Some(head) = &view.head {
                if index as i32 == helmet_index {
                    child.set_equipment("head", head.clone())?;
                } else if view.head_drop_chance < 1.0 && view.random.next_float() < 0.25 {
                    child.set_equipment("head", head.clone())?;
                }
            }
        }
        Ok(())
    }
}

/// The ender slime teleports waiting to land: the damage event records them, the slime's next tick
/// applies them (see the module documentation for how this differs from the original's immediacy).
#[derive(Default)]
struct TeleportQueue {
    /// The live ender slimes, by entity id.
    slimes: BTreeSet<u32>,
    /// Ender slimes that were hurt and teleport themselves.
    self_pending: BTreeSet<u32>,
    /// Victims of an ender slime's hit, by the slime's entity id.
    victims: BTreeMap<u32, Vec<u32>>,
}
static TELEPORTS: Mutex<TeleportQueue> = Mutex::new(TeleportQueue { slimes: BTreeSet::new(), self_pending: BTreeSet::new(), victims: BTreeMap::new() });
static TELEPORT_RNG: Mutex<u64> = Mutex::new(0x9E37_79B9_7F4A_7C15);

/// A xorshift64* draw for the teleport destinations (the helper's own stream in the original).
fn teleport_draw() -> u64 {
    let mut state = TELEPORT_RNG.lock().expect("teleport RNG lock poisoned");
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

fn teleport_double() -> f64 { (teleport_draw() >> 11) as f64 / (1u64 << 53) as f64 }

/// TeleportHelper.randomNearbyTeleport's destination: diameter 16 around the origin, the vertical
/// clamped to the dimension's own build range (the original clamps to the dimension's
/// `minBuildHeight` and logical height: the overworld −64..319, the nether and the end 0..255).
pub(crate) fn teleport_destination(origin: [f64; 3], dimension: &str, dx: f64, dy: i32, dz: f64) -> [f64; 3] {
    let (min_y, max_y) = match dimension {
        "minecraft:the_nether" | "minecraft:the_end" => (0.0, 255.0),
        _ => (-64.0, 319.0),
    };
    [origin[0] + (dx - 0.5) * 16.0, (origin[1] + dy as f64).clamp(min_y, max_y), origin[2] + (dz - 0.5) * 16.0]
}

/// Whether a block is a liquid a teleport refuses to land in: the vanilla fluid blocks and
/// this port's own fluid blocks (the content names every fluid block `<fluid>_fluid`).
fn is_liquid(block: &Identifier) -> bool {
    let id = block.to_string();
    matches!(id.as_str(), "minecraft:water" | "minecraft:lava" | "minecraft:bubble_column") || id.ends_with("_fluid")
}

/// TeleportHelper's landing test, as far as the world reach can state it: the two blocks the
/// body would occupy (feet and head) must hold no liquid and nothing solid — air, or a block
/// the game calls replaceable. An unloaded destination refuses, as a collision there cannot
/// be ruled out (the original's `randomTeleport` fails the same way).
fn teleport_lands(world: &World, dest: [f64; 3]) -> bool {
    let x = dest[0].floor() as i32;
    let y = dest[1].floor() as i32;
    let z = dest[2].floor() as i32;
    for dy in 0..2 {
        let Some(state) = world.get_block_state([x, y + dy, z]) else { return false };
        if is_liquid(&state.block) { return false; }
        if state.block.to_string() != "minecraft:air" && !state.is_in_tag("minecraft:replaceable") { return false; }
    }
    true
}

/// TeleportHelper.randomNearbyTeleport: up to 16 candidate positions around the entity; the
/// first that lands moves it, with the slime_teleport sound at both ends. A passenger leaves
/// its vehicle first (`LivingEntity.randomTeleport` stops riding before it moves).
fn random_nearby_teleport(id: u32, dimension: &str) {
    let Some(origin) = EntityReach::position_of(id as u64) else { return };
    if EntityReach::vehicle_of(id as u64).is_some() { EntityReach::stop_riding(id as u64); }
    let world = World::new(dimension);
    for _ in 0..16 {
        let destination = teleport_destination(origin, dimension, teleport_double(), (teleport_draw() % 16) as i32 - 8, teleport_double());
        if teleport_lands(&world, destination) {
            EntityReach::teleport(id as u64, destination);
            world.play_sound("tconstruct:slime_teleport", origin, 1.0, 1.0);
            world.play_sound("tconstruct:slime_teleport", destination, 1.0, 1.0);
            return;
        }
    }
}

/// EnderSlimeEntity.actuallyHurt and doEnchantDamageEffects, recorded from the damage event.
fn ender_damage(hit: &DamageModification) {
    if hit.amount.get() <= 0.0 { return; }
    let mut queue = TELEPORTS.lock().expect("teleport queue lock poisoned");
    if hit.damage.kind == "tconstruct:ender_slime" && hit.damage.entity > 0 {
        queue.self_pending.insert(hit.damage.entity as u32);
    }
    if hit.damage.attacker > 0 && hit.damage.entity > 0 && hit.damage.attacker != hit.damage.entity
        && queue.slimes.contains(&(hit.damage.attacker as u32)) {
        queue.victims.entry(hit.damage.attacker as u32).or_default().push(hit.damage.entity as u32);
    }
}

fn ender_load(info: &EntityInfo) {
    if info.kind == "tconstruct:ender_slime" {
        TELEPORTS.lock().expect("teleport queue lock poisoned").slimes.insert(info.id);
    }
}

fn ender_unload(info: &EntityInfo) {
    if info.kind == "tconstruct:ender_slime" {
        let mut queue = TELEPORTS.lock().expect("teleport queue lock poisoned");
        queue.slimes.remove(&info.id);
        queue.self_pending.remove(&info.id);
        queue.victims.remove(&info.id);
    }
}

fn ender_tick(info: &EntityInfo) {
    if info.kind != "tconstruct:ender_slime" { return; }
    let (teleport_self, victims) = {
        let mut queue = TELEPORTS.lock().expect("teleport queue lock poisoned");
        (queue.self_pending.remove(&info.id), queue.victims.remove(&info.id).unwrap_or_default())
    };
    if teleport_self { random_nearby_teleport(info.id, &info.dimension); }
    for victim in victims { random_nearby_teleport(victim, &info.dimension); }
}

/// The current expansion of the `tconstruct:slime_spawn/earth` block tag (the crystal blocks plus
/// the earth slimy grass variants), for the vanilla slime's OR placement; pinned against the
/// shipped tag JSONs by the domain tests, like the derived types' expansions in `TYPES`.
pub(crate) const EARTH_SPAWN_BLOCKS: &[&str] = &["tconstruct:earth_slime_crystal_block", "tconstruct:budding_earth_slime_crystal",
    "tconstruct:earth_earth_slime_grass", "tconstruct:earth_sky_slime_grass", "tconstruct:earth_ichor_slime_grass",
    "tconstruct:earth_ender_slime_grass", "tconstruct:earth_vanilla_slime_grass"];

/// SlimePlacementPredicate over the type's slime_spawn tag expansion: never on peaceful, always from
/// a spawner, else the block below is in the tag.
pub(crate) fn tag_placement(context: &SpawnPlacementContext, blocks: &[&str]) -> bool {
    if context.peaceful { return false; }
    if context.reason == "spawner" { return true; }
    let below = context.block_below.to_string();
    blocks.contains(&below.as_str())
}

/// TerracubeEntity.canSpawnHere: never on peaceful, always from a spawner, in water, else a valid
/// spawn block in the dark.
pub(crate) fn terracube_placement(context: &SpawnPlacementContext) -> bool {
    if context.peaceful { return false; }
    if context.reason == "spawner" { return true; }
    // TerracubeEntity.canSpawnHere tests both fluids against FluidTags.WATER, whose members are the
    // still and the flowing water fluids; the context reports the fluid's own id, so both are named.
    let water = Identifier::parse("minecraft:water").expect("fluid id");
    let flowing = Identifier::parse("minecraft:flowing_water").expect("fluid id");
    let in_water = |fluid: &Identifier| fluid == &water || fluid == &flowing;
    if in_water(&context.fluid_at) && in_water(&context.fluid_below) { return true; }
    context.block_below_valid_spawn && context.dark_enough
}

pub(crate) fn register() {
    let factory = Identifier::parse("minecraft:slime").expect("factory id");
    for slime in TYPES {
        let id = Identifier::new("tconstruct", slime.name).expect("entity type id");
        EntityTypeRegistry::register_derived(id.clone(), factory.clone(), 2.04, 2.04, slime.tracking, true)
            .unwrap_or_else(|error| panic!("registering tconstruct:{}: {error}", slime.name));
        let mut behavior = LivingEntityBehavior::new(id.clone())
            .attribute_bases_on_load(armor_base)
            .equipment_drop_chance(head_drop_chance);
        for modifier in class_modifiers() {
            behavior = behavior.transient_attribute_modifier(modifier);
        }
        behavior = match slime.name {
            "sky_slime" => behavior.jump_power(sky_jump).cause_fall_damage(sky_fall),
            "terracube" => behavior.jump_power(terracube_jump).attack_damage(terracube_attack).calculate_fall_damage(terracube_fall),
            _ => behavior,
        };
        LivingEntityBehaviors::register(behavior)
            .unwrap_or_else(|error| panic!("registering the behaviour of tconstruct:{}: {error}", slime.name));
        // The placement predicates of the derived types, on the registry's OR side (the original
        // registers them with Operation.OR — see the module docs).
        if slime.spawn_tag.is_some() {
            let blocks = slime.spawn_blocks;
            SpawnPlacementRegistry::register_or(id, move |context| tag_placement(context, blocks))
                .unwrap_or_else(|error| panic!("registering the placement of tconstruct:{}: {error}", slime.name));
        }
    }
    // The vanilla slime's own OR placement: TinkerWorld registers the same predicate on
    // EntityType.SLIME itself, over the earth tag.
    SpawnPlacementRegistry::register_or(Identifier::parse("minecraft:slime").expect("entity type id"), move |context| tag_placement(context, EARTH_SPAWN_BLOCKS))
        .unwrap_or_else(|error| panic!("registering the placement of minecraft:slime: {error}"));
    SpawnPlacementRegistry::register(Identifier::parse("tconstruct:terracube").expect("entity type id"), terracube_placement)
        .unwrap_or_else(|error| panic!("registering the placement of tconstruct:terracube: {error}"));
    MobSpawnCallback::register(Identifier::parse("tconstruct:slime_finalization").expect("callback id"), finalize_spawn)
        .unwrap_or_else(|error| panic!("registering the slime spawn finalization: {error}"));
    MobSplitCallbacks::register("tconstruct:slime_inheritance", SlimeSplit)
        .unwrap_or_else(|error| panic!("registering the slime split inheritance: {error}"));
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(ender_damage);
    ServerEntityEvents::ENTITY_LOAD.register(ender_load);
    ServerEntityEvents::ENTITY_UNLOAD.register(ender_unload);
    ServerEntityEvents::ENTITY_TICK_POST.register(ender_tick);
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::world_api_v1::{with_world, MemoryWorldOps};

    fn lands(ops: &mut MemoryWorldOps, dest: [f64; 3]) -> bool {
        with_world(ops, || teleport_lands(&World::new("minecraft:overworld"), dest))
    }

    #[test]
    fn a_teleport_lands_in_open_air_and_refuses_solids_and_liquids() {
        let mut ops = MemoryWorldOps::default();
        assert!(lands(&mut ops, [0.5, 64.0, 0.5]), "open air lands");
        // A solid at the feet refuses.
        ops.blocks.insert(("minecraft:overworld".into(), [0, 64, 0]), "minecraft:stone".into());
        assert!(!lands(&mut ops, [0.5, 64.0, 0.5]), "a solid at the feet refuses");
        // A solid at the head refuses.
        let mut ops = MemoryWorldOps::default();
        ops.blocks.insert(("minecraft:overworld".into(), [0, 65, 0]), "minecraft:stone".into());
        assert!(!lands(&mut ops, [0.5, 64.0, 0.5]), "a solid at the head refuses");
        // Water, lava and a mod fluid block refuse.
        for fluid in ["minecraft:water", "minecraft:lava", "tconstruct:molten_iron_fluid"] {
            let mut ops = MemoryWorldOps::default();
            ops.blocks.insert(("minecraft:overworld".into(), [0, 64, 0]), fluid.into());
            assert!(!lands(&mut ops, [0.5, 64.0, 0.5]), "{fluid} refuses");
        }
        // A replaceable block (tall grass) lets the teleport land.
        let mut ops = MemoryWorldOps::default();
        ops.blocks.insert(("minecraft:overworld".into(), [0, 64, 0]), "minecraft:tall_grass".into());
        ops.tags.push(("minecraft:tall_grass".into(), "minecraft:replaceable".into()));
        assert!(lands(&mut ops, [0.5, 64.0, 0.5]), "a replaceable block lands");
    }

    #[test]
    fn liquid_blocks_are_the_vanilla_fluids_and_the_ports_fluid_blocks() {
        for id in ["minecraft:water", "minecraft:lava", "minecraft:bubble_column", "tconstruct:molten_gold_fluid", "tconstruct:venom_fluid"] {
            assert!(is_liquid(&Identifier::parse(id).unwrap()), "{id}");
        }
        for id in ["minecraft:air", "minecraft:stone", "tconstruct:earth_slime_block"] {
            assert!(!is_liquid(&Identifier::parse(id).unwrap()), "{id}");
        }
    }
}
