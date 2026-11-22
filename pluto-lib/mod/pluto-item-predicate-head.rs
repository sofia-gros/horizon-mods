// The only host-shaped item predicate boundary. Consumers see PlutoLib facts.
fn pluto_item_predicate(
    v: &serde_json::Value,
    it: &ItemView,
    w: &dyn World,
) -> std::ops::ControlFlow<bool> {
    use pluto_inventory_api::advancements::{self, predicates as api};
    use std::ops::ControlFlow::{Break, Continue};
    // Installing the library without consumers preserves the native matcher,
    // including avoiding metadata cloning/conversion work on this path.
    if !api::has_item_predicates() {
        return Continue(());
    }
    struct Tags<'a>(&'a dyn World);
    impl api::TagLookup for Tags<'_> {
        fn in_tag(&self, registry: &str, tag: &str, id: &str) -> bool {
            self.0.in_tag(registry, tag, id)
        }
    }
    let components = advancements::data::<_, Option<horizon_world::nbt::Tag>>(it)
        .flatten()
        .map(crate::pluto_component_view::from_nbt);
    let tags = Tags(w);
    let request = api::PredicateRequest {
        predicate: v,
        item: api::ItemSnapshot {
            id: &it.id,
            count: it.count,
            damage: it.damage,
            enchantments: &it.enchantments,
            potion: it.potion.as_deref(),
            custom_name: it.custom_name,
            components: components.as_ref(),
        },
        tags: &tags,
    };
    match api::dispatch_item_predicate(&request) {
        api::DispatchResult::Unsupported => Continue(()),
        api::DispatchResult::Handled(matched) => Break(matched),
        api::DispatchResult::Conflict { .. } => Break(false),
    }
}
