//! Preserve TConstruct-specific ordering while the standard rule API changes.
use horizon_world::nbt::Tag;
use crate::tconstruct_source_mining_rules::{conditional, effective_speed, harvestable};

fn tool() -> Tag {
    Tag::compound([("rules", Tag::List(vec![
        Tag::compound([("blocks", Tag::String("#minecraft:incorrect_for_wooden_tool".into())), ("correct_for_drops", Tag::Byte(0))]),
        Tag::compound([("blocks", Tag::String("#tconstruct:source_effective/minecraft/mineable/pickaxe".into())), ("correct_for_drops", Tag::Byte(1)), ("speed", Tag::Float(2.0f32.to_bits()))]),
    ]))])
}
fn includes(tag: &str, block: &str) -> bool {
    (tag == "minecraft:incorrect_for_wooden_tool" && block == "example:hard_ore")
        || (tag == "tconstruct:source_effective/minecraft/mineable/pickaxe" && matches!(block, "example:hard_ore" | "example:stone"))
}
fn context(damage: u32) -> (Tag, u32) {
    (Tag::compound([
        ("tic_max_damage", Tag::Int(100)),
        ("tic_mining_all", Tag::Float(1.0f32.to_bits())),
        ("tic_durability_all", Tag::Float(1.0f32.to_bits())),
        ("tic_mining_traits", Tag::List(vec![
            Tag::compound([("id", Tag::String("tconstruct:jagged".into())), ("level", Tag::Int(1))]),
            Tag::compound([("id", Tag::String("tconstruct:maintained".into())), ("level", Tag::Int(1))]),
        ])),
    ]), damage)
}

#[test]
fn valid_effective_block_and_tconstruct_conditional_order() {
    let rules = tool();
    assert_eq!(effective_speed(&rules, "example:stone", includes), Some(2.0));
    assert_eq!(harvestable(&rules, "example:stone", includes), Some(true));
    let (custom, damage) = context(25);
    // Given 4.0 after the host's earlier calculations: Maintained adds 3.0,
    // then Jagged multiplies by 0.95. This is not a real-world timing claim.
    assert!((conditional(&custom, damage, 4.0, 1.0).unwrap() - 6.65).abs() < 0.0001);
}

#[test]
fn incorrect_tier_precedes_later_effective_rule() {
    let rules = tool();
    assert_eq!(harvestable(&rules, "example:hard_ore", includes), Some(false));
    // TConstruct treats the earlier incorrect-tier rule as ineffective even
    // when a later effective rule has a speed; keep this policy outside the
    // generic component rule evaluator.
    assert_eq!(effective_speed(&rules, "example:hard_ore", includes), None);
    assert_eq!(effective_speed(&rules, "example:unmatched", includes), None);
    let (broken, damage) = context(100);
    assert!(conditional(&broken, damage, 4.0, 1.0).is_none());
}
