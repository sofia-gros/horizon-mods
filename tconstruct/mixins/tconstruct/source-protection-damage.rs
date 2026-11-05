/// Source protection coefficients differ from virtual vanilla enchantments.
/// Intact tools add the full fractional source value. Fire/Blast/Scorch retain
/// their integer vanilla enchantment on broken worn pieces; other modules do not.
pub(super) fn points(pieces: &[(f32, bool, bool)], source_each: f32, broken_vanilla_each: f32) -> Option<(f32, f32)> {
    if !source_each.is_finite() || source_each < 0.0 || !broken_vanilla_each.is_finite() || broken_vanilla_each < 0.0 { return None; }
    let mut source = 0.0f32;
    let mut broken_vanilla = 0.0f32;
    for (level, broken, worn) in pieces {
        if !level.is_finite() || *level < 0.0 { return None; }
        if *broken {
            if *worn { broken_vanilla += broken_vanilla_each * level.floor(); }
        } else { source += source_each * level; }
    }
    (source.is_finite() && broken_vanilla.is_finite()).then_some((source, broken_vanilla))
}

/// Supported modifiers contribute to one EPF stage. Overlapping damage tags add
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
        assert_eq!(points(&[(1.0,false,true)],3.0,2.0), Some((3.0,0.0)));
        assert_eq!(points(&[(0.4,false,true),(2.0,false,false)],3.0,2.0), Some((3.0 * 0.4 + 6.0,0.0)));
        assert_eq!(points(&[(f32::NAN,false,true)],3.0,2.0), None);
    }
    #[test]
    fn broken_armor_keeps_only_integer_virtual_protection_enchantment() {
        assert_eq!(points(&[(1.4,true,true),(2.0,true,false)],3.0,2.0), Some((0.0,2.0)));
        assert_eq!(points(&[(0.4,true,true)],3.0,2.0), Some((0.0,0.0)));
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
        add_points(&mut table,&fire,&fire,points(&[(1.0,false,true)],3.0,2.0).unwrap());
        add_points(&mut table,&blast,&blast,points(&[(2.0,false,false)],3.0,2.0).unwrap());
        assert_eq!(table["fire"],3.0);
        assert_eq!(table["explosion"],6.0);
        assert_eq!(table["mixed"],9.0);
    }
    #[test]
    fn broken_blast_uses_vanilla_explosion_tag_without_the_source_override() {
        let source = HashSet::from(["custom_source".into()]);
        let vanilla = HashSet::from(["vanilla_explosion".into()]);
        let mut table = BTreeMap::new();
        add_points(&mut table,&source,&vanilla,points(&[(1.4,true,true)],3.0,2.0).unwrap());
        assert!(!table.contains_key("custom_source"));
        assert_eq!(table["vanilla_explosion"],2.0);
    }
}

#[cfg(test)]
mod magic_tests {
    use super::*;
    #[test]
    fn magic_uses_fractional_three_points_with_no_broken_enchantment() {
        assert_eq!(points(&[(0.4,false,true),(2.0,false,false)],3.0,0.0),Some((3.0 * 0.4 + 6.0,0.0)));
        assert_eq!(points(&[(2.4,true,true),(2.4,true,false)],3.0,0.0),Some((0.0,0.0)));
        assert_eq!(points(&[(1.0,false,true)],f32::NAN,0.0),None);
    }
    #[test]
    fn magic_and_fire_overlap_share_additive_protection() {
        use std::collections::{BTreeMap,HashSet};
        let types = HashSet::from(["mixed".into()]);
        let mut table = BTreeMap::new();
        add_points(&mut table,&types,&types,points(&[(1.0,false,true)],3.0,2.0).unwrap());
        add_points(&mut table,&types,&HashSet::new(),points(&[(2.0,false,true)],3.0,0.0).unwrap());
        assert_eq!(table["mixed"],9.0);
    }
}

#[cfg(test)]
mod projectile_tests {
    use super::*;
    #[test]
    fn projectile_uses_two_points_and_has_no_broken_enchantment() {
        assert_eq!(points(&[(1.0,false,true)],2.0,0.0),Some((2.0,0.0)));
        assert_eq!(points(&[(0.4,false,true),(2.0,false,false)],2.0,0.0),Some((2.0 * 0.4 + 4.0,0.0)));
        assert_eq!(points(&[(3.0,true,true),(3.0,true,false)],2.0,0.0),Some((0.0,0.0)));
    }
    #[test]
    fn overlapping_magic_and_projectile_tags_add_unclamped_coefficients() {
        use std::collections::{BTreeMap,HashSet};
        let types = HashSet::from(["mixed".into()]);
        let mut table = BTreeMap::new();
        add_points(&mut table,&types,&HashSet::new(),points(&[(6.0,false,true)],3.0,0.0).unwrap());
        add_points(&mut table,&types,&HashSet::new(),points(&[(2.0,false,true)],2.0,0.0).unwrap());
        assert_eq!(table["mixed"],22.0); // native stage applies its single cap
    }
}

/// Only these native creature/PvP entry points have a verified direct attacker.
/// A damage-type tag alone cannot establish DamageSource.isDirect().
pub(super) fn verified_melee_source(kind: &str, projectile: bool) -> bool {
    !projectile && matches!(kind,
        "minecraft:player_attack" | "minecraft:mob_attack" |
        "minecraft:mob_attack_no_aggro" | "minecraft:sting")
}

#[cfg(test)]
mod melee_tests {
    use super::*;
    #[test]
    fn melee_uses_fractional_two_points_and_no_broken_enchantment() {
        assert_eq!(points(&[(0.4,false,true),(2.0,false,false)],2.0,0.0),Some((0.8 + 4.0,0.0)));
        assert_eq!(points(&[(3.0,true,true),(3.0,true,false)],2.0,0.0),Some((0.0,0.0)));
    }
    #[test]
    fn tags_never_invent_a_direct_entity_for_an_unknown_source() {
        for kind in ["minecraft:player_attack","minecraft:mob_attack","minecraft:mob_attack_no_aggro","minecraft:sting"] {
            assert!(verified_melee_source(kind,false));
            assert!(!verified_melee_source(kind,true));
        }
        for kind in ["minecraft:arrow","minecraft:fireworks","minecraft:cramming","custom:melee"] {
            assert!(!verified_melee_source(kind,false));
        }
    }
}


#[cfg(test)]
mod scorch_enchantment_tests {
    use super::*;
    #[test]
    fn scorch_virtual_fire_epf_survives_only_on_broken_worn_armor() {
        assert_eq!(points(&[(2.0,false,true)],0.0,2.0),Some((0.0,0.0)));
        assert_eq!(points(&[(2.0,true,true)],0.0,2.0),Some((0.0,4.0)));
        assert_eq!(points(&[(2.0,true,false)],0.0,2.0),Some((0.0,0.0)));
    }
}
