//! Actual kind and loot-gate helpers. This does not compile native Game/loot
//! types, constructors or the adapted death pipeline under the closed D gate.
use std::ops::ControlFlow;
mod sunrise_client {pub(crate) mod mobs {
    pub(crate) struct Kind {pub id:String}
    pub(crate) struct Mob {pub kind:Kind}
}}
include!("../../../mod/src/pluto-entity-kind.rs");
pub(crate) mod nav {pub mod pluto_entity_spawn_state {
    pub fn head_drop_eligibility()->Option<bool> {crate::entity_spawn_state_tests::head_drop_eligibility()}
}}
mod drops {
    type LootStack=();
    include!("../../../mod/src/pluto-head-loot-gate.rs");
    pub(super) fn gate(table:&str)->std::ops::ControlFlow<Vec<()>> {pluto_head_loot_gate(table)}
}
#[path="../../../mod/src/pluto-living-drop-state.rs"]
mod combat_state;
#[test]
fn actual_kind_helper_preserves_custom_names_and_head_gate_never_changes_other_loot_tables() {
    for (input,expected) in [("zombie","minecraft:zombie"),("minecraft:zombie","minecraft:zombie"),
        ("tconstruct:sky_slime","tconstruct:sky_slime")] {
        let mob=sunrise_client::mobs::Mob {kind:sunrise_client::mobs::Kind {id:input.into()}};
        assert_eq!(pluto_entity_kind(&mob),ControlFlow::Break(expected.to_owned()));
    }
    let table="minecraft:charged_creeper/root";
    assert_eq!(drops::gate(table),ControlFlow::Continue(()));
    crate::entity_spawn_state_tests::with_head_drop_eligibility(false,|| {
        assert_eq!(drops::gate(table),ControlFlow::Break(vec![]));
        assert_eq!(drops::gate("minecraft:entities/blaze"),ControlFlow::Continue(()));
        assert_eq!(drops::gate("example:other_head_table"),ControlFlow::Continue(()));
        crate::entity_spawn_state_tests::with_head_drop_eligibility(true,||assert_eq!(drops::gate(table),ControlFlow::Continue(())));
        assert_eq!(drops::gate(table),ControlFlow::Break(vec![]));
    });
    assert_eq!(drops::gate(table),ControlFlow::Continue(()));
}

