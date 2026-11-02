use horizon_client_lib::items::ItemStack;
use horizon_world::nbt::Tag;
#[path = "../mixins/pluto_tool_rules/rules.rs"]
mod rules;
use rules::{evaluate, Outcome, Rule};

#[test]
fn ordered_values_and_fallbacks_are_independent() {
    let rules = [
        Rule {
            blocks: vec!["#example:mineable".into()],
            speed: None,
            correct_for_drops: Some(false),
        },
        Rule {
            blocks: vec!["minecraft:stone".into()],
            speed: Some(2.0),
            correct_for_drops: Some(true),
        },
        Rule {
            blocks: vec!["minecraft:stone".into()],
            speed: Some(9.0),
            correct_for_drops: None,
        },
    ];
    let result = evaluate(&rules, 1.0, |id| {
        id == "#example:mineable" || id == "minecraft:stone"
    });
    assert_eq!(
        result,
        Outcome {
            speed: 2.0,
            correct_for_drops: false
        }
    );
    assert_eq!(
        evaluate(&rules, 1.5, |_| false),
        Outcome {
            speed: 1.5,
            correct_for_drops: false
        }
    );
}

#[test]
fn actual_component_and_missing_component() {
    let mut stack = ItemStack::new("example:tool", 1);
    assert!(rules::evaluate_stack(&stack, |_| true).is_none());
    stack.data_mut().set_other(
        "minecraft:tool",
        Some(Tag::compound([
            (
                "rules",
                Tag::List(vec![
                    Tag::compound([
                        ("blocks", Tag::String("#example:mineable".into())),
                        ("correct_for_drops", Tag::Byte(0)),
                    ]),
                    Tag::compound([
                        (
                            "blocks",
                            Tag::List(vec![Tag::String("minecraft:stone".into())]),
                        ),
                        ("speed", Tag::Float(2f32.to_bits())),
                        ("correct_for_drops", Tag::Byte(1)),
                    ]),
                ]),
            ),
            ("default_mining_speed", Tag::Int(1)),
        ])),
    );
    assert_eq!(
        rules::evaluate_stack(&stack, |id| id == "#example:mineable"
            || id == "minecraft:stone"),
        Some(Outcome {
            speed: 2.0,
            correct_for_drops: false
        })
    );
    stack.data_mut().set_other(
        "minecraft:tool",
        Some(Tag::compound([("rules", Tag::Int(1))])),
    );
    assert!(rules::evaluate_stack(&stack, |_| true).is_none());
}
