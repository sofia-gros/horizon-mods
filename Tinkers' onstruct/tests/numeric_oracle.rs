use serde::Deserialize;
use tconstruct_horizon::stats::Accumulator;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    name: String,
    default: f32,
    operations: Vec<(String, f64)>,
    expected_bits: u32,
}

#[test]
fn matches_verbatim_upstream_java_float_aggregation_bit_for_bit() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("fixtures/numeric-oracle.json")).unwrap();
    assert!(fixture.cases.len() >= 20);
    for case in fixture.cases {
        let mut stat = Accumulator::new(case.default);
        for (op, value) in case.operations {
            match op.as_str() {
                "update" => stat.update(value as f32),
                "add" => stat.add(value),
                "percent" => stat.percent(value),
                "multiply" => stat.multiply(value),
                "percent_float" => stat.percent(f64::from(value as f32)),
                "add_float" => stat.add(f64::from(value as f32)),
                other => panic!("unknown oracle operation {other}"),
            }
        }
        assert_eq!(stat.build().to_bits(), case.expected_bits, "{}", case.name);
    }
}
