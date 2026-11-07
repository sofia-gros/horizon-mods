// Preserve machine and tank compound extensions, not fluid entry fields.
use horizon_world::nbt::{JavaString, Tag};

pub(super) fn merge_top(previous: &Tag, updated: &Tag) -> Option<Tag> {
    let (Tag::Compound(old), Tag::Compound(next)) = (previous, updated) else { return None; };
    let mut updates: Vec<(JavaString, Option<Tag>)> = Vec::new();
    // Owned optional fields absent from the update must not be resurrected.
    for key in ["slots", "progress", "tank", "fuel", "fuel_quality", "fuel_view",
        "temperature", "rate", "fuel_cost", "phase", "entity_phase", "heat_codes",
        "slot_count", "heat_start", "heat_page"] {
        updates.push((key.into(), None));
    }
    updates.extend(next.iter().map(|(key, value)| (key.clone(), Some(value.clone()))));
    if let Some(old_tank) = old.get(&"tank".into()) {
        let new_tank = next.get(&"tank".into())?;
        let (Tag::Compound(old_fields), Tag::Compound(new_fields)) = (old_tank, new_tank) else { return None; };
        // An unpaired UTF-16 surrogate would be changed by the existing
        // lossy tank decoder. Permit only an exact raw tank passthrough.
        if let Some(Tag::List(rows)) = old_fields.get(&"fluids".into()) {
            let lossy = rows.iter().any(|row| matches!(row.get("id"), Some(Tag::String(id)) if id.to_string().is_err()));
            if lossy && old_tank != new_tank { return None; }
        }
        let mut tank_updates: Vec<(JavaString, Option<Tag>)> = Vec::new();
        // Do not retain stale legacy aliases beside the current native fields.
        // Earlier writes replaced the tank wholesale and removed these keys.
        for key in ["capacity", "fluids", "FluidName", "Amount", "Tag"] { tank_updates.push((key.into(), None)); }
        tank_updates.extend(new_fields.iter().map(|(key, value)| (key.clone(), Some(value.clone()))));
        let tank = horizon_client::pluto_nbt_storage::patch_compound(old_tank, tank_updates)?;
        updates.push(("tank".into(), Some(tank)));
    }
    horizon_client::pluto_nbt_storage::patch_compound(previous, updates)
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
    #[test] fn lossy_fluid_id_requires_unchanged_raw_tank() {
        let bad = Tag::String(horizon_world::nbt::JavaString(vec![0xd800]));
        let row = Tag::compound([("id", bad), ("amount", Tag::Long(1))]);
        let old_tank = Tag::compound([("capacity", Tag::Long(100)), ("fluids", Tag::List(vec![row]))]);
        let old = Tag::compound([("tank", old_tank.clone()), ("fuel", Tag::Long(10))]);
        let passthrough = Tag::compound([("tank", old_tank.clone()), ("fuel", Tag::Long(9))]);
        assert!(merge_top(&old, &passthrough).is_some());
        let replaced = Tag::compound([("capacity", Tag::Long(100)), ("fluids", Tag::List(vec![Tag::compound([("id", Tag::String("�".into())), ("amount", Tag::Long(1))])]))]);
        assert!(merge_top(&old, &Tag::compound([("tank", replaced)])).is_none());
        let valid = Tag::compound([("capacity", Tag::Long(100)), ("fluids", Tag::List(vec![Tag::compound([("id", Tag::String("test:😀".into())), ("amount", Tag::Long(1))])]))]);
        assert!(merge_top(&Tag::compound([("tank", valid.clone())]), &Tag::compound([("tank", valid)])).is_some());
    }
    #[test] fn malformed_or_missing_updated_tank_is_refused() {
        let old = Tag::compound([("tank", Tag::compound([("capacity", Tag::Long(100))]))]);
        assert!(merge_top(&old, &Tag::compound([("tank", Tag::Int(1))])).is_none());
        assert!(merge_top(&old, &Tag::compound([("fuel", Tag::Long(1))])).is_none());
    }
    #[test] fn merge_keeps_exact_utf16_keys_and_proposal_overlay_order() {
        let old_key = JavaString(vec![0xd800]); let new_key = JavaString(vec![0xdc00]);
        let mut old = horizon_world::nbt::Compound::new();
        old.insert(old_key.clone(), Tag::Int(7)); old.insert("heat_codes".into(), Tag::IntArray(vec![1]));
        let mut next = horizon_world::nbt::Compound::new();
        next.insert(new_key.clone(), Tag::String(JavaString(vec![0xd800])));
        next.insert("heat_codes".into(), Tag::IntArray(vec![2]));
        let saved = merge_top(&Tag::Compound(old), &Tag::Compound(next)).unwrap();
        let Tag::Compound(fields) = saved else { panic!("compound"); };
        assert_eq!(fields.get(&old_key), Some(&Tag::Int(7)));
        assert_eq!(fields.get(&new_key), Some(&Tag::String(JavaString(vec![0xd800]))));
        assert_eq!(fields.get(&"heat_codes".into()), Some(&Tag::IntArray(vec![2])));
    }
    #[test] fn malformed_values_never_produce_a_replacement() {
        let valid = Tag::compound([("fuel", Tag::Long(5))]);
        assert!(merge_top(&Tag::Int(1), &valid).is_none());
        assert!(merge_top(&valid, &Tag::Int(1)).is_none());
    }
}
