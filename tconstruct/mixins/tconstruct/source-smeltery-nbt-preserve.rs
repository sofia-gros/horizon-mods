// Preserve machine and tank compound extensions, not fluid entry fields.
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
    if let Some(old_tank) = old.get(&"tank".into()) {
        let (Tag::Compound(old_fields), Tag::Compound(new_fields)) = (old_tank, next.get(&"tank".into())?) else { return None; };
        let mut tank = old_fields.clone();
        // Do not retain stale legacy aliases beside the current native fields.
        // Earlier writes replaced the tank wholesale and removed these keys.
        for key in ["capacity", "fluids", "FluidName", "Amount", "Tag"] { tank.remove(&key.into()); }
        for (key, value) in new_fields { tank.insert(key.clone(), value.clone()); }
        merged.insert("tank".into(), Tag::Compound(tank));
    }
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
    #[test] fn tank_fields_survive_normal_fluid_changes_without_altering_results() {
        let old_tank = Tag::compound([("capacity", Tag::Long(100)), ("fluids", Tag::List(vec![Tag::Int(1)])), ("future", Tag::Int(42))]);
        let new_tank = Tag::compound([("capacity", Tag::Long(50)), ("fluids", Tag::List(vec![]))]);
        let previous = Tag::compound([("tank", old_tank)]); let updated = Tag::compound([("tank", new_tank.clone())]);
        let actual = merge_top(&previous, &updated).unwrap();
        let tank = actual.get("tank").unwrap();
        assert_eq!(tank.get("future"), Some(&Tag::Int(42)));
        assert_eq!(tank.get("capacity"), new_tank.get("capacity"));
        assert_eq!(tank.get("fluids"), new_tank.get("fluids"));
    }
    #[test] fn tank_without_extra_fields_retains_exact_normal_result() {
        let old = Tag::compound([("tank", Tag::compound([("capacity", Tag::Long(100)), ("fluids", Tag::List(vec![]))]))]);
        let next = Tag::compound([("tank", Tag::compound([("capacity", Tag::Long(50)), ("fluids", Tag::List(vec![Tag::Int(4)]))]))]);
        assert_eq!(merge_top(&old, &next), Some(next));
    }
    #[test] fn stale_forge_aliases_are_not_carried_forward() {
        let old_tank = Tag::compound([("capacity", Tag::Long(100)), ("fluids", Tag::List(vec![])),
            ("FluidName", Tag::String("test:old".into())), ("Amount", Tag::Int(80)),
            ("Tag", Tag::compound([("legacy", Tag::Int(1))])), ("future", Tag::Int(42))]);
        // A serialized update removes them as before; a raw patch that explicitly
        // carries them keeps them, also matching its previous replacement behavior.
        let new_tank = Tag::compound([("capacity", Tag::Long(100)), ("fluids", Tag::List(vec![]))]);
        let previous = Tag::compound([("tank", old_tank.clone())]);
        let result = merge_top(&previous, &Tag::compound([("tank", new_tank)])).unwrap();
        let tank = result.get("tank").unwrap();
        for key in ["FluidName", "Amount", "Tag"] { assert!(tank.get(key).is_none()); }
        assert_eq!(tank.get("future"), Some(&Tag::Int(42)));
        let raw_patch = Tag::compound([("tank", old_tank.clone())]);
        assert_eq!(merge_top(&previous, &raw_patch).unwrap().get("tank"), Some(&old_tank));
    }
    #[test] fn malformed_or_missing_updated_tank_is_refused() {
        let old = Tag::compound([("tank", Tag::compound([("capacity", Tag::Long(100))]))]);
        assert!(merge_top(&old, &Tag::compound([("tank", Tag::Int(1))])).is_none());
        assert!(merge_top(&old, &Tag::compound([("fuel", Tag::Long(1))])).is_none());
    }
    #[test] fn malformed_values_never_produce_a_replacement() {
        let valid = Tag::compound([("fuel", Tag::Long(5))]);
        assert!(merge_top(&Tag::Int(1), &valid).is_none());
        assert!(merge_top(&valid, &Tag::Int(1)).is_none());
    }
}
