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
            correct_for_drops: false,
            speed_rule: Some(1),
            drops_rule: Some(0),
        }
    );
    assert_eq!(
        evaluate(&rules, 1.5, |_| false),
        Outcome {
            speed: 1.5,
            correct_for_drops: false,
            speed_rule: None,
            drops_rule: None,
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
            correct_for_drops: false,
            speed_rule: Some(1),
            drops_rule: Some(0),
        })
    );
    stack.data_mut().set_other(
        "minecraft:tool",
        Some(Tag::compound([("rules", Tag::Int(1))])),
    );
    assert!(rules::evaluate_stack(&stack, |_| true).is_none());
}

#[test]
fn public_parser_distinguishes_absent_invalid_and_no_match() {
    use rules::{parse_tool, ToolParseError};
    assert_eq!(parse_tool(None), Ok(None));
    assert_eq!(
        parse_tool(Some("bad trailing")),
        Err(ToolParseError::InvalidSnbt)
    );
    assert_eq!(
        parse_tool(Some("{rules:[],extra:1}")),
        Err(ToolParseError::InvalidComponent)
    );
    let tool = parse_tool(Some(r##"{rules:[{blocks:"#mineable/pickaxe",speed:2.0f},{blocks:"stone",correct_for_drops:1b}],default_mining_speed:1.0f}"##)).unwrap().unwrap();
    let matched =
        tool.evaluate(|id| id == "#minecraft:mineable/pickaxe" || id == "minecraft:stone");
    assert_eq!(matched.speed_rule, Some(0));
    assert_eq!(matched.drops_rule, Some(1));
    assert_eq!(matched.speed, 2.0);
    assert!(matched.correct_for_drops);
    let none = tool.evaluate(|_| false);
    assert_eq!((none.speed_rule, none.drops_rule), (None, None));
    assert_eq!(none.speed, 1.0);
    assert!(!none.correct_for_drops);
}
