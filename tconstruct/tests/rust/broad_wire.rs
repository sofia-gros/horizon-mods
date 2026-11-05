#[path = "../../mixins/tconstruct/source-broad-wire.rs"] mod broad_wire;
#[path = "../../mixins/tconstruct/source-pure-rules.rs"] mod tconstruct_source_dwarven;
#[path = "../../mixins/tconstruct/source-mining-math.rs"] mod tconstruct_source_mining_math;

#[test]
fn snapshot_preserves_broad_and_narrow_speed_and_unknown_skips_trait() {
    use std::collections::HashSet;
    use tconstruct_source_mining_math::{conditional_speed_for, Conditional};
    let host = HashSet::from(["tconstruct:hammer".to_owned()]);
    let guest = broad_wire::decode(&broad_wire::encode(Some(&host))).unwrap();
    for (tool, expected) in [("tconstruct:hammer", 10.4), ("tconstruct:pickaxe", 12.0)] {
        let modifiers = [Conditional::Overwield(1, 1, guest.contains(tool))];
        let speed = conditional_speed_for(10.0, 0, 100, 1.0, 1.0, &modifiers, None, true, false);
        assert!((speed - expected).abs() < 0.0001);
    }
    let unknown = broad_wire::decode(&broad_wire::encode(None));
    let modifiers: Vec<_> = unknown.as_ref().map(|tags| vec![Conditional::Overwield(1, 1, tags.contains("tconstruct:hammer"))]).unwrap_or_default();
    assert_eq!(conditional_speed_for(10.0, 0, 100, 1.0, 1.0, &modifiers, None, true, false), 10.0);
}
