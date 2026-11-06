use super::*;

#[test]
fn all_builtin_effects_preserve_raw_canonical_color_and_instant() {
    for def in potions::EFFECTS {
        let facts = effect(def.id).unwrap();
        assert_eq!(facts.raw_id, def.id);
        assert_eq!(facts.canonical_id, potions::namespaced(def.id));
        assert_eq!(facts.color_rgb, def.color);
        assert_eq!(facts.instant, def.instant);
        assert_eq!(effect(facts.canonical_id), Some(facts));
        assert!(!has_custom_namespace(facts.raw_id));
        assert!(!has_custom_namespace(facts.canonical_id));
    }
}

#[test]
fn every_potion_preserves_name_empty_state_effect_order_and_values() {
    for def in potions::POTIONS {
        let facts = potion(def.id).unwrap();
        assert_eq!(facts.raw_id, def.id);
        assert_eq!(facts.name_suffix, def.name);
        assert_eq!(facts.effects, def.effects);
        assert_eq!(potion(&format!("minecraft:{}", def.id)), Some(facts));
    }
    assert!(potion("water").unwrap().effects.is_empty());
    assert!(potion("turtle_master").unwrap().effects.len() > 1);
}

#[test]
fn lookup_keeps_missing_and_official_alias_behavior() {
    for id in ["", "missing", "minecraft:missing", "not_registered:missing", "other:speed"] {
        assert_eq!(effect(id), None);
        assert_eq!(potion(id), None);
    }
    // Preserve even the official repeated-prefix normalization; do not
    // silently add a stricter validation policy during extraction.
    assert_eq!(effect("minecraft:minecraft:speed"), effect("speed"));
    assert_eq!(potion("minecraft:minecraft:water"), potion("water"));
}

#[test]
fn custom_namespace_is_syntax_only_not_membership_or_validation() {
    for id in ["", "speed", "minecraft:speed", "minecraft:missing", "other:missing", ":", "minecraft:minecraft:speed"] {
        assert_eq!(has_custom_namespace(id), potions::is_mod_effect(id));
    }
    assert!(has_custom_namespace("not_registered:missing"));
    assert!(effect("not_registered:missing").is_none());
}

#[test]
fn registered_custom_effect_retains_its_identity_without_aliasing() {
    // Sole registry writer in this isolated fixture process. This is not a
    // Pluto registration API, and no registry mutation occurs in consumers.
    assert!(potions::register_mod_effects([("pluto_fixture:glow".into(), 0x123456, potions::Category::Neutral)]));
    let facts = effect("pluto_fixture:glow").unwrap();
    assert_eq!(facts.raw_id, "pluto_fixture:glow");
    assert_eq!(facts.canonical_id, facts.raw_id);
    assert_eq!(facts.color_rgb, 0x123456);
    assert!(!facts.instant);
    assert!(has_custom_namespace(facts.raw_id));
    assert!(effect("glow").is_none());
    assert!(effect("minecraft:pluto_fixture:glow").is_none());
    assert!(potion("pluto_fixture:glow").is_none());
}
