use super::{legacy_oracle, legacy_view::{self, Error, Limits}, nbt::{JavaString, Tag}};
fn limits() -> Limits { Limits { max_entries: 32, max_fields_per_entry: 16, max_id_utf16_units: 128, max_total_id_utf16_units: 512 } }
fn player(effects: Tag) -> Tag { Tag::compound([("active_effects", effects)]) }
fn entry(fields: impl IntoIterator<Item = (&'static str, Tag)>) -> Tag {
    Tag::compound(std::iter::once(("id", Tag::String("minecraft:speed".into()))).chain(fields))
}
fn equal_native(player: &Tag) {
    let inspected = legacy_view::inspect(player.get("active_effects"), limits()).unwrap();
    assert_eq!(inspected.native_tuples().cloned().collect::<Vec<_>>(), legacy_oracle::decode(player));
}
#[test]
fn proposal_legacy_missing_wrong_root_and_empty_are_distinct() {
    let missing = Tag::Compound(Default::default());
    let absent = legacy_view::inspect(missing.get("active_effects"), limits()).unwrap();
    assert!(absent.original.is_none()); assert!(!absent.wrong_root_type);
    let wrong = player(Tag::Int(7)); let before = wrong.clone();
    let inspected = legacy_view::inspect(wrong.get("active_effects"), limits()).unwrap();
    assert!(inspected.wrong_root_type); assert!(inspected.entries.is_empty());
    assert!(std::ptr::eq(inspected.original.unwrap(), wrong.get("active_effects").unwrap()));
    equal_native(&missing); equal_native(&wrong); equal_native(&player(Tag::List(vec![])));
    assert_eq!(before, wrong);
}
#[test]
fn proposal_legacy_exact_numeric_defaults_and_unsigned_byte() {
    let numerics = vec![Tag::Byte(-1), Tag::Short(-2), Tag::Int(300), Tag::Long(0x1_0000_0001),
        Tag::Float(2.75f32.to_bits()), Tag::Double((-3.9f64).to_bits()), Tag::Double(f64::NAN.to_bits()),
        Tag::Double(f64::INFINITY.to_bits()), Tag::String("wrong".into())];
    for number in numerics {
        let raw = player(Tag::List(vec![entry([("duration", number.clone()), ("amplifier", number.clone()),
            ("ambient", number.clone()), ("show_particles", number)])]));
        equal_native(&raw);
    }
    let raw = player(Tag::List(vec![entry([]), entry([("amplifier", Tag::Byte(-1))])]));
    let view = legacy_view::inspect(raw.get("active_effects"), limits()).unwrap();
    assert_eq!(view.entries[0].native_tuple.as_ref().unwrap(), &("minecraft:speed".into(), 0, 0, false, true));
    assert_eq!(view.entries[1].native_tuple.as_ref().unwrap().2, 255);
    equal_native(&raw);
}
#[test]
fn proposal_legacy_omissions_and_lossy_ids_remain_visible() {
    let raw = player(Tag::List(vec![Tag::Int(2), Tag::compound([("id", Tag::Int(3))]),
        Tag::compound([("id", Tag::String(JavaString(vec![0xd800])))])]));
    let original = raw.clone();
    let view = legacy_view::inspect(raw.get("active_effects"), limits()).unwrap();
    assert_eq!(view.entries.len(), 3); assert_eq!(view.native_tuples().count(), 1);
    assert!(view.entries[0].notes.omitted_by_native && view.entries[1].notes.omitted_by_native);
    assert!(view.entries[2].notes.lossy_id);
    assert_eq!(view.entries[2].native_tuple.as_ref().unwrap().0, "\u{fffd}");
    equal_native(&raw); assert_eq!(raw, original);
}
#[test]
fn proposal_legacy_hidden_icons_unknown_fields_are_retained_not_adopted() {
    let raw = player(Tag::List(vec![entry([("hidden_effect", entry([("duration", Tag::Int(500))])),
        ("show_icon", Tag::Byte(0)), ("unknown:future", Tag::ByteArray(vec![0, 255]))])]));
    let view = legacy_view::inspect(raw.get("active_effects"), limits()).unwrap();
    let item = &view.entries[0];
    assert!(item.notes.hidden_present && item.notes.icon_present && item.notes.unknown_fields);
    assert!(item.original.get("hidden_effect").is_some());
    assert_eq!(item.original.get("unknown:future"), Some(&Tag::ByteArray(vec![0,255])));
    // Particle default remains true; no inferred link to an independent icon.
    assert!(item.native_tuple.as_ref().unwrap().4); equal_native(&raw);
}
#[test]
fn proposal_legacy_keeps_order_duplicates_and_unresolved_ids() {
    let raw = player(Tag::List(vec![entry([("duration", Tag::Int(2))]),
        Tag::compound([("id", Tag::String("missing:effect".into()))]),
        entry([("duration", Tag::Int(3))])]));
    let view = legacy_view::inspect(raw.get("active_effects"), limits()).unwrap();
    assert_eq!(view.native_tuples().map(|x| (x.0.as_str(), x.1)).collect::<Vec<_>>(),
        [("minecraft:speed",2), ("missing:effect",0), ("minecraft:speed",3)]);
    equal_native(&raw); // No registry lookup, merge or constructor was performed.
}
#[test]
fn proposal_legacy_rejects_allocation_bounds_without_changing_source() {
    let raw = player(Tag::List(vec![entry([]), entry([])])); let before = raw.clone();
    let l = limits();
    for (limits, error) in [(Limits { max_entries: 1, ..l }, Error::TooManyEntries),
        (Limits { max_fields_per_entry: 0, ..l }, Error::TooManyFields),
        (Limits { max_id_utf16_units: 1, ..l }, Error::IdTooLong),
        (Limits { max_total_id_utf16_units: 20, ..l }, Error::TotalIdsTooLong)] {
        assert_eq!(legacy_view::inspect(raw.get("active_effects"), limits).unwrap_err(), error);
        assert_eq!(raw, before);
    }
}
#[test]
fn proposal_legacy_out_of_range_is_not_silently_clamped() {
    let raw = player(Tag::List(vec![entry([("amplifier", Tag::Int(-1))]), entry([("amplifier", Tag::Int(300))])]));
    let view = legacy_view::inspect(raw.get("active_effects"), limits()).unwrap();
    assert!(view.entries.iter().all(|e| e.notes.amplifier_outside_record_range));
    assert_eq!(view.native_tuples().map(|t| t.2).collect::<Vec<_>>(), [-1,300]);
    equal_native(&raw);
}
