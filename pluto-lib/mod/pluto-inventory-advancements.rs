// Adapted from Horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0, game/advancements.rs.
// Changes: component-aware inventory key and scoped predicate metadata.
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

fn pluto_inventory_advancements(game: &mut Game<'_>) -> std::ops::ControlFlow<()> {
    use horizon_client::pluto_inventory::advancements;
    // The library is inert without an early, explicit consumer request.
    // Continue before snapshots, component encoding, hashes or state writes.
    if !advancements::inventory_component_events_requested() {
        return std::ops::ControlFlow::Continue(());
    }
    // Encode once from this actor's authoritative inventory. Keep slot order
    // and empty positions for the event key; metadata travels with each view.
    let slots: Vec<Option<(ItemView, Option<horizon_world::nbt::Tag>)>> = game
        .inventory
        .iter()
        .chain(game.armor.iter())
        .chain(std::iter::once(&game.offhand))
        .map(|slot| {
            slot.as_ref().map(|stack| {
                (
                    item_view(stack),
                    horizon_client::item_data::encode_components(stack),
                )
            })
        })
        .collect();
    let key = advancements::inventory_key(slots.iter().map(|slot| {
        slot.as_ref()
            .map(|(view, components)| (view.id.as_str(), view.count, components))
    }));
    if key == game.adv.last_inventory {
        return std::ops::ControlFlow::Break(());
    }
    game.adv.last_inventory = key;
    let stacks: Vec<_> = slots.into_iter().flatten().collect();
    // InventoryChangeTrigger slots: occupied, full and empty counts.
    let all: Vec<&Option<ItemStack>> = game
        .inventory
        .iter()
        .chain(game.armor.iter())
        .chain(std::iter::once(&game.offhand))
        .collect();
    let occupied = all.iter().filter(|s| s.is_some()).count() as f64;
    let empty = all.len() as f64 - occupied;
    let full = all
        .iter()
        .filter_map(|s| s.as_ref())
        .filter(|s| {
            game.res
                .items()
                .get(&s.item)
                .is_some_and(|i| s.count >= i.max_stack)
        })
        .count() as f64;
    // Only the criteria the items held could pass (`InventoryIndex`).
    if game.remote.is_some() || game.no_player_here() {
        return std::ops::ControlFlow::Break(());
    }
    game.load_advancements();
    let tree = game.adv.tree.clone();
    let index = game.inventory_index();
    let list = index.candidates(stacks.iter().map(|(view, _)| view.id.as_str()));
    if list.is_empty() {
        return std::ops::ControlFlow::Break(());
    }
    let me = game.player_entity_for("minecraft:inventory_changed");
    game.trigger_criteria(&tree, &list, move |c, w| {
        let slots = &c["slots"];
        let range = |k: &str, v: f64| {
            let r = &slots[k];
            r.is_null()
                || r.as_f64().is_some_and(|n| (n - v).abs() < 0.5)
                || r.is_object()
                    && r["min"].as_f64().is_none_or(|m| v >= m)
                    && r["max"].as_f64().is_none_or(|m| v <= m)
        };
        if !(range("occupied", occupied) && range("full", full) && range("empty", empty)) {
            return false;
        }
        let mut wanted: Vec<&Value> = c["items"]
            .as_array()
            .map(|a| a.iter().collect())
            .unwrap_or_default();
        for (view, components) in &stacks {
            advancements::with_data(view, components.clone(), || {
                wanted.retain(|predicate| !adv::item(predicate, view, w));
            });
        }
        wanted.is_empty()
            && adv::conditions(
                &c["player"],
                &adv::Entities {
                    this: Some(&me),
                    origin: me.position,
                    tool: None,
                    block_pos: None,
                },
                w,
            )
    });
    std::ops::ControlFlow::Break(())
}
