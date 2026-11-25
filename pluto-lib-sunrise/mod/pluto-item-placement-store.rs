// UNINSTALLABLE at the proposed cached::behaviours STORE: its initializer is
// build()?, and the pinned Mixins modified_value_error rejects a selected Try
// expression. This is a source finding, not an executed plan/build diagnostic.
// Keep this helper undeclared; do not install only the cache suppression hooks.
// A different owned-table or symbolic resource-export route is still needed.
fn pluto_item_placement_store(mut value: crate::level::Behaviours) -> crate::level::Behaviours {
    if let Err(error) = crate::level::pluto_item_placement::apply(&mut value) {
        // The overlay refuses all declared items on a failed batch; unrelated
        // items remain native. If registry names are unavailable all placements
        // are refused. No invalid table is persisted by the paired cache guards.
        eprintln!("pluto_lib: item placement batch rejected: {error}");
    }
    value
}
