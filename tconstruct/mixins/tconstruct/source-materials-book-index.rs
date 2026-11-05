// Never imply unsupported exploration acquisition routes are implemented.
pub(super) fn gadgetry_exploration_page(name: &str) -> bool {
    matches!(name, "cobalt_ore" | "cobalt_blasting")
}

// Keep long descriptions usable as index links without wrapping across entries.
// The host tag snapshot is a set, so retain the pinned source's built-in
// order explicitly. Unknown datapack additions follow in lexical order.
pub(super) fn puny_armor_order(id: &str) -> usize {
    const ORDER: &[&str] = &[
        "tconstruct:travelers_helmet", "tconstruct:travelers_chestplate",
        "tconstruct:travelers_leggings", "tconstruct:travelers_boots",
        "tconstruct:travelers_shield", "tconstruct:plate_helmet",
        "tconstruct:plate_chestplate", "tconstruct:plate_leggings",
        "tconstruct:plate_boots", "tconstruct:plate_shield",
    ];
    ORDER.iter().position(|known| *known == id).unwrap_or(usize::MAX)
}

// Mighty exposes bounded broad-tool pages, static advice and the adapted
// tool-tank page. The original screenshot and unsupported effects stay omitted.
pub(super) fn mighty_advice_page(name: &str) -> bool {
    matches!(name, "group_general" | "group_ranged" | "group_info" | "tanks" | "tips" | "designing")
}

// These source pages describe existing broad-tool families, not complete
// effect parity. Javelins and external additions are deliberately omitted.
pub(super) fn mighty_tool_guide_ready(id: &str, registered: bool, inputs_available: bool, source_tool: Option<&str>) -> bool {
    registered && inputs_available && source_tool == Some(id)
        && matches!(id, "tconstruct:sledge_hammer" | "tconstruct:vein_hammer" | "tconstruct:excavator"
            | "tconstruct:broad_axe" | "tconstruct:scythe" | "tconstruct:cleaver" | "tconstruct:longbow")
}

// Expose source pages whose described basics have an existing limited path.
// Adapted pages explicitly distinguish missing operations from supported basics.
pub(super) fn mighty_smeltery_page(name: &str) -> bool {
    matches!(name, "features" | "controller" | "structure" | "structurereq" | "alloying" | "blazing_blood_barter" | "sgui1" | "sgui2" | "seared_tanks" | "seared_blocks" | "seared_chute" | "seared_ladder" | "seared_glass" | "tinted_glass" | "soul_glass" | "seared_drain" | "seared_duct")
}

// The source repeats the first page name. Match its full identity so an
// unrelated replacement cannot inherit the qualified reference presentation.
pub(super) fn mighty_entity_melting_page(name: &str, path: &str, kind: &str) -> bool {
    matches!((name, path, kind),
        ("entitymelting", "entitymelting/intro.json", "mantle:image_text")
        | ("entitymelting", "entitymelting/entitymelting.json", "mantle:text")
        | ("noteableentities", "entitymelting/notable.json", "mantle:text"))
}

// Never show source advice without the reader's partial-coverage notice.
pub(super) fn mighty_advice_ready(labels: [Option<&str>; 3]) -> bool {
    labels.into_iter().all(|label| label.is_some_and(|text| !text.trim().is_empty()))
}

// The pinned source has six overview pages and five armor subgroups without
// prose. Keep their identities bounded; actual membership comes from live tags.
pub(super) fn mighty_ability_group(name: &str) -> Option<(&'static str, Option<&'static str>)> {
    Some(match name {
        "group_general" => ("tconstruct:abilities/general", Some("abilities/general.json")),
        "group_interact" => ("tconstruct:abilities/interaction", Some("abilities/interact.json")),
        "group_melee" => ("tconstruct:abilities/melee", Some("abilities/melee.json")),
        "group_harvest" => ("tconstruct:abilities/harvest", Some("abilities/harvest.json")),
        "group_ranged" => ("tconstruct:abilities/ranged", Some("abilities/ranged.json")),
        "group_armor" => ("tconstruct:abilities/armor/general", Some("abilities/armor.json")),
        "group_armor.helmet" => ("tconstruct:abilities/armor/helmet", None),
        "group_armor.chestplate" => ("tconstruct:abilities/armor/chestplate", None),
        "group_armor.leggings" => ("tconstruct:abilities/armor/leggings", None),
        "group_armor.boots" => ("tconstruct:abilities/armor/boots", None),
        "group_armor.shield" => ("tconstruct:abilities/armor/shield", None),
        _ => return None,
    })
}

// Defense shares the bounded live recipe projection, but never manufactures
// defense-slot support or gameplay effects from a source category label.
pub(super) fn mighty_defense_group(name: &str) -> Option<(&'static str, Option<&'static str>)> {
    Some(match name {
        "group_protection" => ("tconstruct:defense/protection", Some("defense/protection.json")),
        "group_special" => ("tconstruct:defense/special", Some("defense/special.json")),
        _ => return None,
    })
}

// Puny has five overview pages plus Bonus Damage and four armor subgroups.
// Like Mighty, group identity is fixed while recipe membership stays live.
pub(super) fn puny_upgrade_group(name: &str) -> Option<(&'static str, Option<&'static str>)> {
    Some(match name {
        "group_general" => ("tconstruct:upgrades/general", Some("upgrades/general.json")),
        "group_melee" => ("tconstruct:upgrades/melee", Some("upgrades/melee.json")),
        "group_damage" => ("tconstruct:upgrades/damage", None),
        "group_harvest" => ("tconstruct:upgrades/harvest", Some("upgrades/harvest.json")),
        "group_ranged" => ("tconstruct:upgrades/ranged", Some("upgrades/ranged.json")),
        "group_armor" => ("tconstruct:upgrades/armor/general", Some("upgrades/armor.json")),
        "group_armor.helmet" => ("tconstruct:upgrades/armor/helmet", None),
        "group_armor.chestplate" => ("tconstruct:upgrades/armor/chestplate", None),
        "group_armor.leggings" => ("tconstruct:upgrades/armor/leggings", None),
        "group_armor.boots" => ("tconstruct:upgrades/armor/boots", None),
        _ => return None,
    })
}

// Damage Tools is an ordinary illustrated page between the first two groups.
pub(super) fn puny_slotless_group(name: &str) -> Option<(&'static str, &'static str)> {
    Some(match name {
        "group_slotless" => ("tconstruct:slotless/general", "slotless/slotless.json"),
        "group_bonus_upgrades" => ("tconstruct:slotless/bonus", "slotless/bonus_upgrades.json"),
        "group_cosmetic" => ("tconstruct:slotless/cosmetic", "slotless/cosmetic.json"),
        _ => return None,
    })
}

// The Encyclopedia uses different keys from the progression books. Only its
// four single-type detailed material chapters share this renderer. Slimesuit
// materials need their own five-type treatment and must not fall through.
pub(super) fn encyclopedia_material_chapter(name: &str, data: &str, kind: &str, detailed: bool) -> Option<&'static str> {
    if data != "no-load" || !detailed { return None; }
    match (name, kind) {
        ("materials_harvest", "tconstruct:melee_harvest") => Some("melee_harvest_materials"),
        ("materials_ranged", "tconstruct:ranged") => Some("ranged_materials"),
        ("materials_ammo", "tconstruct:ammo") => Some("ammo_materials"),
        ("materials_armor", "tconstruct:armor") => Some("armor_materials"),
        _ => None,
    }
}

// Encyclopedia has a separate Slimesuit chapter, unlike the existing broad
// armor projection used by the progression books. Do not mix those stat types.
pub(super) const ENCYCLOPEDIA_ARMOR_STATS: &[&str] = &[
    "tconstruct:plating_helmet", "tconstruct:plating_chestplate",
    "tconstruct:plating_leggings", "tconstruct:plating_boots",
    "tconstruct:plating_shield", "tconstruct:maille", "tconstruct:cuirass",
    "tconstruct:shield_core",
];

pub(super) fn encyclopedia_armor_order(cuirass: bool, maille: bool, chestplate: bool) -> i32 {
    if cuirass { if maille { 2 } else { 1 } }
    else if chestplate { 4 }
    else if maille { 3 }
    else { 5 }
}

// Default IntRange in the source material transformer, not a progression-book
// tier cap. Negative tiers and values outside Java's short range stay hidden.
pub(super) fn encyclopedia_material_tier(tier: i32) -> bool {
    (0..=i16::MAX as i32).contains(&tier)
}

// Source book tier and material-tag predicates. Stat support and hidden state
// are checked separately against the current material registry.
pub(super) fn material_book_tier(puny: bool, key: &str, tier: i32, distant: bool, basic_ammo: bool) -> bool {
    if puny {
        !distant && match key {
            "ranged_materials" => tier == 2,
            "armor_materials" => (0..=2).contains(&tier),
            _ => false,
        }
    } else {
        (0..=1).contains(&tier) && if key == "ammo_materials" { basic_ammo } else { !distant }
    }
}

// The pinned Mighty index uses tier 3 and excludes distant materials for
// all three chapters. Stat support and hidden status are checked by the caller.
pub(super) fn mighty_material_tier(key: &str, tier: i32, distant: bool) -> bool {
    matches!(key, "melee_harvest_materials" | "ranged_materials" | "armor_materials")
        && tier == 3 && !distant
}

// The melee chapter also admits Whitestone when its special item tag is
// populated, even when that material is tagged as distant.
pub(super) fn puny_melee_material_tier(tier: i32, distant: bool, id: &str, whitestone_ingots: bool) -> bool {
    tier == 2 && (!distant || (id.split('#').next() == Some("tconstruct:whitestone") && whitestone_ingots))
}

pub(super) fn show_material_chapter(puny: bool, count: usize) -> bool {
    !puny || count > 0
}

pub(super) fn label(text: &str) -> String {
    let first = text.lines().next().unwrap_or("");
    if first.chars().count() <= 26 { return first.to_owned(); }
    let prefix: String = first.chars().take(26).collect();
    let prefix = prefix.rsplit_once(' ').map(|(head, _)| head).filter(|s| !s.is_empty()).unwrap_or(&prefix);
    format!("{prefix}…")
}

// One-based book page numbers for chapter and page indexes.
pub(super) fn layout(sizes: &[usize]) -> Option<(usize, Vec<(usize, usize)>)> {
    let indexes = sizes.len().checked_add(4)? / 5;
    let mut next = indexes.checked_add(1)?;
    let mut chapters = Vec::with_capacity(sizes.len());
    for &pages in sizes {
        let chapter_indexes = pages.checked_add(4)? / 5;
        let first_page = next.checked_add(chapter_indexes)?;
        chapters.push((next, first_page));
        next = first_page.checked_add(pages)?;
    }
    Some((indexes, chapters))
}

#[cfg(test)]
mod tests {
    use super::{gadgetry_exploration_page, label, layout, mighty_advice_page, mighty_smeltery_page, mighty_advice_ready, mighty_material_tier, material_book_tier, puny_armor_order, puny_melee_material_tier, show_material_chapter};
    #[test]
    fn gadgetry_exploration_excludes_unimplemented_sources() {
        for name in ["cobalt_ore", "cobalt_blasting"] { assert!(gadgetry_exploration_page(name)); }
        for name in ["index", "steel_shard", "cobalt_shard", "necrotic_bones", "dragon_scales", "grass_seeds", "skyslime_island", "ichor_geode", "unknown"] { assert!(!gadgetry_exploration_page(name)); }
    }
    #[test]
    fn mighty_smeltery_only_exposes_selected_source_pages() {
        for name in ["features", "controller", "structure", "structurereq", "alloying", "blazing_blood_barter", "sgui1", "sgui2", "seared_tanks", "seared_blocks", "seared_chute", "seared_ladder", "seared_glass", "tinted_glass", "soul_glass", "seared_drain", "seared_duct"] { assert!(mighty_smeltery_page(name)); }
        for name in ["unknown"] {
            assert!(!mighty_smeltery_page(name));
        }
    }
    #[test]
    fn encyclopedia_armor_keeps_source_stats_and_order_separate_from_slimesuit() {
        assert_eq!(super::ENCYCLOPEDIA_ARMOR_STATS.len(), 8);
        for stat in ["slime", "skull", "ribcage", "shell", "laces"] {
            assert!(!super::ENCYCLOPEDIA_ARMOR_STATS.contains(&format!("tconstruct:{stat}").as_str()));
        }
        for (cuirass, maille, plating, expected) in [
            (true, false, false, 1), (true, false, true, 1),
            (true, true, false, 2), (true, true, true, 2),
            (false, true, false, 3), (false, false, true, 4),
            (false, true, true, 4), (false, false, false, 5),
        ] {
            assert_eq!(super::encyclopedia_armor_order(cuirass, maille, plating), expected);
        }
    }
    #[test]
    fn encyclopedia_uses_the_full_source_default_tier_range() {
        for tier in [0, 1, 2, 3, 4, 5, 100, i16::MAX as i32] {
            assert!(super::encyclopedia_material_tier(tier));
        }
        for tier in [i32::MIN, -1, i16::MAX as i32 + 1, i32::MAX] {
            assert!(!super::encyclopedia_material_tier(tier));
        }
    }
    #[test]
    fn encyclopedia_requires_exact_detailed_material_chapters() {
        for (name, kind, key) in [
            ("materials_harvest", "tconstruct:melee_harvest", "melee_harvest_materials"),
            ("materials_ranged", "tconstruct:ranged", "ranged_materials"),
            ("materials_ammo", "tconstruct:ammo", "ammo_materials"),
            ("materials_armor", "tconstruct:armor", "armor_materials"),
        ] {
            assert_eq!(super::encyclopedia_material_chapter(name, "no-load", kind, true), Some(key));
            assert_eq!(super::encyclopedia_material_chapter(name, "sections/materials.json", kind, true), None);
            assert_eq!(super::encyclopedia_material_chapter(name, "no-load", "tconstruct:unknown", true), None);
            assert_eq!(super::encyclopedia_material_chapter(name, "no-load", kind, false), None);
        }
        for name in ["materials_slime", "fluid_effects", "tools", "armor", "unknown"] {
            assert_eq!(super::encyclopedia_material_chapter(name, "no-load", "tconstruct:armor", true), None);
        }
        assert_eq!(super::encyclopedia_material_chapter("materials_ammo", "no-load", "tconstruct:ranged", true), None);
    }
    #[test]
    fn mighty_materials_only_include_nondistant_tier_three() {
        for key in ["melee_harvest_materials", "ranged_materials", "armor_materials"] {
            assert!(mighty_material_tier(key, 3, false));
            assert!(!mighty_material_tier(key, 2, false));
            assert!(!mighty_material_tier(key, 4, false));
            assert!(!mighty_material_tier(key, 3, true));
        }
        assert!(!mighty_material_tier("ammo_materials", 3, false));
        assert!(!mighty_material_tier("unknown", 3, false));
    }
    #[test]
    fn mighty_advice_needs_all_notice_labels() {
        assert!(mighty_advice_ready([Some("Partial"), Some("Notice"), Some("Details")]));
        assert!(!mighty_advice_ready([Some("Partial"), None, Some("Details")]));
        assert!(!mighty_advice_ready([Some("Partial"), Some(" "), Some("Details")]));
    }
    #[test]
    fn puny_slotless_keeps_three_exact_groups_and_separate_damage_page() {
        assert_eq!(super::puny_slotless_group("group_slotless"), Some(("tconstruct:slotless/general", "slotless/slotless.json")));
        assert_eq!(super::puny_slotless_group("group_bonus_upgrades"), Some(("tconstruct:slotless/bonus", "slotless/bonus_upgrades.json")));
        assert_eq!(super::puny_slotless_group("group_cosmetic"), Some(("tconstruct:slotless/cosmetic", "slotless/cosmetic.json")));
        for name in ["damage_tools", "group_unknown", "group_general", "group_bonus"] {
            assert_eq!(super::puny_slotless_group(name), None);
        }
    }
    #[test]
    fn puny_upgrades_keep_five_overviews_and_five_bounded_subgroups() {
        for name in ["general", "melee", "harvest", "ranged", "armor"] {
            let (tag, page) = super::puny_upgrade_group(&format!("group_{name}")).unwrap();
            assert!(tag.starts_with("tconstruct:upgrades/"));
            assert_eq!(page, Some(format!("upgrades/{name}.json").as_str()));
        }
        assert_eq!(super::puny_upgrade_group("group_armor").unwrap().0, "tconstruct:upgrades/armor/general");
        assert_eq!(super::puny_upgrade_group("group_damage"), Some(("tconstruct:upgrades/damage", None)));
        for name in ["helmet", "chestplate", "leggings", "boots"] {
            let (tag, page) = super::puny_upgrade_group(&format!("group_armor.{name}")).unwrap();
            assert_eq!(tag, format!("tconstruct:upgrades/armor/{name}"));
            assert_eq!(page, None);
        }
        for name in ["group_unknown", "group_armor.shield", "group_armor.unknown", "group_interact"] {
            assert_eq!(super::puny_upgrade_group(name), None);
        }
    }
    #[test]
    fn mighty_defense_keeps_two_exact_source_overviews() {
        for name in ["protection", "special"] {
            let (tag, page) = super::mighty_defense_group(&format!("group_{name}")).unwrap();
            assert_eq!(tag, format!("tconstruct:defense/{name}"));
            assert_eq!(page, Some(format!("defense/{name}.json").as_str()));
        }
        for name in ["group_general", "group_armor", "group_unknown", "protection", "special"] {
            assert_eq!(super::mighty_defense_group(name), None);
        }
    }
    #[test]
    fn mighty_abilities_keep_six_overviews_and_five_bounded_subgroups() {
        for name in ["general", "interact", "melee", "harvest", "ranged", "armor"] {
            let (tag, page) = super::mighty_ability_group(&format!("group_{name}")).unwrap();
            assert!(tag.starts_with("tconstruct:abilities/"));
            assert_eq!(page, Some(format!("abilities/{name}.json").as_str()));
        }
        for name in ["helmet", "chestplate", "leggings", "boots", "shield"] {
            let (tag, page) = super::mighty_ability_group(&format!("group_armor.{name}")).unwrap();
            assert_eq!(tag, format!("tconstruct:abilities/armor/{name}"));
            assert_eq!(page, None);
        }
        assert_eq!(super::mighty_ability_group("group_interact").unwrap().0, "tconstruct:abilities/interaction");
        assert_eq!(super::mighty_ability_group("group_unknown"), None);
        assert_eq!(super::mighty_ability_group("group_armor.unknown"), None);
    }
    #[test]
    fn mighty_entity_melting_requires_complete_source_page_identity() {
        let cases = [
            ("entitymelting", "entitymelting/intro.json", "mantle:image_text"),
            ("entitymelting", "entitymelting/entitymelting.json", "mantle:text"),
            ("noteableentities", "entitymelting/notable.json", "mantle:text"),
        ];
        for (name, path, kind) in cases {
            assert!(super::mighty_entity_melting_page(name, path, kind));
            assert!(!super::mighty_entity_melting_page("unknown", path, kind));
            assert!(!super::mighty_entity_melting_page(name, "entitymelting/unknown.json", kind));
            assert!(!super::mighty_entity_melting_page(name, path, "tconstruct:unknown"));
        }
        assert!(!super::mighty_entity_melting_page("entitymelting", "entitymelting/intro.json", "mantle:text"));
    }
    #[test]
    fn mighty_advice_includes_bounded_groups_and_adapted_tank_page() {
        for name in ["group_general", "group_ranged", "group_info", "tanks", "tips", "designing"] { assert!(mighty_advice_page(name)); }
        for name in ["unknown"] { assert!(!mighty_advice_page(name)); }
    }
    #[test]
    fn mighty_tool_pages_require_known_registered_tools_inputs_and_matching_source() {
        for name in ["sledge_hammer", "vein_hammer", "excavator", "broad_axe", "scythe", "cleaver", "longbow"] {
            let id = format!("tconstruct:{name}");
            assert!(super::mighty_tool_guide_ready(&id, true, true, Some(&id)));
            assert!(!super::mighty_tool_guide_ready(&id, false, true, Some(&id)));
            assert!(!super::mighty_tool_guide_ready(&id, true, false, Some(&id)));
            assert!(!super::mighty_tool_guide_ready(&id, true, true, Some("tconstruct:other")));
            assert!(!super::mighty_tool_guide_ready(&id, true, true, None));
        }
        for id in ["tconstruct:javelin", "tconstruct:pickaxe", "other:longbow"] {
            assert!(!super::mighty_tool_guide_ready(id, true, true, Some(id)));
        }
    }
    #[test]
    fn puny_melee_whitestone_exception_is_narrow_and_tier_bound() {
        assert!(puny_melee_material_tier(2, false, "tconstruct:other", false));
        assert!(!puny_melee_material_tier(2, true, "tconstruct:other", true));
        assert!(!puny_melee_material_tier(2, true, "tconstruct:whitestone", false));
        assert!(puny_melee_material_tier(2, true, "tconstruct:whitestone", true));
        assert!(puny_melee_material_tier(2, true, "tconstruct:whitestone#variant", true));
        assert!(!puny_melee_material_tier(1, true, "tconstruct:whitestone", true));
        assert!(!puny_melee_material_tier(3, true, "tconstruct:whitestone", true));
    }
    #[test]
    fn empty_puny_material_chapter_is_omitted_without_changing_basic_book() {
        assert!(!show_material_chapter(true, 0));
        assert!(show_material_chapter(true, 1));
        assert!(show_material_chapter(false, 0));
    }
    #[test]
    fn book_tiers_keep_puny_ranged_separate_from_basic_materials() {
        assert!(material_book_tier(true, "ranged_materials", 2, false, false));
        assert!(!material_book_tier(true, "ranged_materials", 1, false, false));
        assert!(!material_book_tier(true, "ranged_materials", 2, true, false));
        assert!(!material_book_tier(true, "melee_harvest_materials", 2, false, false));
        for tier in 0..=2 { assert!(material_book_tier(true, "armor_materials", tier, false, false)); }
        assert!(!material_book_tier(true, "armor_materials", -1, false, false));
        assert!(!material_book_tier(true, "armor_materials", 3, false, false));
        assert!(!material_book_tier(true, "armor_materials", 2, true, false));
        assert!(material_book_tier(false, "ranged_materials", 1, false, false));
        assert!(!material_book_tier(false, "ranged_materials", 2, false, false));
        assert!(material_book_tier(false, "ammo_materials", 1, true, true));
        assert!(!material_book_tier(false, "ammo_materials", 1, false, false));
    }
    #[test]
    fn puny_armor_uses_source_order_and_leaves_unknowns_last() {
        assert!(puny_armor_order("tconstruct:travelers_helmet") < puny_armor_order("tconstruct:plate_helmet"));
        assert!(puny_armor_order("tconstruct:plate_boots") < puny_armor_order("tconstruct:plate_shield"));
        assert_eq!(puny_armor_order("example:custom"), usize::MAX);
    }
    #[test]
    fn index_labels_keep_names_and_bound_long_descriptions() {
        assert_eq!(label("Traveler's Goggles"), "Traveler's Goggles");
        let long = label("Basic armor allows basic protection without needing to hunt for resources");
        assert!(long.ends_with('…'));
        assert!(long.chars().count() <= 27);
    }
    #[test]
    fn chapter_and_page_links_account_for_multiple_index_pages() {
        assert_eq!(layout(&[6, 1, 0, 5, 2, 8]), Some((2, vec![(3, 5), (11, 12), (13, 13), (13, 14), (19, 20), (22, 24)])));
        assert_eq!(layout(&[usize::MAX]), None);
    }
}
