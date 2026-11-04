// Real host FluidState with the production growth selector.
#[path = "../../mixins/tconstruct/source-geode-growth.rs"]
mod growth;
use horizon_world::fluid::{FluidKind, FluidState};

#[test]
fn full_falling_water_can_start_but_does_not_waterlog() {
    let water = FluidState::new(FluidKind::FlowingWater, 8, true);
    assert!(growth::can_start(false, true, water.is_full()));
    assert_ne!(water.kind(), FluidKind::Water);
    assert_eq!(growth::next("tconstruct:budding_sky_slime_crystal", "minecraft:water", false,
        growth::can_start(false, true, water.is_full())).as_deref(), Some("tconstruct:small_sky_slime_crystal_bud"));
}
#[test]
fn source_water_waterlogs_and_shallow_water_does_not_start() {
    let source = FluidState::WATER;
    let shallow = FluidState::new(FluidKind::FlowingWater, 7, false);
    assert!(growth::can_start(false, true, source.is_full()));
    assert_eq!(source.kind(), FluidKind::Water);
    assert!(!growth::can_start(false, true, shallow.is_full()));
    assert!(!growth::can_start(false, false, source.is_full()));
}
