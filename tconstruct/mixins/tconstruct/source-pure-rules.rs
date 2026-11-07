// Dependency-free source rules shared by existing Mixins module registrations.
// Keep each public entry point and registration namespace stable. These rules
// use no parent-module or Horizon imports, so every destination is equivalent.
// Section test modules are named to avoid collisions in the shared source.

// ---- source-anvil-capacity.rs ----
// Source TinkerStationBlock slot counts include the tool slot.
pub(super) fn input_capacity(block: &str) -> Option<usize> {
    match block {
        "tconstruct:tinker_station" => Some(3),
        "tconstruct:modifier_worktable" => Some(2),
        "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil" => Some(5),
        _ => None,
    }
}

#[cfg(test)]
mod anvil_capacity_tests {
    use super::input_capacity;

    #[test]
    fn source_tables_have_distinct_input_capacities() {
        assert_eq!(input_capacity("tconstruct:tinker_station"), Some(3));
        assert_eq!(input_capacity("tconstruct:tinkers_anvil"), Some(5));
        assert_eq!(input_capacity("tconstruct:scorched_anvil"), Some(5));
        assert_eq!(input_capacity("tconstruct:modifier_worktable"), Some(2));
        assert_eq!(input_capacity("tconstruct:part_builder"), None);
    }
}

// ---- source-area-shape.rs ----
// Original side-hit box iterator for the unexpanded 3x3 hammer/shovel plane.
pub(super) fn plane(origin: [i32; 3], face: [i32; 3], yaw: f32) -> Option<Vec<[i32; 3]>> {
    let horizontal = match ((yaw / 90.0 + 0.5).floor() as i32).rem_euclid(4) {
        0 => [0, 0, 1], 1 => [-1, 0, 0], 2 => [0, 0, -1], _ => [1, 0, 0],
    };
    let (width, height) = match face {
        [0, 1, 0] | [0, -1, 0] => ([-horizontal[2], 0, horizontal[0]], horizontal),
        [0, 0, 1] | [0, 0, -1] | [1, 0, 0] | [-1, 0, 0] => ([face[2], 0, -face[0]], [0, 1, 0]),
        _ => return None,
    };
    let mut positions = Vec::with_capacity(8);
    for h in -1i32..=1 { for w in -1i32..=1 {
        if w == 0 && h == 0 { continue; }
        let mut pos = origin;
        for axis in 0..3 { pos[axis] = pos[axis].checked_add(width[axis]*w + height[axis]*h)?; }
        positions.push(pos);
    }}
    Some(positions)
}

pub(super) fn compatible_hardness(center: f32, candidate: f32) -> bool {
    candidate >= 0.0 && if center == 0.0 { candidate == 0.0 } else { candidate / center <= 3.0 }
}

#[cfg(test)]
mod area_shape_tests {
    use super::*;
    #[test] fn all_faces_have_eight_positions_on_hit_plane() {
        for face in [[0,1,0],[0,-1,0],[0,0,1],[0,0,-1],[1,0,0],[-1,0,0]] {
            for yaw in [0.0,90.0,180.0,270.0] {
                let p = plane([10,20,30],face,yaw).unwrap();
                assert_eq!(p.len(),8); assert!(!p.contains(&[10,20,30]));
                assert!(p.iter().all(|p| (p[0]-10)*face[0]+(p[1]-20)*face[1]+(p[2]-30)*face[2]==0));
                assert_eq!(p.iter().copied().collect::<std::collections::HashSet<_>>().len(),8);
            }
        }
    }
    #[test] fn source_traversal_and_hardness() {
        assert_eq!(plane([0,0,0],[0,0,1],0.0).unwrap()[0],[-1,-1,0]);
        assert!(compatible_hardness(2.0,6.0)); assert!(!compatible_hardness(2.0,6.01));
        assert!(compatible_hardness(0.0,0.0)); assert!(!compatible_hardness(0.0,0.1));
        assert!(!compatible_hardness(2.0,-1.0)); assert!(plane([0,0,0],[0,0,0],0.0).is_none());
    }
}

// ---- source-bow-start.rs ----
/// Client prediction only. The server remains authoritative for the shot.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Decision { Pass, Reject, Handled, Start }

pub(super) fn decide(item: &str, charged: bool, broken: bool, using: bool, creative: bool, ammo: bool) -> Decision {
    match item {
        "tconstruct:crossbow" if charged => return Decision::Handled,
        "tconstruct:longbow" | "tconstruct:crossbow" => {},
        _ => return Decision::Pass,
    }
    if broken { return Decision::Reject; }
    if using { return Decision::Handled; }
    if creative || ammo { Decision::Start } else { Decision::Reject }
}

#[cfg(test)]
mod bow_start_tests {
    use super::*;
    #[test]
    fn start_reject_and_other_items_match_use_result() {
        assert_eq!(decide("tconstruct:longbow", false, false, false, false, true), Decision::Start);
        assert_eq!(decide("tconstruct:crossbow", false, false, false, true, false), Decision::Start);
        assert_eq!(decide("tconstruct:longbow", false, false, false, false, false), Decision::Reject);
        assert_eq!(decide("tconstruct:longbow", false, true, false, true, true), Decision::Reject);
        assert_eq!(decide("minecraft:bow", false, false, false, false, true), Decision::Pass);
    }
    #[test]
    fn held_use_and_charged_crossbow_do_not_start_twice() {
        assert_eq!(decide("tconstruct:crossbow", false, false, true, false, false), Decision::Handled);
        assert_eq!(decide("tconstruct:crossbow", true, false, false, false, false), Decision::Handled);
    }
}

// ---- source-campfire-select.rs ----
/// The source pathing action can extinguish either lit vanilla campfire.
pub(super) fn is_lit_campfire(name: &str, lit: Option<&str>) -> bool {
    matches!(name, "minecraft:campfire" | "minecraft:soul_campfire") && lit == Some("true")
}

#[cfg(test)]
mod campfire_select_tests {
    use super::*;
    #[test]
    fn only_lit_campfires_are_selected() {
        for name in ["minecraft:campfire", "minecraft:soul_campfire"] {
            assert!(is_lit_campfire(name, Some("true")));
            assert!(!is_lit_campfire(name, Some("false")));
            assert!(!is_lit_campfire(name, None));
        }
        assert!(!is_lit_campfire("minecraft:furnace", Some("true")));
    }
}

// ---- source-cast-info.rs ----
// PartCastItem registrations in TinkerSmeltery; the three cast variants
// share a supplier for the corresponding material item and its recipe cost.
pub(super) const PARTS: [&str; 20] = [
    "repair_kit", "pick_head", "small_axe_head", "small_blade", "adze_head",
    "hammer_head", "broad_axe_head", "broad_blade", "large_plate", "tool_binding",
    "tough_binding", "tool_handle", "tough_handle", "bow_limb", "bow_grip",
    "helmet_plating", "chestplate_plating", "leggings_plating", "boots_plating", "maille",
];

pub(super) fn cast_index(item: &str) -> Option<usize> {
    let short = item.strip_prefix("tconstruct:")?;
    let base = short.strip_suffix("_red_sand_cast")
        .or_else(|| short.strip_suffix("_sand_cast"))
        .or_else(|| short.strip_suffix("_cast"))?;
    PARTS.iter().position(|part| *part == base)
}

pub(super) fn parse_costs(values: &[String]) -> Option<Vec<i32>> {
    if values.len() != PARTS.len() { return None; }
    values.iter().map(|s| s.parse::<i32>().ok().filter(|n| *n >= 0)).collect()
}

#[cfg(test)]
mod cast_info_tests {
    use super::*;
    #[test]
    fn shared_part_cast_mapping_and_complete_frame() {
        let i = cast_index("tconstruct:pick_head_cast").unwrap();
        assert_eq!(cast_index("tconstruct:pick_head_sand_cast"), Some(i));
        assert_eq!(cast_index("tconstruct:pick_head_red_sand_cast"), Some(i));
        assert_eq!(cast_index("tconstruct:ingot_cast"), None);
        let mut values = vec!["0".to_owned(); PARTS.len()];
        values[i] = "2147483647".to_owned();
        assert_eq!(parse_costs(&values).unwrap()[i], i32::MAX);
        assert!(parse_costs(&values[..19]).is_none());
        values[i] = "-1".to_owned();
        assert!(parse_costs(&values).is_none());
    }
}

// ---- source-dwarven.rs ----
// DwarvenModifier.getBoost at TConstruct f4e77d43.
// `bonus` is the source stat multiplier times 6.0 for mining or 0.05 for velocity.
pub(super) fn boost(y: f32, max_build_height: i32, level: i32, base: f32, bonus: f32) -> f32 {
    if y < 64.0 {
        let scale = ((64.0 - y) / 64.0).clamp(0.0, 2.0);
        return base + level.max(0) as f32 * scale * bonus;
    }
    let baseline = (max_build_height as f32 - 192.0).max(96.0);
    if y > baseline {
        if y >= baseline + 128.0 { return base * 0.25; }
        return base * (1.0 - (y - baseline) / 128.0 * 0.75);
    }
    base
}

#[cfg(test)]
mod dwarven_tests {
    use super::boost;
    #[test]
    fn depth_boost_and_high_altitude_penalty_match_source() {
        assert_eq!(boost(64.0, 320, 1, 4.0, 6.0), 4.0);
        assert_eq!(boost(0.0, 320, 1, 4.0, 6.0), 10.0);
        assert_eq!(boost(-64.0, 320, 1, 4.0, 6.0), 16.0);
        assert_eq!(boost(-256.0, 320, 1, 4.0, 6.0), 16.0);
        assert_eq!(boost(128.0, 320, 1, 4.0, 6.0), 4.0);
        assert_eq!(boost(192.0, 320, 1, 4.0, 6.0), 2.5);
        assert_eq!(boost(256.0, 320, 1, 4.0, 6.0), 1.0);
    }
    #[test]
    fn source_uses_same_height_curve_for_velocity_and_short_worlds() {
        assert!((boost(0.0, 128, 2, 1.0, 0.05) - 1.1).abs() < 0.00001);
        assert_eq!(boost(96.0, 128, 1, 4.0, 6.0), 4.0);
        assert_eq!(boost(224.0, 128, 1, 4.0, 6.0), 1.0);
    }
}

// ---- source-haste-source.rs ----
// Exact pinned MIT source definition. A pack override, even one differing only
// in whitespace, remains unknown until full modifier loading is implemented.
pub(super) const PINNED_HASTE: &[u8] = br#"{
  "level_display": {
    "type": "tconstruct:unique",
    "first_unique": false,
    "unique_until": 5
  },
  "modules": [
    {
      "type": "tconstruct:stat_boost",
      "each_level": 4.0,
      "operation": "add",
      "stat": "tconstruct:mining_speed"
    },
    {
      "type": "tconstruct:attribute",
      "attribute": "tconstruct:player.mining_speed_multiplier",
      "each_level": 0.1,
      "operation": "multiply_total",
      "slots": [
        "mainhand",
        "offhand",
        "feet",
        "legs",
        "chest",
        "head"
      ],
      "tool": {
        "type": "mantle:inverted",
        "inverted_type": "mantle:tag",
        "tag": "tconstruct:modifiable/harvest"
      }
    }
  ]
}"#;

/// None is deliberately distinct from an empty/negative harvest tag.
pub(super) fn source_haste_trait_allowed(is_harvest: Option<bool>, source: Option<&[u8]>) -> bool {
    is_harvest == Some(false) && source == Some(PINNED_HASTE)
}


#[cfg(test)] mod haste_source_tests {
    use super::*;
    #[test]
    fn trait_requires_local_actor_fresh_tag_and_exact_source() {
        assert!(source_haste_trait_allowed(Some(false), Some(PINNED_HASTE)));
        assert!(!source_haste_trait_allowed(None, Some(PINNED_HASTE)));
        assert!(!source_haste_trait_allowed(Some(true), Some(PINNED_HASTE)));
        assert!(!source_haste_trait_allowed(Some(false), None));
        assert!(!source_haste_trait_allowed(Some(false), Some(b"{}")));
    }
}

// ---- source-materials-book-spread.rs ----
// usize::MAX represents the cover without adding a synthetic wire page.
pub(crate) const COVER: usize = usize::MAX;
pub(crate) fn turn(page: usize, count: usize, forward: bool) -> usize {
    if page == COVER { return if forward && count > 0 { 0 } else { COVER }; }
    let base = (page / 2) * 2;
    if forward { if base.saturating_add(2) < count { base + 2 } else { base } }
    else if base == 0 { COVER } else { base - 2 }
}
pub(crate) fn remember(history: &mut Option<usize>, current: usize, target: i32, count: usize) {
    if target > 0 && (target as usize) <= count && (target as usize - 1) != current { *history = Some(current); }
}

#[cfg(test)]
mod materials_book_spread_tests {
    use super::*;
    #[test]
    fn link_history_remembers_only_valid_page_changes() {
        let mut history = None;
        remember(&mut history, 4, 9, 12);
        assert_eq!(history.take(), Some(4));
        remember(&mut history, 4, 5, 12);
        remember(&mut history, 4, 13, 12);
        assert_eq!(history, None);
    }
    #[test]
    fn cover_and_spreads_turn_in_both_directions() {
        assert_eq!(turn(COVER, 5, false), COVER);
        assert_eq!(turn(COVER, 5, true), 0);
        assert_eq!(turn(0, 5, false), COVER);
        assert_eq!(turn(1, 5, true), 2);
        assert_eq!(turn(3, 5, false), 0);
        assert_eq!(turn(4, 5, true), 4);
        assert_eq!(turn(COVER, 0, true), COVER);
    }
}

// ---- source-materials-book-stat.rs ----
// Source material stat display: percent boosts are stored as fractions.
pub(super) fn number(kind: &str, field: &str, value: f64) -> String {
    let percent = kind == "handle" || kind == "grip" && field == "durability";
    if percent {
        format!("{:+.0}%", value * 100.0)
    } else if matches!(kind, "limb" | "grip") && matches!(field, "draw_speed" | "velocity" | "accuracy") {
        format!("{value:+}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod materials_book_stat_tests {
    use super::number;
    #[test]
    fn handle_and_grip_percent_boosts_are_not_flat_stats() {
        assert_eq!(number("handle", "durability", 0.25), "+25%");
        assert_eq!(number("grip", "durability", -0.1), "-10%");
        assert_eq!(number("head", "durability", 60.0), "60");
        assert_eq!(number("limb", "velocity", 0.2), "+0.2");
    }
}

// ---- source-melee-knockback.rs ----
// Original tool definitions supply Knockback II on Excavator and I on
// Battlesign. The source attack context applies the attack-knockback
// attribute divided by two; this is in addition to native sprint/enchantments.
pub(super) fn builtin_bonus(item: &str) -> f32 {
    match item {
        "tconstruct:excavator" => 1.0,
        "tconstruct:battlesign" => 0.5,
        _ => 0.0,
    }
}

#[cfg(test)]
mod melee_knockback_tests {
    use super::*;
    #[test] fn source_builtin_levels() {
        assert_eq!(builtin_bonus("tconstruct:excavator"), 1.0);
        assert_eq!(builtin_bonus("tconstruct:battlesign"), 0.5);
        assert_eq!(builtin_bonus("minecraft:iron_axe"), 0.0);
    }
}

// ---- source-melee-stat-ops.rs ----
// Source StatBoostModule operations for the supported melee stat traits.
// Callers provide authoritative trait levels and source tool-tag membership.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MeleeInputs {
    pub damage: f32,
    pub damage_add_multiplier: f32,
    pub speed: f32,
    pub ductile: i32,
    pub solid: i32,
    pub pierce: i32,
    pub lightweight: i32,
    pub vintage: i32,
    pub sharpness: f32,
    pub swiftstrike: f32,
    pub melee: bool,
    pub melee_weapon: bool,
}

pub(super) fn melee_stats(v: MeleeInputs) -> (f32, f32) {
    let damage = if v.melee {
        (v.damage + (0.5 * v.pierce.max(0) as f32 + 0.75 * v.sharpness.max(0.0)) * v.damage_add_multiplier)
            * (1.0 + 0.05 * v.ductile.max(0) as f32 + 0.15 * v.solid.max(0) as f32)
    } else { v.damage };
    let speed = if v.melee_weapon {
        v.speed * (1.0 + 0.08 * v.lightweight.max(0) as f32 + 0.05 * v.swiftstrike.max(0.0))
            * (1.0 - 0.1 * v.vintage.max(0) as f32)
    } else { v.speed };
    (if v.melee { damage.clamp(0.0, 2048.0) } else { damage },
     if v.melee_weapon { speed.clamp(0.0, 1024.0) } else { speed })
}

#[cfg(test)]
mod melee_stat_ops_tests {
    use super::*;
    #[test]
    fn source_operations_combine_from_base_once() {
        let v = MeleeInputs { damage: 5.0, damage_add_multiplier: 1.25, speed: 1.2, ductile: 2, solid: 1, pierce: 1, lightweight: 2, vintage: 1, sharpness: 0.5, swiftstrike: 0.5, melee: true, melee_weapon: true };
        let (damage, speed) = melee_stats(v);
        assert!((damage - 7.6171875).abs() < 0.0001);
        assert!((speed - 1.2798).abs() < 0.0001);
        assert_eq!(melee_stats(v), (damage, speed));
    }
    #[test]
    fn unsupported_tool_tags_do_not_receive_stat_effects() {
        let v = MeleeInputs { damage: 3.0, speed: 2.0, pierce: 2, solid: 1, lightweight: 3, vintage: 2, ..Default::default() };
        assert_eq!(melee_stats(v), (3.0, 2.0));
    }
}

// ---- source-melter-layout.rs ----
// The original menu has three vertically arranged inputs and an optional fuel
// slot from the heater immediately below it. Chest protocol padding is hidden.
pub(super) fn position(slot: usize) -> Option<(f32, f32)> {
    Some(match slot {
        0..=8 => (8.0 + slot as f32 * 18.0, 142.0),
        9..=35 => (8.0 + ((slot - 9) % 9) as f32 * 18.0, 84.0 + ((slot - 9) / 9) as f32 * 18.0),
        300..=302 => (22.0, 16.0 + (slot - 300) as f32 * 18.0),
        327 => (151.0, 32.0),
        303..=326 | 328..=353 => (-10000.0, -10000.0),
        _ => return None,
    })
}

pub(super) fn hidden(slot: usize) -> bool {
    matches!(slot, 303..=326 | 328..=353)
}

#[cfg(test)]
mod melter_layout_tests {
    use super::*;
    #[test]
    fn melter_and_heater_slots_match_source_without_exposing_padding() {
        assert_eq!((position(300), position(301), position(302)), (Some((22.0, 16.0)), Some((22.0, 34.0)), Some((22.0, 52.0))));
        assert_eq!(position(327), Some((151.0, 32.0)));
        assert!((303..=326).chain(328..=353).all(|slot| hidden(slot) && position(slot) == Some((-10000.0, -10000.0))));
        assert!(!hidden(327));
    }
}

// ---- source-menu-wire-cache.rs ----
/// Keep our framed menu data separate from Horizon's vanilla data-slot cache.
pub(super) fn changed(old: Option<&(i16, Vec<i16>)>, selection: i16, frame: &[i16]) -> (bool, bool) {
    (old.is_none_or(|(previous, _)| *previous != selection), old.is_none_or(|(_, previous)| previous != frame))
}

#[cfg(test)]
mod menu_wire_cache_tests {
    use super::*;
    #[test]
    fn selection_and_frame_changes_are_independent() {
        assert_eq!(changed(None, -1, &[1, 2]), (true, true));
        let old = (-1, vec![1, 2]);
        assert_eq!(changed(Some(&old), -1, &[1, 2]), (false, false));
        assert_eq!(changed(Some(&old), 0, &[1, 2]), (true, false));
        assert_eq!(changed(Some(&old), -1, &[1, 3]), (false, true));
    }
}

// ---- source-original-item-durability.rs ----
// Generated from pinned Minecraft 1.20.1 bytecode by
// tools/extract_original_item_durability.py. Unknown items remain unknown.
pub(super) fn max_damage(item: &str) -> Option<i32> {
    match item {
        "minecraft:bow" => Some(384),
        "minecraft:brush" => Some(64),
        "minecraft:carrot_on_a_stick" => Some(25),
        "minecraft:chainmail_boots" => Some(195),
        "minecraft:chainmail_chestplate" => Some(240),
        "minecraft:chainmail_helmet" => Some(165),
        "minecraft:chainmail_leggings" => Some(225),
        "minecraft:crossbow" => Some(465),
        "minecraft:diamond_axe" => Some(1561),
        "minecraft:diamond_boots" => Some(429),
        "minecraft:diamond_chestplate" => Some(528),
        "minecraft:diamond_helmet" => Some(363),
        "minecraft:diamond_hoe" => Some(1561),
        "minecraft:diamond_leggings" => Some(495),
        "minecraft:diamond_pickaxe" => Some(1561),
        "minecraft:diamond_shovel" => Some(1561),
        "minecraft:diamond_sword" => Some(1561),
        "minecraft:elytra" => Some(432),
        "minecraft:fishing_rod" => Some(64),
        "minecraft:flint_and_steel" => Some(64),
        "minecraft:golden_axe" => Some(32),
        "minecraft:golden_boots" => Some(91),
        "minecraft:golden_chestplate" => Some(112),
        "minecraft:golden_helmet" => Some(77),
        "minecraft:golden_hoe" => Some(32),
        "minecraft:golden_leggings" => Some(105),
        "minecraft:golden_pickaxe" => Some(32),
        "minecraft:golden_shovel" => Some(32),
        "minecraft:golden_sword" => Some(32),
        "minecraft:iron_axe" => Some(250),
        "minecraft:iron_boots" => Some(195),
        "minecraft:iron_chestplate" => Some(240),
        "minecraft:iron_helmet" => Some(165),
        "minecraft:iron_hoe" => Some(250),
        "minecraft:iron_leggings" => Some(225),
        "minecraft:iron_pickaxe" => Some(250),
        "minecraft:iron_shovel" => Some(250),
        "minecraft:iron_sword" => Some(250),
        "minecraft:leather_boots" => Some(65),
        "minecraft:leather_chestplate" => Some(80),
        "minecraft:leather_helmet" => Some(55),
        "minecraft:leather_leggings" => Some(75),
        "minecraft:netherite_axe" => Some(2031),
        "minecraft:netherite_boots" => Some(481),
        "minecraft:netherite_chestplate" => Some(592),
        "minecraft:netherite_helmet" => Some(407),
        "minecraft:netherite_hoe" => Some(2031),
        "minecraft:netherite_leggings" => Some(555),
        "minecraft:netherite_pickaxe" => Some(2031),
        "minecraft:netherite_shovel" => Some(2031),
        "minecraft:netherite_sword" => Some(2031),
        "minecraft:shears" => Some(238),
        "minecraft:shield" => Some(336),
        "minecraft:stone_axe" => Some(131),
        "minecraft:stone_hoe" => Some(131),
        "minecraft:stone_pickaxe" => Some(131),
        "minecraft:stone_shovel" => Some(131),
        "minecraft:stone_sword" => Some(131),
        "minecraft:trident" => Some(250),
        "minecraft:turtle_helmet" => Some(275),
        "minecraft:warped_fungus_on_a_stick" => Some(100),
        "minecraft:wooden_axe" => Some(59),
        "minecraft:wooden_hoe" => Some(59),
        "minecraft:wooden_pickaxe" => Some(59),
        "minecraft:wooden_shovel" => Some(59),
        "minecraft:wooden_sword" => Some(59),
        _ => None,
    }
}

// ---- source-part-material-id.rs ----
// Source ToolPartItem advanced material ID, restricted to the built-in part
// registrations. It is derived only from the stack, so no remote registry is
// guessed. The unknown sentinel and malformed IDs do not produce a line.
pub(super) fn source_part_material_id(item: &str, material: &str, advanced: bool, hidden: bool, display: bool) -> Option<String> {
    if !advanced || hidden || display { return None; }
    let part = item.strip_prefix("tconstruct:")?;
    if !["pick_head", "hammer_head", "small_axe_head", "broad_axe_head", "small_blade", "broad_blade", "adze_head", "large_plate", "bow_limb", "bow_grip", "bowstring", "arrow_head", "arrow_shaft", "fletching", "tool_binding", "tough_binding", "tool_handle", "tough_handle", "helmet_plating", "chestplate_plating", "leggings_plating", "boots_plating", "maille", "shield_core", "ribcage", "shell", "laces"].contains(&part) { return None; }
    if material.is_empty() { return None; }
    let (base, variant) = material.split_once('#').unwrap_or((material, ""));
    let (namespace, path) = base.split_once(':').unwrap_or(("minecraft", base));
    let namespace = if namespace.is_empty() { "minecraft" } else { namespace };
    if path.is_empty() || !namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
        || !path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
        || !variant.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c)) { return None; }
    let base = format!("{namespace}:{path}");
    if variant.is_empty() {
        if base == "tconstruct:unknown" { return None; }
        Some(base)
    } else { Some(format!("{base}#{variant}")) }
}

#[cfg(test)]
mod part_material_id_tests {
    use super::*;
    #[test]
    fn advanced_material_id_is_scoped_and_canonical() {
        for part in ["pick_head", "hammer_head", "small_axe_head", "broad_axe_head", "small_blade", "broad_blade", "adze_head", "large_plate", "bow_limb", "bow_grip", "bowstring", "arrow_head", "arrow_shaft", "fletching", "tool_binding", "tough_binding", "tool_handle", "tough_handle", "helmet_plating", "chestplate_plating", "leggings_plating", "boots_plating", "maille", "shield_core", "ribcage", "shell", "laces"] {
            assert_eq!(source_part_material_id(&format!("tconstruct:{part}"), "tconstruct:wood#spruce", true, false, false).as_deref(), Some("tconstruct:wood#spruce"));
        }
        for item in ["tconstruct:repair_kit", "tconstruct:fake_ingot", "tconstruct:pickaxe", "minecraft:pick_head"] {
            assert_eq!(source_part_material_id(item, "tconstruct:wood", true, false, false), None);
        }
        assert_eq!(source_part_material_id("tconstruct:pick_head", "wood", true, false, false).as_deref(), Some("minecraft:wood"));
        assert_eq!(source_part_material_id("tconstruct:pick_head", ":wood#", true, false, false).as_deref(), Some("minecraft:wood"));
        for bad in ["", "tconstruct:unknown", "Bad:wood", "tconstruct:", "tconstruct:wood#Bad", "tconstruct:wood#one#two", "tconstruct:wood:bad", "tconstruct:wood#bad:variant"] {
            assert_eq!(source_part_material_id("tconstruct:pick_head", bad, true, false, false), None);
        }
    }
    #[test]
    fn flags_suppress_details() {
        for (advanced, hidden, display) in [(false,false,false),(true,true,false),(true,false,true)] {
            assert_eq!(source_part_material_id("tconstruct:pick_head", "tconstruct:wood", advanced, hidden, display), None);
        }
    }
}

// ---- source-slime-seed-rules.rs ----
// Adapted from SlimeGrassSeedItem, Copyright (c) 2022 SlimeKnights,
// MIT license: tconstruct/third_party/tconstruct/LICENSE.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ResultKind {
    Grass(&'static str, &'static str),
    Vine(&'static str),
}

pub(super) fn conversion(item: &str, block: &str) -> Option<ResultKind> {
    let foliage = match item {
        "tconstruct:earth_slime_grass_seeds" => "earth",
        "tconstruct:sky_slime_grass_seeds" => "sky",
        "tconstruct:ender_slime_grass_seeds" => "ender",
        "tconstruct:blood_slime_grass_seeds" => "blood",
        "tconstruct:ichor_slime_grass_seeds" => "ichor",
        _ => return None,
    };
    if block == "minecraft:vine" {
        return match foliage {
            "sky" => Some(ResultKind::Vine("sky")),
            "ender" => Some(ResultKind::Vine("ender")),
            _ => None,
        };
    }
    let soil = match block {
        "minecraft:dirt" => "vanilla",
        "tconstruct:earth_slime_dirt" => "earth",
        "tconstruct:sky_slime_dirt" => "sky",
        "tconstruct:ender_slime_dirt" => "ender",
        "tconstruct:ichor_slime_dirt" => "ichor",
        _ => return None,
    };
    Some(ResultKind::Grass(foliage, soil))
}

// ---- source-transform-mode.rs ----
pub(super) fn mode(id:&str)->Option<(&'static str,u8)> {
    match id {
        "tconstruct:mattock"|"tconstruct:kama"|"tconstruct:scythe"=>Some(("minecraft:wooden_hoe",0)),
        "tconstruct:excavator"=>Some(("minecraft:wooden_shovel",1)),
        "tconstruct:hand_axe"|"tconstruct:broad_axe"=>Some(("minecraft:wooden_axe",2)),
        _=>None,
    }
}

#[cfg(test)]
mod transform_mode_tests {
    use super::*;
    #[test] fn source_tool_traits_choose_the_right_transformer() {
        for item in ["mattock","kama","scythe"] {assert_eq!(mode(&format!("tconstruct:{item}")),Some(("minecraft:wooden_hoe",0)));}
        assert_eq!(mode("tconstruct:excavator"),Some(("minecraft:wooden_shovel",1)));
        for item in ["hand_axe","broad_axe"] {assert_eq!(mode(&format!("tconstruct:{item}")),Some(("minecraft:wooden_axe",2)));}
        assert_eq!(mode("minecraft:wooden_axe"),None);
        assert_eq!(mode("tconstruct:pickaxe"),None);
    }
}
