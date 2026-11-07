// Adapted from Horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0, game/advancements.rs.
// Changes: shared owner snapshots and a separate per-world custom inventory-index cache.
// MIT License
//
// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

fn pluto_advancement_tag_index(
    game: &mut Game<'_>,
) -> std::ops::ControlFlow<Arc<adv::InventoryIndex>> {
    use horizon_client::pluto_inventory::advancements::tags::{self, Lookup, OwnedSnapshots};
    use std::ops::ControlFlow::{Break, Continue};
    type Cached = (
        Arc<OwnedSnapshots>,
        Arc<adv::Tree>,
        Arc<adv::InventoryIndex>,
    );
    let state = game
        .world
        .chunks
        .__horizon_mixins_pluto_native_creative_advancement_tags();
    // The native cache remains entirely untouched, including after clear/release
    // and after returning from another parked world with custom tag providers.
    if state.owners.is_empty() {
        return Continue(());
    }
    let snapshot = state.owners.clone();
    let tree = game.adv.tree.clone();
    if let Some((built_tags, built_tree, index)) = state
        .cache
        .as_ref()
        .and_then(|cache| cache.downcast_ref::<Cached>())
    {
        if Arc::ptr_eq(built_tags, &snapshot) && Arc::ptr_eq(built_tree, &tree) {
            return Break(index.clone());
        }
    }
    let items = game.items();
    let index = Arc::new(adv::InventoryIndex::build(
        &tree,
        |tag| match tags::lookup(&snapshot, tag) {
            Lookup::Unmanaged => items
                .item_tags
                .get(tag)
                .map(|members| members.iter().map(String::as_str).collect())
                .unwrap_or_default(),
            Lookup::Unavailable | Lookup::Conflict => Vec::new(),
            Lookup::Members(members) => members.iter().map(String::as_str).collect(),
        },
    ));
    let cached: Cached = (snapshot, tree, index.clone());
    game.world
        .chunks
        .__horizon_mixins_pluto_native_creative_advancement_tags_mut()
        .cache = Some(Arc::new(cached));
    Break(index)
}
