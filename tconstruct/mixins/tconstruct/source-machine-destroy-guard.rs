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
        Some("id" | "x" | "y" | "z" | "Items") ) && key.to_string().ok().as_deref() != Some(kind)) { return true; }
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
    live_matches_with_closed_projection(kind, raw, live, page_offset, false)
}

/// A closed projection may be empty only after a verified local close. The
/// proof is ephemeral and is revoked on open, removal, unload or world reset.
pub(super) fn live_matches_with_closed_projection(kind: &str, raw: &Tag, live: &[Option<horizon_items::items::ItemStack>], page_offset: Option<usize>, closed_projection: bool) -> bool {
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
        (Some(a),Some(b)) => a.wire_patch().is_none() && b.wire_patch().is_none()
            && a.count == b.count && a.damage == b.damage && a.same_item_same_components(b),
        _ => false,
    };
    if kind == "tconstruct:melter" {
        return live.len() >= saved.len() && live.iter().take(saved.len()).zip(&saved).all(|(a,b)| equal(a,b))
            && live.iter().skip(saved.len()).all(Option::is_none);
    }
    if closed_projection && page_offset.is_none()
        && live.len() == super::tconstruct_source_smeltery_page::MENU_SIZE
        && live.iter().all(Option::is_none) { return true; }
    if live.iter().all(Option::is_none) && saved.iter().all(Option::is_none) { return true; }
    let Some(offset) = page_offset else { return false; };
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
    if live.len() != expected.len() || !live.iter().zip(&expected).all(|(a,b)| match (a,b) {
        (None,None) => true,
        (Some(a),Some(b)) => a.wire_patch().is_none() && b.wire_patch().is_none()
            && a.count == b.count && a.damage == b.damage && a.same_item_same_components(b),
        _ => false,
    }) { return false; }
    live.fill(None);
    true
}
