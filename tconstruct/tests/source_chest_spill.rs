#[derive(Debug, PartialEq)]
enum PreRemove { Contents { slots: usize, furnace: bool } }

include!("../mixins/tconstruct/station-spill.rs");

#[test]
fn removal_includes_every_chest_slot() {
    use std::ops::ControlFlow::{Break, Continue};
    assert_eq!(tconstruct_spill("tconstruct:part_chest"), Break(PreRemove::Contents { slots: 256, furnace: false }));
    assert_eq!(tconstruct_spill("tconstruct:tinkers_chest"), Break(PreRemove::Contents { slots: 64, furnace: false }));
    assert_eq!(tconstruct_spill("minecraft:chest"), Continue(()));
}
