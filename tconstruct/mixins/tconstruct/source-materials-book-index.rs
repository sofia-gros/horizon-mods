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
    use super::{label, layout, material_book_tier, puny_armor_order, puny_melee_material_tier, show_material_chapter};
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
