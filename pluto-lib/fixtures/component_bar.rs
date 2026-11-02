//! Run against the pinned host's `horizon-client-render` and the staged policy.
#[path = "../mixins/pluto_component_bar/policy.rs"]
mod policy;

#[test]
fn host_bar_uses_component_maximum() {
    use horizon_client_render::hud_overlays::damage_bar;
    let (damage, max) = policy::bar_input(Some(60), 30, false, false).unwrap();
    assert_eq!(damage_bar(damage, max).unwrap().0, 7);
    assert!(damage_bar(0, max).is_none());
    assert_eq!(damage_bar(60, max).unwrap().0, 0);
    assert_ne!(damage_bar(damage, max), damage_bar(damage, 1561));
}
