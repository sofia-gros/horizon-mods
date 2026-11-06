// Tests of the UNAPPLIED proposed native excerpt, not the installed engine.
use super::{EffectInstance, extension::{Error, Extensions, Limits, Record}};

fn limits() -> Limits {
    Limits { max_layers: 16, max_entries_per_layer: 8, max_key_bytes: 64,
        max_value_bytes: 256, max_id_bytes: 64, max_total_bytes: 4096 }
}
fn owned(duration: i32, amplifier: i32, value: u8) -> EffectInstance {
    let mut e = EffectInstance::new("speed", duration, amplifier);
    e.extensions.insert("example:policy".into(), vec![value], limits()).unwrap();
    e
}
fn policy(e: &EffectInstance) -> u8 { e.extensions.get("example:policy").unwrap()[0] }
fn resolve(id: &str) -> Option<&'static str> { match id { "speed" => Some("speed"), _ => None } }

#[test]
fn new_and_cross_module_style_updates_have_empty_data() {
    let e = EffectInstance { ambient: true, ..EffectInstance::new("speed", 10, 999) };
    assert!(e.extensions.is_empty()); assert!(e.ambient); assert_eq!(e.amplifier, 255);
}
#[test]
fn whole_clone_has_independent_active_and_hidden_payloads() {
    let mut e = owned(100, 0, 1); e.hidden = Some(Box::new(owned(200, 0, 2)));
    let mut copy = e.clone();
    copy.extensions.insert("example:policy".into(), vec![3], limits()).unwrap();
    copy.hidden.as_mut().unwrap().extensions.insert("example:policy".into(), vec![4], limits()).unwrap();
    assert_eq!(policy(&e), 1); assert_eq!(policy(e.hidden.as_ref().unwrap()), 2);
}
#[test]
fn native_equality_does_not_acquire_new_payload_policy() {
    assert_eq!(owned(100, 1, 1), owned(100, 1, 2));
    assert_ne!(owned(100, 1, 1).extensions, owned(100, 1, 2).extensions);
}
#[test]
fn same_level_refresh_and_stronger_replacement_keep_active_policy() {
    let mut e = owned(100, 0, 1); assert!(e.update(&owned(200, 0, 2))); assert_eq!(policy(&e), 1);
    assert!(e.update(&owned(300, 1, 3))); assert_eq!(policy(&e), 1); assert!(e.hidden.is_none());
}
#[test]
fn shorter_stronger_hides_previous_payload_and_restores_it() {
    let mut e = owned(10, 0, 1); e.update(&owned(2, 1, 2));
    assert_eq!(policy(&e), 1); assert_eq!(policy(e.hidden.as_ref().unwrap()), 1);
    e.extensions.insert("example:policy".into(), vec![3], limits()).unwrap();
    e.tick_client(); e.tick_client();
    assert_eq!((e.duration, e.amplifier, policy(&e)), (8, 0, 1)); assert!(e.hidden.is_none());
}
#[test]
fn weaker_longer_copies_incoming_policy_without_incoming_hidden_chain() {
    let mut e = owned(2, 1, 1); let mut incoming = owned(10, 0, 2);
    incoming.hidden = Some(Box::new(owned(100, 0, 9)));
    e.update(&incoming); assert_eq!(policy(&e), 1);
    assert_eq!(policy(e.hidden.as_ref().unwrap()), 2); assert!(e.hidden.as_ref().unwrap().hidden.is_none());
    e.tick_client(); e.tick_client(); assert_eq!(policy(&e), 2);
}
#[test]
fn recursive_hidden_update_retains_that_instances_policy() {
    let mut e = owned(2, 2, 1); e.update(&owned(10, 1, 2)); e.update(&owned(20, 1, 3));
    let hidden = e.hidden.as_ref().unwrap(); assert_eq!(hidden.duration, 20); assert_eq!(policy(hidden), 2);
}
#[test]
fn record_roundtrip_retains_order_unknown_namespaces_and_independence() {
    let mut e = owned(2, 2, 1); e.hidden = Some(Box::new(owned(-1, 1, 2)));
    e.hidden.as_mut().unwrap().extensions.insert("unknown:future/data".into(), vec![0, 255, 3], limits()).unwrap();
    let record = Record::capture(&e, limits()).unwrap(); let mut restored = record.restore(limits(), resolve).unwrap();
    assert_eq!(Record::capture(&restored, limits()).unwrap(), record);
    restored.hidden.as_mut().unwrap().extensions.insert("example:policy".into(), vec![4], limits()).unwrap();
    assert_eq!(policy(e.hidden.as_ref().unwrap()), 2); assert_eq!(record.layers[1].extensions.get("example:policy"), Some([2].as_slice()));
}
#[test]
fn record_limits_and_unresolved_ids_reject_without_clamping_or_dropping() {
    let mut e = owned(1, 1, 1); e.hidden = Some(Box::new(owned(2, 0, 2)));
    let record = Record::capture(&e, limits()).unwrap();
    assert_eq!(Record::capture(&e, Limits { max_layers: 1, ..limits() }), Err(Error::TooManyLayers));
    assert_eq!(record.restore(Limits { max_total_bytes: 1, ..limits() }, resolve), Err(Error::TotalTooLarge));
    let mut bad = record.clone(); bad.layers[1].id = "missing:effect".into();
    assert_eq!(bad.restore(limits(), resolve), Err(Error::UnresolvedId { layer: 1 }));
    bad = record.clone(); bad.layers[0].amplifier = 256;
    assert_eq!(bad.restore(limits(), resolve), Err(Error::InvalidAmplifier));
    assert_eq!(Record { layers: vec![] }.restore(limits(), resolve), Err(Error::EmptyRecord));
    assert_eq!(Record::capture(&e, limits()).unwrap(), record);
}
#[test]
fn extension_validation_is_atomic_and_duplicate_keys_are_not_collapsed() {
    let mut data = Extensions::from_entries(vec![("example:a".into(), vec![1])], limits()).unwrap();
    let before = data.clone();
    for (key, value, bound, error) in [
        ("bad".into(), vec![], limits(), Error::InvalidKey),
        ("example:next".into(), vec![0; 257], limits(), Error::ValueTooLong),
        ("example:b".into(), vec![], Limits { max_entries_per_layer: 1, ..limits() }, Error::TooManyEntries),
    ] { assert_eq!(data.insert(key, value, bound), Err(error)); assert_eq!(data, before); }
    assert_eq!(Extensions::from_entries(vec![("example:a".into(), vec![1]), ("example:a".into(), vec![2])], limits()), Err(Error::DuplicateKey));
    data.insert("example:a".into(), vec![2], limits()).unwrap(); assert_eq!(data.get("example:a"), Some([2].as_slice()));
}

// Production PlayerEffects::saved/from_saved and Level NBT remain unchanged.
// These tests intentionally do not stand in for the missing migration.
