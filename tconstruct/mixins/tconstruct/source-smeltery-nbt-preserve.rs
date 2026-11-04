// Preserve machine-compound top-level extensions; nested fields are not merged.
use horizon_world::nbt::Tag;

pub(super) fn merge_top(previous: &Tag, updated: &Tag) -> Option<Tag> {
    let (Tag::Compound(old), Tag::Compound(next)) = (previous, updated) else { return None; };
    let mut merged = old.clone();
    // Owned optional fields absent from the update must not be resurrected.
    for key in ["slots", "progress", "tank", "fuel", "fuel_quality", "fuel_view",
        "temperature", "rate", "fuel_cost", "phase", "entity_phase", "heat_codes",
        "slot_count", "heat_start", "heat_page"] {
        merged.remove(&key.into());
    }
    for (key, value) in next { merged.insert(key.clone(), value.clone()); }
    Some(Tag::Compound(merged))
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn machine_extension_survives_known_fields_update() {
        let old = Tag::compound([("future", Tag::Int(42)), ("fuel", Tag::Long(20))]);
        let next = Tag::compound([("fuel", Tag::Long(19))]);
        let actual = merge_top(&old, &next).unwrap();
        assert_eq!(actual.get("future"), Some(&Tag::Int(42)));
        assert_eq!(actual.get("fuel"), Some(&Tag::Long(19)));
    }
    #[test] fn omitted_owned_heat_codes_are_removed_but_unknown_fields_stay() {
        let old = Tag::compound([("heat_codes", Tag::IntArray(vec![7])), ("heat_start", Tag::Int(5)), ("heat_page", Tag::IntArray(vec![3])), ("slot_count", Tag::Int(10)), ("future", Tag::Int(42))]);
        let next = Tag::compound([("fuel", Tag::Long(1))]);
        let actual = merge_top(&old, &next).unwrap();
        for key in ["heat_codes", "heat_start", "heat_page", "slot_count"] { assert!(actual.get(key).is_none()); }
        assert_eq!(actual.get("future"), Some(&Tag::Int(42)));
    }
    #[test] fn malformed_values_never_produce_a_replacement() {
        let valid = Tag::compound([("fuel", Tag::Long(5))]);
        assert!(merge_top(&Tag::Int(1), &valid).is_none());
        assert!(merge_top(&valid, &Tag::Int(1)).is_none());
    }
}
