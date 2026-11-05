/// Intact Fire/Blast Protection provides 3 EPF per effective level. Its virtual
/// vanilla Fire/Blast Protection is canceled by EnchantmentModule.Protection, so it
/// must not be added a second time. Broken worn armor retains only the virtual
/// vanilla enchantment: the source equipment iterator skips both modifier hooks.
pub(super) fn points(pieces: &[(f32, bool, bool)]) -> Option<(f32, f32)> {
    let mut source = 0.0f32;
    let mut broken_vanilla = 0.0f32;
    for (level, broken, worn) in pieces {
        if !level.is_finite() || *level < 0.0 { return None; }
        if *broken {
            if *worn { broken_vanilla += 2.0 * level.floor(); }
        } else { source += 3.0 * level; }
    }
    (source.is_finite() && broken_vanilla.is_finite()).then_some((source, broken_vanilla))
}

/// Both modifiers contribute to one EPF stage. Overlapping damage tags add
/// points; they must not apply separate percentage reductions or clamps.
pub(super) fn add_points(
    table: &mut std::collections::BTreeMap<String,f32>,
    source_types: &std::collections::HashSet<String>,
    vanilla_types: &std::collections::HashSet<String>,
    points: (f32,f32),
) {
    for (types, amount) in [(source_types,points.0),(vanilla_types,points.1)] {
        if amount <= 0.0 { continue; }
        for kind in types { *table.entry(kind.clone()).or_default() += amount; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intact_levels_are_fractional_and_not_double_counted_as_enchantments() {
        assert_eq!(points(&[(1.0,false,true)]), Some((3.0,0.0)));
        assert_eq!(points(&[(0.4,false,true),(2.0,false,false)]), Some((3.0 * 0.4 + 6.0,0.0)));
        assert_eq!(points(&[(f32::NAN,false,true)]), None);
    }
    #[test]
    fn broken_armor_keeps_only_integer_virtual_protection_enchantment() {
        assert_eq!(points(&[(1.4,true,true),(2.0,true,false)]), Some((0.0,2.0)));
        assert_eq!(points(&[(0.4,true,true)]), Some((0.0,0.0)));
    }
}

#[cfg(test)]
mod combined_tests {
    use super::*;
    use std::collections::{BTreeMap,HashSet};
    #[test]
    fn overlapping_fire_and_blast_tags_add_points_before_one_native_cap() {
        let fire = HashSet::from(["fire".into(),"mixed".into()]);
        let blast = HashSet::from(["explosion".into(),"mixed".into()]);
        let mut table = BTreeMap::new();
        add_points(&mut table,&fire,&fire,points(&[(1.0,false,true)]).unwrap());
        add_points(&mut table,&blast,&blast,points(&[(2.0,false,false)]).unwrap());
        assert_eq!(table["fire"],3.0);
        assert_eq!(table["explosion"],6.0);
        assert_eq!(table["mixed"],9.0);
    }
    #[test]
    fn broken_blast_uses_vanilla_explosion_tag_without_the_source_override() {
        let source = HashSet::from(["custom_source".into()]);
        let vanilla = HashSet::from(["vanilla_explosion".into()]);
        let mut table = BTreeMap::new();
        add_points(&mut table,&source,&vanilla,points(&[(1.4,true,true)]).unwrap());
        assert!(!table.contains_key("custom_source"));
        assert_eq!(table["vanilla_explosion"],2.0);
    }
}
