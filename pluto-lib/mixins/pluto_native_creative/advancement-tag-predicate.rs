fn pluto_advancement_tag_predicate(
    view: &View<'_, '_>,
    registry: &str,
    tag: &str,
    id: &str,
) -> std::ops::ControlFlow<bool> {
    use horizon_client::pluto_inventory::advancements::tags::{self, Lookup};
    use std::ops::ControlFlow::{Break, Continue};
    if registry != "item" {
        return Continue(());
    }
    let state = view
        .0
        .world
        .chunks
        .__horizon_mixins_pluto_native_creative_advancement_tags();
    match tags::lookup(&state.owners, tag) {
        Lookup::Unmanaged => Continue(()),
        Lookup::Unavailable | Lookup::Conflict => Break(false),
        Lookup::Members(members) => Break(members.iter().any(|member| member == id)),
    }
}
