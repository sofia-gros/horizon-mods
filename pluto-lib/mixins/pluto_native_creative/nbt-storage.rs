// Non-mutating native NBT compound patches with caller-owned schema policy.
use horizon_world::nbt::{JavaString, Tag};

/// Stage ordered field updates without changing the original compound.
/// `Some(value)` sets a field; `None` removes it. Repeated keys apply in order,
/// so the last update wins. Untouched keys and values are cloned exactly,
/// including unknown fields and unpaired UTF-16 surrogates.
///
/// A non-compound original fails before the update iterator is consumed.
/// The caller owns field names, aliases, value validation and authorization.
/// This helper neither decodes values nor persists or locks live storage.
pub fn patch_compound<K: Into<JavaString>>(
    original: &Tag,
    updates: impl IntoIterator<Item = (K, Option<Tag>)>,
) -> Option<Tag> {
    let Tag::Compound(fields) = original else { return None; };
    let mut fields = fields.clone();
    for (key, value) in updates {
        let key = key.into();
        match value {
            Some(value) => { fields.insert(key, value); }
            None => { fields.remove(&key); }
        }
    }
    Some(Tag::Compound(fields))
}

/// Copy only caller-owned fields from a compound proposal onto the original.
/// An owned key omitted by the proposal is removed, not resurrected from old
/// data. Proposal fields outside `owned_keys` are ignored. Both values must be
/// compounds. Schema validation and nested patches remain the caller's job.
pub fn patch_owned_fields(original: &Tag, proposal: &Tag, owned_keys: &[&str]) -> Option<Tag> {
    if !matches!(proposal, Tag::Compound(_)) { return None; }
    patch_compound(original, owned_keys.iter().map(|key| (*key, proposal.get(key).cloned())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_updates_set_remove_and_preserve_unowned_fields() {
        let original = Tag::compound([("keep", Tag::Int(7)), ("remove", Tag::Int(2))]);
        let result = patch_compound(&original, [
            ("new", Some(Tag::Int(1))), ("new", Some(Tag::Int(3))),
            ("remove", None), ("missing", None),
        ]).unwrap();
        assert_eq!(result, Tag::compound([("keep", Tag::Int(7)), ("new", Tag::Int(3))]));
        assert_eq!(original.get("remove"), Some(&Tag::Int(2)));
        assert!(original.get("new").is_none());
        assert_eq!(patch_compound(&original, std::iter::empty::<(&str, Option<Tag>)>()), Some(original));
    }

    #[test]
    fn last_update_can_remove_a_new_field() {
        let original = Tag::compound([] as [(&str, Tag); 0]);
        assert_eq!(patch_compound(&original, [("x", Some(Tag::Int(1))), ("x", None)]), Some(original));
    }

    #[test]
    fn unknown_utf16_keys_nested_values_and_float_bits_are_untouched() {
        let opaque_key = JavaString(vec![0xd800]);
        let opaque_value = Tag::List(vec![Tag::String(JavaString(vec![0xdc00])), Tag::Float(0x7fc01234)]);
        let mut fields = horizon_world::nbt::Compound::new();
        fields.insert(opaque_key.clone(), opaque_value.clone());
        fields.insert("owned".into(), Tag::Int(1));
        let original = Tag::Compound(fields);
        let result = patch_compound(&original, [("owned", Some(Tag::Int(2)))]).unwrap();
        let Tag::Compound(fields) = result else { panic!("compound"); };
        assert_eq!(fields.get(&opaque_key), Some(&opaque_value));
        let changed = patch_compound(&original, [(opaque_key.clone(), Some(Tag::Int(5)))]).unwrap();
        let Tag::Compound(fields) = changed else { panic!("compound"); };
        assert_eq!(fields.get(&opaque_key), Some(&Tag::Int(5)));
        let Tag::Compound(fields) = original else { panic!("compound"); };
        assert_eq!(fields.get(&opaque_key), Some(&opaque_value));
    }

    #[test]
    fn malformed_original_does_not_consume_update_iterator() {
        let updates = std::iter::once_with(|| -> (&str, Option<Tag>) { panic!("must not consume updates"); });
        assert!(patch_compound(&Tag::Int(1), updates).is_none());
    }

    #[test]
    fn owned_patch_removes_missing_owned_fields_and_ignores_other_proposals() {
        let original = Tag::compound([("owned", Tag::Int(1)), ("empty", Tag::Int(2)), ("unknown", Tag::Int(3))]);
        let proposal = Tag::compound([("owned", Tag::Int(4)), ("unknown", Tag::Int(5)), ("new_unknown", Tag::Int(6))]);
        let saved = patch_owned_fields(&original, &proposal, &["owned", "empty"]).unwrap();
        assert_eq!(saved, Tag::compound([("owned", Tag::Int(4)), ("unknown", Tag::Int(3))]));
        assert_eq!(original.get("empty"), Some(&Tag::Int(2)));
        assert_eq!(proposal.get("unknown"), Some(&Tag::Int(5)));
    }

    #[test]
    fn owned_patch_rejects_malformed_values_even_with_no_owned_keys() {
        let compound = Tag::compound([] as [(&str, Tag); 0]);
        assert!(patch_owned_fields(&compound, &Tag::Int(1), &[]).is_none());
        assert!(patch_owned_fields(&Tag::Int(1), &compound, &[]).is_none());
        assert_eq!(patch_owned_fields(&compound, &compound, &[]), Some(compound));
    }
}
