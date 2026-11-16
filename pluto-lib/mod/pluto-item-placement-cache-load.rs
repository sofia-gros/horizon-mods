// Draft HEAD helper for level::data::cache::load; not yet in the manifest.
fn pluto_item_placement_cache_load() -> ::core::ops::ControlFlow<Option<Behaviours>> {
    if crate::level::pluto_item_placement_access::uses_custom_bindings() {
        ::core::ops::ControlFlow::Break(None)
    } else {
        ::core::ops::ControlFlow::Continue(())
    }
}
