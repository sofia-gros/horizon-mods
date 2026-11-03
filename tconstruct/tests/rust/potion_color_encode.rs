use horizon_items::items::ItemStack;
use horizon_protocol::item::{component, Component, Stack};
use horizon_world::nbt::Tag;

include!("../../mixins/tconstruct/source-potion-color-encode.rs");

fn colored_potion() -> Option<ItemStack> {
    let mut stack = ItemStack::new("minecraft:potion", 1).with_potion("minecraft:healing");
    stack.data_mut().potion_extra.push(("custom_color".into(), Tag::Int(0x123456)));
    Some(stack)
}
fn encoded_potion() -> Option<Stack> {
    let mut stack = Stack::new(1, 1);
    stack.added.push((component::POTION_CONTENTS, Component::Potion { potion: Some(2), color: None, rest: vec![] }));
    Some(stack)
}
#[test]
fn restores_plain_color_in_real_protocol_component() {
    let out = tconstruct_potion_color_encode(encoded_potion(), &colored_potion()).unwrap();
    assert!(matches!(out.component(component::POTION_CONTENTS), Some(Component::Potion { color: Some(0x123456), .. })));
}
#[test]
fn leaves_opaque_wire_extra_effects_and_existing_bytes_untouched() {
    let mut cached = encoded_potion();
    cached.as_mut().unwrap().wire.0 = Some(vec![1].into());
    let out = tconstruct_potion_color_encode(cached.clone(), &colored_potion()).unwrap();
    assert_eq!(out.wire.0.as_deref(), Some(&[1][..]));
    assert_eq!(out.added, cached.unwrap().added);
    let mut source = colored_potion();
    source.as_mut().unwrap().data_mut().potion_extra.push(("custom_effects".into(), Tag::List(vec![])));
    let out = tconstruct_potion_color_encode(encoded_potion(), &source).unwrap();
    assert!(matches!(out.component(component::POTION_CONTENTS), Some(Component::Potion { color: None, .. })));
    let mut encoded = encoded_potion();
    if let Component::Potion { rest, .. } = &mut encoded.as_mut().unwrap().added[0].1 { rest.push(3); }
    let out = tconstruct_potion_color_encode(encoded, &colored_potion()).unwrap();
    assert!(matches!(out.component(component::POTION_CONTENTS), Some(Component::Potion { color: None, rest, .. }) if rest == &[3]));
}
#[test]
fn leaves_existing_duplicate_and_removed_components_untouched() {
    let mut existing = encoded_potion();
    if let Component::Potion { color, .. } = &mut existing.as_mut().unwrap().added[0].1 { *color = Some(7); }
    let out = tconstruct_potion_color_encode(existing, &colored_potion()).unwrap();
    assert!(matches!(out.component(component::POTION_CONTENTS), Some(Component::Potion { color: Some(7), .. })));
    let mut duplicate = encoded_potion();
    let component = duplicate.as_ref().unwrap().added[0].clone();
    duplicate.as_mut().unwrap().added.push(component);
    let out = tconstruct_potion_color_encode(duplicate, &colored_potion()).unwrap();
    assert!(out.added.iter().all(|(_, c)| matches!(c, Component::Potion { color: None, .. })));
    let mut removed = encoded_potion();
    removed.as_mut().unwrap().removed.push(component::POTION_CONTENTS);
    let out = tconstruct_potion_color_encode(removed, &colored_potion()).unwrap();
    assert!(matches!(out.component(component::POTION_CONTENTS), Some(Component::Potion { color: None, .. })));
}
