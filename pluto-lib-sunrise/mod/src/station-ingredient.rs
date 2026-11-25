// Borrow the exact item registry provided to the station-table parser.
use super::{Ingredient, Items};
use pluto_inventory_api::crafting::ingredients::{self, Outcome, RegistryView, Request};

struct Registry<'a>(&'a Items);
impl RegistryView for Registry<'_> {
    fn contains_item(&self, id: &str) -> bool {
        self.0.items.contains_key(id)
    }
    fn ordered_tag(&self, id: &str) -> Option<&[String]> {
        self.0.item_tag_order.get(id).map(Vec::as_slice)
    }
}
pub fn parse(value: &serde_json::Value, items: &Items) -> std::ops::ControlFlow<Ingredient> {
    use std::ops::ControlFlow::{Break, Continue};
    if !ingredients::has_parsers() {
        return Continue(());
    }
    match ingredients::evaluate(&Request {
        value,
        registry: &Registry(items),
    }) {
        Outcome::Native => Continue(()),
        Outcome::Resolved { items, .. } => Break(Ingredient::of(items)),
        Outcome::Conflict { .. } | Outcome::Invalid { .. } => Break(Ingredient::of(Vec::new())),
    }
}
