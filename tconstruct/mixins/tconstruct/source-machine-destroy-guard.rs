// Refuse player destruction when the machine's saved metadata cannot be
// represented by its current decoder. Other destruction paths are unchanged.
use horizon_world::nbt::Tag;

fn unchanged_fields(raw: &Tag, normalized: &Tag, allow_heat: bool) -> bool {
    let (Tag::Compound(raw), Tag::Compound(normalized)) = (raw, normalized) else { return false; };
    raw.iter().all(|(key, value)| {
        let name = key.to_string().ok();
        let display = allow_heat && match name.as_deref() {
            Some("heat_codes" | "heat_page") => matches!(value, Tag::IntArray(_)),
            Some("heat_start" | "slot_count") => matches!(value, Tag::Int(_)),
            _ => false,
        };
        display || normalized.get(key) == Some(value)
    })
}

pub(super) fn protect(kind: &str, outer: &Tag) -> bool {
    let Tag::Compound(fields) = outer else { return true; };
    // The host owns the outer position and inventory projection; unknown
    // siblings would otherwise disappear when machine metadata is cleared.
    if fields.keys().any(|key| !matches!(key.to_string().ok().as_deref(),
        Some("id" | "x" | "y" | "z" | "Items" | "keepPacked" | "components" | "tconstruct:retired_projection") ) && key.to_string().ok().as_deref() != Some(kind)) { return true; }
    if outer.get("keepPacked").is_some_and(|value| *value != Tag::Byte(0)) { return true; }
    if outer.get("components").is_some_and(|value| !matches!(value, Tag::Compound(fields) if fields.is_empty())) { return true; }
    if let Some(marker) = outer.get("tconstruct:retired_projection") {
        if kind != "tconstruct:smeltery" || (*marker != Tag::Int(0) && *marker != Tag::Int(1)) { return true; }
    }
    // The host chest projection is stored separately from machine slots.
    // Reject malformed, duplicate or non-round-tripping serialized items.
    if let Some(items) = outer.get("Items") {
        let Tag::List(items) = items else { return true; };
        let mut seen = std::collections::HashSet::new();
        for item in items {
            let Tag::Compound(mut fields) = item.clone() else { return true; };
            let Some(Tag::Byte(slot)) = fields.remove(&"Slot".into()) else { return true; };
            let limit = if kind == "tconstruct:smeltery" { 54 } else { 27 };
            if !(0..limit).contains(&slot) || !seen.insert(slot) { return true; }
            let raw = Tag::Compound(fields);
            let Some(stack) = horizon_items::item_nbt::item_from_nbt(&raw) else { return true; };
            if horizon_items::item_nbt::item_to_nbt(&stack) != raw { return true; }
        }
    }
    if !matches!(outer.get("id"), Some(Tag::String(id)) if id.to_string().ok().as_deref() == Some(kind)) { return true; }
    if ["x", "y", "z"].iter().any(|key| outer.get(key).is_some_and(|value| !matches!(value, Tag::Int(_)))) { return true; }
    let Some(raw) = outer.get(kind) else { return true; };
    match kind {
        "tconstruct:melter" => super::tconstruct_source_melter_state::Melter::from_nbt(raw)
            .is_none_or(|state| !unchanged_fields(raw, &state.to_nbt(), true)),
        "tconstruct:smeltery" => super::tconstruct_source_smeltery_state::Smeltery::from_nbt(raw)
            .is_none_or(|state| !unchanged_fields(raw, &state.to_nbt(), true)),
        _ => false,
    }
}

type Stack = Option<horizon_items::items::ItemStack>;
fn equal(a: &Stack, b: &Stack) -> bool {
    match (a,b) {
        (None,None) => true,
        (Some(a),Some(b)) => a.data().is_none_or(|data| data.wire.patch().is_none())
            && b.data().is_none_or(|data| data.wire.patch().is_none())
            && a.count == b.count && a.damage == b.damage && a.same_item_same_components(b),
        _ => false,
    }
}
fn page_equal(a: &[Stack], b: &[Stack]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a,b)| equal(a,b))
}
fn saved_slots(raw: &Tag) -> Option<Vec<Stack>> {
    raw.get("tconstruct:smeltery").and_then(super::tconstruct_source_smeltery_state::Smeltery::from_nbt).map(|state| state.slots)
}

/// A marker can authorize only an EMPTY view, never deletion of a nonempty
/// cache whose contents no longer occur in current authoritative state.
pub(super) fn retired_projection(raw: &Tag) -> bool {
    raw.get("tconstruct:retired_projection") == Some(&Tag::Int(1))
        && matches!(raw.get("Items"), Some(Tag::List(items)) if items.is_empty())
}

pub(super) fn current_projection(raw: &Tag, live: &[Stack], offset: Option<usize>, containers: &[Stack; 2]) -> bool {
    if protect("tconstruct:smeltery", raw) { return false; }
    let Some(saved) = saved_slots(raw) else { return false; };
    if let Some(offset) = offset {
        return page_equal(live, &super::tconstruct_source_smeltery_page::project_with_containers(&saved, offset, containers));
    }
    if containers.iter().any(Option::is_some) { return false; }
    if live.len() == 54 && live.iter().all(Option::is_none) && retired_projection(raw) { return true; }
    if live.iter().all(Option::is_none) && saved.iter().all(Option::is_none) { return true; }
    // No guessed page: the whole loaded view must match both the serialized
    // Items and a legal current authoritative page. An old stale view fails.
    if live.len() != 54 || live[..2].iter().any(Option::is_some)
        || raw.get("Items") != Some(&horizon_items::item_nbt::encode_items(live)) { return false; }
    (0..=super::tconstruct_source_smeltery_layout::max_offset(saved.len()))
        .any(|offset| page_equal(live, &super::tconstruct_source_smeltery_page::project(&saved, offset)))
}

pub(super) fn live_matches(kind: &str, raw: &Tag, live: &[Stack], page_offset: Option<usize>) -> bool {
    if kind == "tconstruct:smeltery" { return current_projection(raw, live, page_offset, &[None, None]); }
    let Some(state) = raw.get(kind).and_then(super::tconstruct_source_melter_state::Melter::from_nbt) else { return false; };
    live.len() >= state.slots.len() && live.iter().take(state.slots.len()).zip(&state.slots).all(|(a,b)| equal(a,b))
        && live.iter().skip(state.slots.len()).all(Option::is_none)
}

pub(super) fn close_projection(raw: &Tag, live: &mut [Stack], offset: Option<usize>, containers: &[Stack; 2]) -> bool {
    if offset.is_none() || !current_projection(raw, live, offset, containers) { return false; }
    live.fill(None);
    true
}

pub(super) fn retire_outer(raw: &Tag) -> Option<Tag> {
    if protect("tconstruct:smeltery", raw) { return None; }
    let Tag::Compound(mut fields) = raw.clone() else { return None; };
    fields.insert("tconstruct:retired_projection".into(), Tag::Int(1));
    fields.insert("Items".into(), Tag::List(Vec::new()));
    Some(Tag::Compound(fields))
}

pub(super) fn open_outer(raw: &Tag) -> Option<Tag> {
    if protect("tconstruct:smeltery", raw) { return None; }
    let Tag::Compound(mut fields) = raw.clone() else { return None; };
    fields.insert("tconstruct:retired_projection".into(), Tag::Int(0));
    Some(Tag::Compound(fields))
}

/// Synchronize serialized Items only after proving the full current page.
/// This marker is not a historical snapshot and grants no nonempty-view trust.
pub(super) fn snapshot_outer(raw: &Tag, live: &[Stack], offset: usize, containers: &[Stack; 2]) -> Option<Tag> {
    if !current_projection(raw, live, Some(offset), containers) { return None; }
    let Tag::Compound(mut fields) = open_outer(raw)? else { return None; };
    fields.insert("Items".into(), horizon_items::item_nbt::encode_items(live));
    Some(Tag::Compound(fields))
}

pub(super) fn legacy_snapshot_outer(raw: &Tag, live: &[Stack]) -> Option<Tag> {
    if raw.get("tconstruct:retired_projection").is_some() || !current_projection(raw, live, None, &[None, None]) { return None; }
    retire_outer(raw)
}

/// Stage a source update and its corresponding live view as one transaction.
/// Callers must leave the old verified live page intact until this commit.
/// A proposed source that happens to match an unexplained new live item is
/// never evidence that the old view was safe to discard.
pub(super) fn transition(before: Option<&Tag>, after: &Tag, live: &[Stack], offset: Option<usize>, containers: &[Stack; 2]) -> Option<(Tag, Vec<Stack>)> {
    if protect("tconstruct:smeltery", after) { return None; }
    let known = match before {
        None => live.iter().all(Option::is_none) && matches!(live.len(), 0 | 27 | 54),
        Some(before) => current_projection(before, live, offset, containers),
    };
    if !known { return None; }
    if let Some(offset) = offset {
        let view = super::tconstruct_source_smeltery_page::project_with_containers(&saved_slots(after)?, offset, containers);
        Some((snapshot_outer(after, &view, offset, containers)?, view))
    } else {
        Some((retire_outer(after)?, vec![None; 54]))
    }
}
