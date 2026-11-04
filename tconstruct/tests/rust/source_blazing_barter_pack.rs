#[path = "../../mixins/tconstruct/source-blazing-barter-pack.rs"]
mod source_blazing_barter_pack;

#[test]
fn pinned_injector_and_overrides() {
    use source_blazing_barter_pack::{canonical, enabled, observe};
    let source = include_bytes!("../fixtures/blazing_barter_source.json");
    assert!(canonical(source));
    let mut highest = None;
    observe(&mut highest, true, Some(source.to_vec()));
    assert!(enabled(highest.clone()));
    observe(&mut highest, true, Some(b"{}".to_vec()));
    assert!(!enabled(highest.clone()));
    observe(&mut highest, true, Some(source.to_vec()));
    assert!(enabled(highest));
}

#[test]
fn bundled_injector_matches_the_pinned_source_fixture() {
    let bundled =
        include_bytes!("../../content/data/tconstruct/mantle/loot_injectors/piglin_bartering.json");
    let source = include_bytes!("../fixtures/blazing_barter_source.json");
    assert_eq!(bundled.as_slice(), source.as_slice());
    assert!(source_blazing_barter_pack::enabled(Some(Some(
        bundled.to_vec()
    ))));
}
