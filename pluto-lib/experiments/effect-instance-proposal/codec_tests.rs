// Proposed codec with the complete unchanged official NBT module, not a mock Tag.
use super::{EffectInstance, extension::{Extensions, Limits, Record, Error as RecordError},
    nbt::{self, JavaString, NamedTag, Tag}, record_nbt::{self as codec, Error}};

fn limits() -> Limits { Limits { max_layers: 8, max_entries_per_layer: 8, max_key_bytes: 64,
    max_value_bytes: 128, max_id_bytes: 64, max_total_bytes: 2048 } }
fn wire() -> nbt::Limits { nbt::Limits { max_bytes: 4096, max_nodes: 256, max_depth: 16 } }
fn record() -> Record {
    let mut e = EffectInstance::new("speed", 12, 255);
    e.extensions = Extensions::from_entries(vec![("unknown:future".into(), vec![0, 128, 255])], limits()).unwrap();
    let mut hidden = EffectInstance::new("speed", -1, 0); hidden.ambient = true; hidden.visible = false;
    hidden.extensions = Extensions::from_entries(vec![("another:policy".into(), vec![])], limits()).unwrap();
    e.hidden = Some(Box::new(hidden)); Record::capture(&e, limits()).unwrap()
}
fn field<'a>(tag: &'a mut Tag, key: &str) -> &'a mut Tag { tag.get_mut(key).unwrap() }
fn layer(tag: &mut Tag) -> &mut Tag {
    let Tag::List(layers) = field(tag, "layers") else { panic!() }; &mut layers[0]
}

#[test]
fn actual_named_nbt_roundtrip_preserves_hidden_order_and_unknown_payload_bytes() {
    let r = record(); let bytes = codec::encode(&r, limits(), wire()).unwrap();
    let back = codec::decode(&bytes, limits(), wire()).unwrap(); assert_eq!(back, r);
    assert_eq!(codec::encode(&back, limits(), wire()).unwrap(), bytes);
    assert_eq!(back.layers[0].amplifier, 255); assert_eq!(back.layers[1].duration, -1);
    assert_eq!(back.layers[0].extensions.get("unknown:future"), Some([0, 128, 255].as_slice()));
}
#[test]
fn empty_extension_state_is_explicit_and_not_an_implicit_curative_list() {
    let r = Record::capture(&EffectInstance::new("speed", 20, 0), limits()).unwrap();
    let back = codec::decode(&codec::encode(&r, limits(), wire()).unwrap(), limits(), wire()).unwrap();
    assert!(back.layers[0].extensions.is_empty()); assert_eq!(back, r);
}
#[test]
fn unknown_version_and_fields_fail_without_mutating_input() {
    let original = codec::to_tag(&record(), limits()).unwrap();
    let mut changed = original.clone(); *field(&mut changed, "version") = Tag::Int(2);
    let before = changed.clone(); assert_eq!(codec::from_tag(&changed, limits()), Err(Error::Version)); assert_eq!(changed, before);
    let mut changed = original.clone(); let Tag::Compound(map) = &mut changed else { panic!() };
    map.insert("unknown".into(), Tag::Byte(1)); let before = changed.clone();
    assert_eq!(codec::from_tag(&changed, limits()), Err(Error::Schema)); assert_eq!(changed, before);
    let mut changed = original; let Tag::Compound(map) = layer(&mut changed) else { panic!() };
    map.insert("show_icon".into(), Tag::Byte(0));
    assert_eq!(codec::from_tag(&changed, limits()), Err(Error::Schema));
}
#[test]
fn malformed_types_utf16_and_amplifiers_are_not_normalized() {
    let original = codec::to_tag(&record(), limits()).unwrap();
    for (key, value, error) in [
        ("amplifier", Tag::Byte(-1), Error::Schema),
        ("amplifier", Tag::Int(256), Error::Record(RecordError::InvalidAmplifier)),
        ("visible", Tag::Byte(2), Error::Schema),
        ("id", Tag::String(JavaString(vec![0xd800])), Error::InvalidUtf16),
    ] {
        let mut changed = original.clone(); *field(layer(&mut changed), key) = value;
        assert_eq!(codec::from_tag(&changed, limits()), Err(error));
    }
}
#[test]
fn hidden_layer_and_aggregate_payload_limits_are_enforced() {
    let tag = codec::to_tag(&record(), limits()).unwrap();
    assert_eq!(codec::from_tag(&tag, Limits { max_layers: 1, ..limits() }), Err(Error::Record(RecordError::TooManyLayers)));
    assert_eq!(codec::from_tag(&tag, Limits { max_total_bytes: 1, ..limits() }), Err(Error::Record(RecordError::TotalTooLarge)));
    assert_eq!(codec::from_tag(&tag, Limits { max_value_bytes: 2, ..limits() }), Err(Error::Record(RecordError::ValueTooLong)));
}
#[test]
fn native_wire_limits_truncation_trailing_bytes_and_named_roots_fail() {
    let r = record(); let bytes = codec::encode(&r, limits(), wire()).unwrap();
    assert!(matches!(codec::encode(&r, limits(), nbt::Limits { max_bytes: 1, ..wire() }), Err(Error::Nbt(_))));
    assert!(matches!(codec::decode(&bytes, limits(), nbt::Limits { max_nodes: 1, ..wire() }), Err(Error::Nbt(_))));
    assert!(matches!(codec::decode(&bytes, limits(), nbt::Limits { max_depth: 1, ..wire() }), Err(Error::Nbt(_))));
    for end in [0, 1, bytes.len() - 1] { assert!(codec::decode(&bytes[..end], limits(), wire()).is_err()); }
    let mut extra = bytes; extra.push(0); assert_eq!(codec::decode(&extra, limits(), wire()), Err(Error::TrailingBytes));
    let named = nbt::write_named(&NamedTag { name: "unexpected".into(), tag: codec::to_tag(&r, limits()).unwrap() }, wire()).unwrap();
    assert_eq!(codec::decode(&named, limits(), wire()), Err(Error::RootName));
}
#[test]
fn native_legacy_player_tags_are_not_misrepresented_as_this_new_schema() {
    let legacy = Tag::compound([("id", Tag::String("minecraft:speed".into())),
        ("duration", Tag::Int(20)), ("amplifier", Tag::Byte(-1)),
        ("ambient", Tag::Byte(0)), ("show_particles", Tag::Byte(1))]);
    assert_eq!(codec::from_tag(&legacy, limits()), Err(Error::Schema));
    assert_eq!(codec::from_tag(&Tag::List(vec![legacy]), limits()), Err(Error::Schema));
}
#[test]
fn oversized_modified_utf_is_rejected_before_native_disk_writer_can_empty_it() {
    let mut r = record(); r.layers[0].id = "\0".repeat(32768);
    let large = Limits { max_id_bytes: 100000, max_total_bytes: 200000, ..limits() };
    assert_eq!(codec::to_tag(&r, large), Err(Error::UnencodableString));
    // Logical UTF-8 bytes alone are not the native modified-UTF length.
    assert_eq!(r.layers[0].id.len(), 32768);
}
