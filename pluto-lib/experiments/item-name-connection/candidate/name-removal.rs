// Candidate internal adapter. Not included in the normal PlutoLib package.
use horizon_protocol::item::{Stack, component};
use horizon_world::nbt::Tag;

pub(crate) const ITEM_NAME: &str = "minecraft:item_name";
pub(crate) const REMOVED_ITEM_NAME: &str = "!minecraft:item_name";

/// Keep the native patch's explicit removal before native ItemStack decoding.
///
/// The native components_tag result is None or a Compound at the pinned target.
/// An unexpected noncompound value is left unchanged, not reconstructed. A
/// removal wins over an added name in a contradictory patch, while the native
/// retained wire bytes remain untouched by this owned-Tag operation.
pub(crate) fn receive(mut value: Option<Tag>, source: &Stack) -> Option<Tag> {
    if !source.removed.contains(&component::ITEM_NAME) {
        return value;
    }
    let Tag::Compound(fields) = value.get_or_insert_with(|| Tag::Compound(Default::default())) else {
        return value;
    };
    fields.retain(|key, _| !key.0.iter().copied().eq(ITEM_NAME.encode_utf16()));
    fields.insert(REMOVED_ITEM_NAME.into(), Tag::Compound(Default::default()));
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire(removed: bool) -> Stack {
        let mut source = Stack::new(1, 1);
        if removed { source.removed.push(component::ITEM_NAME); }
        source
    }

    #[test]
    fn no_removal_preserves_none_and_owned_values() {
        assert_eq!(receive(None, &wire(false)), None);
        let value = Tag::compound([("other:opaque", Tag::Long(i64::MIN))]);
        assert_eq!(receive(Some(value.clone()), &wire(false)), Some(value));
    }

    #[test]
    fn removal_without_added_components_becomes_explicit() {
        let value = receive(None, &wire(true)).unwrap();
        assert_eq!(value.get(REMOVED_ITEM_NAME), Some(&Tag::Compound(Default::default())));
        assert_eq!(value.get(ITEM_NAME), None);
    }

    #[test]
    fn removal_preserves_unrelated_raw_tags_and_invalid_utf16_keys() {
        let mut fields = horizon_world::nbt::Compound::new();
        let unknown_key = horizon_world::nbt::JavaString(vec![0xd800, 0x78]);
        fields.insert(unknown_key.clone(), Tag::IntArray(vec![i32::MIN, i32::MAX]));
        fields.insert("other:data".into(), Tag::List(vec![Tag::Byte(-1)]));
        let original = fields.clone();
        let Tag::Compound(mut result) = receive(Some(Tag::Compound(fields)), &wire(true)).unwrap() else { panic!() };
        let marker_key = horizon_world::nbt::JavaString::from(REMOVED_ITEM_NAME);
        assert_eq!(result.remove(&marker_key), Some(Tag::Compound(Default::default())));
        assert_eq!(result, original);
    }

    #[test]
    fn explicit_removal_wins_and_repeated_removals_are_idempotent() {
        let mut source = wire(true);
        source.removed.push(component::ITEM_NAME);
        source.added.push((component::ITEM_NAME, horizon_protocol::item::Component::Text(
            Tag::compound([("translate", Tag::String("item.example.name".into()))]))));
        let input = Tag::compound([(ITEM_NAME, Tag::String("old name".into())),
                                  (REMOVED_ITEM_NAME, Tag::String("noncanonical marker".into()))]);
        let once = receive(Some(input), &source);
        let twice = receive(once.clone(), &source);
        assert_eq!(once, twice);
        assert_eq!(once.unwrap().get(ITEM_NAME), None);
        assert_eq!(source.added.len(), 1);
        assert_eq!(source.removed.len(), 2);
    }

    #[test]
    fn unrelated_removal_does_not_materialize_name_state() {
        let mut source = wire(false);
        source.removed.push(component::LORE);
        assert_eq!(receive(None, &source), None);
    }

    #[test]
    fn unexpected_noncompound_is_preserved_not_guessed() {
        let input = Tag::LongArray(vec![i64::MIN, 0]);
        assert_eq!(receive(Some(input.clone()), &wire(true)), Some(input));
    }
}
