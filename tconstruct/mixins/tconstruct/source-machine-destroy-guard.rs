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
        if kind != "tconstruct:smeltery" || (*marker != Tag::Int(0) && *marker != Tag::Int(1) && snapshot_view(outer).is_none()) { return true; }
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

/// Check the in-memory chest projection without assuming a particular
/// Smeltery page offset. A live projection must equal the saved page in the
/// same positions; opaque wire patches are not safe to rewrite on a drop.
pub(super) fn live_matches(kind: &str, raw: &Tag, live: &[Option<horizon_items::items::ItemStack>], page_offset: Option<usize>) -> bool {
    live_matches_with_closed_projection(kind, raw, live, page_offset, retired_projection(raw))
}

/// A closed projection may be empty only after verified retirement. The
/// versioned proof persists with the source and is revoked before local open.
pub(super) fn live_matches_with_closed_projection(kind: &str, raw: &Tag, live: &[Option<horizon_items::items::ItemStack>], page_offset: Option<usize>, closed_projection: bool) -> bool {
    let raw_outer = raw;
    let Some(raw) = raw.get(kind) else { return false; };
    let saved: Vec<_> = match kind {
        "tconstruct:melter" => match super::tconstruct_source_melter_state::Melter::from_nbt(raw) {
            Some(state) => state.slots.to_vec(), None => return false,
        },
        "tconstruct:smeltery" => match super::tconstruct_source_smeltery_state::Smeltery::from_nbt(raw) {
            Some(state) => state.slots, None => return false,
        },
        _ => return false,
    };
    let equal = |a: &Option<horizon_items::items::ItemStack>, b: &Option<horizon_items::items::ItemStack>| match (a,b) {
        (None,None) => true,
        (Some(a),Some(b)) => a.data().is_none_or(|data| data.wire.patch().is_none()) && b.data().is_none_or(|data| data.wire.patch().is_none())
            && a.count == b.count && a.damage == b.damage && a.same_item_same_components(b),
        _ => false,
    };
    if kind == "tconstruct:melter" {
        return live.len() >= saved.len() && live.iter().take(saved.len()).zip(&saved).all(|(a,b)| equal(a,b))
            && live.iter().skip(saved.len()).all(Option::is_none);
    }
    if page_offset.is_none() {
        if let Some(snapshot) = snapshot_view(raw_outer) {
            // Bucket slots remain separately owned, even after reloading with
            // no local or guest container registry. Never discard those items.
            return snapshot[..2].iter().all(Option::is_none) && live.len() == snapshot.len()
                && live.iter().zip(&snapshot).all(|(a,b)| equal(a,b));
        }
    }
    if closed_projection && page_offset.is_none()
        && live.len() == super::tconstruct_source_smeltery_page::MENU_SIZE
        && live.iter().all(Option::is_none) { return true; }
    if live.iter().all(Option::is_none) && saved.iter().all(Option::is_none) { return true; }
    let Some(offset) = page_offset else { return legacy_snapshot_outer(raw_outer, live).is_some(); };
    let expected = super::tconstruct_source_smeltery_page::project(&saved, offset);
    live.len() == expected.len() && live.iter().zip(&expected).all(|(a,b)| equal(a,b))
}

/// Retire only a complete, lossless page that still agrees with saved inputs
/// and the separately owned container slots. Unknown live data is never erased.
/// Call before returning containers, then keep the successful result as proof.
pub(super) fn close_projection(raw: &Tag, live: &mut [Option<horizon_items::items::ItemStack>], page_offset: Option<usize>, containers: &[Option<horizon_items::items::ItemStack>; 2]) -> bool {
    if protect("tconstruct:smeltery", raw) { return false; }
    let Some(offset) = page_offset else { return false; };
    let Some(saved) = raw.get("tconstruct:smeltery")
        .and_then(super::tconstruct_source_smeltery_state::Smeltery::from_nbt) else { return false; };
    let expected = super::tconstruct_source_smeltery_page::project_with_containers(&saved.slots, offset, containers);
    let equal = |a: &Option<horizon_items::items::ItemStack>, b: &Option<horizon_items::items::ItemStack>| match (a,b) {
        (None,None) => true,
        (Some(a),Some(b)) => a.data().is_none_or(|data| data.wire.patch().is_none()) && b.data().is_none_or(|data| data.wire.patch().is_none())
            && a.count == b.count && a.damage == b.damage && a.same_item_same_components(b),
        _ => false,
    };
    let current = live.len() == expected.len() && live.iter().zip(&expected).all(|(a,b)| equal(a,b));
    // A guest may have updated saved inputs since this local page was drawn.
    // The old cache is still disposable if its whole recorded snapshot and
    // separately owned bucket slots agree exactly. Never return a stale bucket
    // twice or mistake an unexplained live insertion for a projected input.
    let recorded = snapshot_view(raw).is_some_and(|snapshot| live.len() == snapshot.len()
        && live.iter().zip(&snapshot).all(|(a,b)| equal(a,b))
        && containers.iter().zip(&snapshot[..2]).all(|(a,b)| equal(a,b)));
    if !current && !recorded { return false; }
    live.fill(None);
    true
}

/// A persisted retired view must have an explicitly empty serialized Items
/// list. An absent, unknown-version or malformed marker confers no trust.
pub(super) fn retired_projection(raw: &Tag) -> bool {
    raw.get("tconstruct:retired_projection") == Some(&Tag::Int(1))
        && matches!(raw.get("Items"), Some(Tag::List(items)) if items.is_empty())
}

/// Caller must first verify the complete live page (or freshly create an
/// empty machine with no preexisting live inventory), then commit the source
/// and retired live view together without callbacks in between.
pub(super) fn retire_outer(raw: &Tag) -> Option<Tag> {
    if protect("tconstruct:smeltery", raw) { return None; }
    let Tag::Compound(mut fields) = raw.clone() else { return None; };
    fields.insert("tconstruct:retired_projection".into(), Tag::Int(1));
    fields.insert("Items".into(), Tag::List(Vec::new()));
    Some(Tag::Compound(fields))
}

pub(super) fn open_outer(raw: &Tag) -> Option<Tag> {
    // Do not silently discard an unknown marker version, even on opening.
    if raw.get("tconstruct:retired_projection").is_some_and(|value| *value != Tag::Int(0) && *value != Tag::Int(1)) && snapshot_view(raw).is_none() { return None; }
    let Tag::Compound(mut fields) = raw.clone() else { return None; };
    // Explicitly unproven, not absent: a new local view must never be
    // mistaken for an unmigrated legacy page while its update is in flight.
    fields.insert("tconstruct:retired_projection".into(), Tag::Int(0));
    Some(Tag::Compound(fields))
}

/// A supported active-view snapshot is exactly the page last proved to be a
/// projection. Retained item bytes or extra snapshot fields are never trusted.
fn snapshot_view(raw: &Tag) -> Option<Vec<Option<horizon_items::items::ItemStack>>> {
    let Tag::Compound(marker) = raw.get("tconstruct:retired_projection")? else { return None; };
    if marker.len() != 2 || marker.get(&"version".into()) != Some(&Tag::Int(1)) { return None; }
    let Some(Tag::List(items)) = marker.get(&"view".into()) else { return None; };
    if items.len() != super::tconstruct_source_smeltery_page::MENU_SIZE { return None; }
    items.iter().map(|raw| {
        if matches!(raw, Tag::Compound(fields) if fields.is_empty()) { return Some(None); }
        let stack = horizon_items::item_nbt::item_from_nbt(raw)?;
        (stack.data().is_none_or(|data| data.wire.patch().is_none()) && horizon_items::item_nbt::item_to_nbt(&stack) == *raw).then_some(Some(stack))
    }).collect()
}

/// Record only an exact complete page, validated against the current saved
/// machine inputs and separately owned containers before creating provenance.
pub(super) fn snapshot_outer(raw: &Tag, live: &[Option<horizon_items::items::ItemStack>], offset: usize, containers: &[Option<horizon_items::items::ItemStack>; 2]) -> Option<Tag> {
    let mut checked = live.to_vec();
    if !close_projection(raw, &mut checked, Some(offset), containers) { return None; }
    let view = live.iter().map(|stack| stack.as_ref().map(horizon_items::item_nbt::item_to_nbt)
        .unwrap_or_else(|| Tag::Compound(Default::default()))).collect();
    let Tag::Compound(mut fields) = raw.clone() else { return None; };
    fields.insert("tconstruct:retired_projection".into(), Tag::compound([
        ("version", Tag::Int(1)), ("view", Tag::List(view))]));
    Some(Tag::Compound(fields))
}

/// Migrate only an exact serialized legacy page. Unknown live additions must
/// agree with neither a guessed offset nor a bag of saved items: the full
/// view must equal both outer Items and one complete legal source projection.
pub(super) fn legacy_snapshot_outer(raw: &Tag, live: &[Option<horizon_items::items::ItemStack>]) -> Option<Tag> {
    if raw.get("tconstruct:retired_projection").is_some() || protect("tconstruct:smeltery", raw)
        || live.len() != super::tconstruct_source_smeltery_page::MENU_SIZE
        || live[..2].iter().any(Option::is_some)
        || raw.get("Items") != Some(&horizon_items::item_nbt::encode_items(live)) { return None; }
    let saved = raw.get("tconstruct:smeltery").and_then(super::tconstruct_source_smeltery_state::Smeltery::from_nbt)?;
    (0..=super::tconstruct_source_smeltery_layout::max_offset(saved.slots.len()))
        .find_map(|offset| snapshot_outer(raw, live, offset, &[None, None]))
}
