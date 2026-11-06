// Real native protocol/items tests. No Game or socket execution is claimed.
use horizon_items::items::ItemStack;
use horizon_protocol::buf::{Codec, Reader, Writer};
use horizon_protocol::item::{Component, DelimitedSlot, Slot, Stack, WirePatch, component};
use horizon_protocol::registry::{Builtin, Dynamic, Ids};
use horizon_world::nbt::Tag;
use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn registries() -> (Builtin, Dynamic) {
    let mut builtin = Builtin::default();
    builtin.registries.insert("minecraft:item".into(), Ids::from_names(vec!["minecraft:air".into(), "minecraft:stone".into()]));
    let pairs = [
        (component::ITEM_NAME, "minecraft:item_name"),
        (component::CUSTOM_NAME, "minecraft:custom_name"),
        (component::CUSTOM_DATA, "minecraft:custom_data"),
        (component::DAMAGE, "minecraft:damage"),
        (component::MAX_STACK_SIZE, "minecraft:max_stack_size"),
        (component::LORE, "minecraft:lore"),
        (component::CHARGED_PROJECTILES, "minecraft:charged_projectiles"),
        (component::BUNDLE_CONTENTS, "minecraft:bundle_contents"),
        (component::CONTAINER, "minecraft:container"),
    ];
    let mut names = vec![String::new(); pairs.iter().map(|(id, _)| *id as usize).max().unwrap() + 1];
    for (id, name) in pairs { names[id as usize] = name.into(); }
    builtin.registries.insert("minecraft:data_component_type".into(), Ids::from_names(names));
    (builtin, Dynamic::default())
}

fn named_patch() -> Stack {
    let mut stack = Stack::new(1, 1);
    stack.added.push((component::ITEM_NAME, Component::Text(Tag::compound([
        ("translate", Tag::String("item.example.removed".into())),
        ("color", Tag::String("#123456".into())),
    ]))));
    stack.removed.push(component::ITEM_NAME);
    stack
}

fn bytes(stack: &Option<Stack>) -> Vec<u8> {
    let mut writer = Writer::new();
    DelimitedSlot::write(stack, &mut writer);
    writer.data
}

fn wire_roundtrip(stack: Stack) -> Stack {
    let bytes = bytes(&Some(stack));
    let mut reader = Reader::new(&bytes);
    let decoded = DelimitedSlot::read(&mut reader).unwrap().unwrap();
    assert_eq!(reader.remaining(), 0);
    decoded
}

fn received(stack: &Stack) -> ItemStack {
    let (builtin, dynamic) = registries();
    crate::item_sync::from_network(stack, &builtin, &dynamic)
}

fn removed() -> ItemStack {
    received(&wire_roundtrip(named_patch()))
}

fn send(stack: &Option<ItemStack>) -> Option<Stack> {
    crate::guest::stack_to_network(|_| 1, |_, _| None, stack)
}

fn marker(stack: &ItemStack) -> bool {
    stack.data().is_some_and(|d| d.other("!minecraft:item_name").is_some())
}

fn assert_removed(stack: &Stack) {
    assert_eq!(stack.component(component::ITEM_NAME), None);
    assert_eq!(stack.removed.iter().filter(|id| **id == component::ITEM_NAME).count(), 1);
}

#[test]
fn receive_removal_precedes_native_wire_baseline() {
    let wire = wire_roundtrip(named_patch());
    let original_patch = wire.wire.0.clone().unwrap();
    let stack = received(&wire);
    assert!(marker(&stack));
    assert!(stack.item_name().is_none());
    assert_eq!(stack.wire_patch().unwrap().as_ref(), original_patch.as_ref());
    assert!(wire.component(component::ITEM_NAME).is_some());
}

#[test]
fn canonical_root_removal_wins_without_mutating_input() {
    let input = removed();
    let before = input.clone();
    let raw = input.wire_patch().unwrap();
    let out = send(&Some(input.clone())).unwrap();
    assert_removed(&out);
    assert_eq!(input, before);
    assert_eq!(input.wire_patch().unwrap().as_ref(), raw.as_ref());
    assert!(out.wire.0.is_none());
}

#[test]
fn no_marker_matches_unchanged_native_send() {
    let mut input = ItemStack::new("minecraft:stone", 3);
    input.damage = 4;
    input.data_mut().set_other("minecraft:item_name", Some(Tag::String("kept".into())));
    let input = Some(input);
    let expected = crate::native_guest::stack_to_network(|_| 1, |_, _| None, &input);
    assert_eq!(bytes(&send(&input)), bytes(&expected));
}

#[test]
fn no_removal_matches_unchanged_native_receive() {
    let (builtin, dynamic) = registries();
    let mut wire = named_patch();
    wire.removed.clear();
    let wire = wire_roundtrip(wire);
    let expected = crate::native_sync::from_network(&wire, &builtin, &dynamic);
    let actual = received(&wire);
    assert_eq!(actual, expected);
    assert_eq!(actual.wire_patch().unwrap().as_ref(), expected.wire_patch().unwrap().as_ref());
}

#[test]
fn charged_children_use_actual_native_template_codec() {
    let mut root = ItemStack::new("minecraft:stone", 1);
    root.data_mut().charged.push(removed());
    let out = wire_roundtrip(send(&Some(root)).unwrap());
    let Component::Stacks(children) = out.component(component::CHARGED_PROJECTILES).unwrap() else { panic!() };
    assert_removed(&children[0]);
    assert!(marker(&received(&out).data().unwrap().charged[0]));
}

#[test]
fn bundle_children_use_actual_native_template_codec() {
    let mut root = ItemStack::new("minecraft:stone", 1);
    root.data_mut().bundle_contents.push(removed());
    let out = wire_roundtrip(send(&Some(root)).unwrap());
    let Component::Stacks(children) = out.component(component::BUNDLE_CONTENTS).unwrap() else { panic!() };
    assert_removed(&children[0]);
    assert!(marker(&received(&out).data().unwrap().bundle_contents[0]));
}

#[test]
fn container_children_preserve_native_raw_container_encoding() {
    let mut root = ItemStack::new("minecraft:stone", 1);
    root.data_mut().container.push((3, removed()));
    let out = wire_roundtrip(send(&Some(root)).unwrap());
    assert!(matches!(out.component(component::CONTAINER), Some(Component::Raw(_))));
    let decoded = received(&out);
    assert_eq!(decoded.data().unwrap().container[0].0, 3);
    assert!(marker(&decoded.data().unwrap().container[0].1));
}

#[test]
fn mixed_nested_entries_run_each_native_body_once_in_order() {
    let mut root = removed(); root.item = "example:root".into();
    let mut charged = removed(); charged.item = "example:charged".into();
    let mut bundle = removed(); bundle.item = "example:bundle".into();
    let mut container = removed(); container.item = "example:container".into();
    charged.data_mut().bundle_contents.push(bundle);
    root.data_mut().charged.push(charged);
    root.data_mut().container.push((0, container));
    let visited = RefCell::new(Vec::new());
    let output = crate::guest::stack_to_network(|name| { visited.borrow_mut().push(name.to_owned()); 1 }, |_, _| None, &Some(root));
    assert!(output.is_some());
    assert_eq!(*visited.borrow(), ["example:root", "example:charged", "example:bundle", "example:container"]);
}

#[test]
fn unrelated_custom_data_and_source_patch_are_not_mutated() {
    let mut wire = named_patch();
    let mut unknown = horizon_world::nbt::Compound::new();
    unknown.insert(horizon_world::nbt::JavaString(vec![0xd800]), Tag::LongArray(vec![i64::MIN, 9]));
    let custom = Tag::Compound(unknown);
    wire.added.push((component::CUSTOM_DATA, Component::Nbt(custom.clone())));
    let wire = wire_roundtrip(wire);
    let before = bytes(&Some(wire.clone()));
    let input = received(&wire);
    let output = send(&Some(input)).unwrap();
    assert_eq!(output.component(component::CUSTOM_DATA), Some(&Component::Nbt(custom)));
    assert_eq!(bytes(&Some(wire)), before);
}

#[test]
fn usable_raw_patch_and_count_only_echo_keep_original_wire() {
    let wire = wire_roundtrip(named_patch());
    let original_patch = wire.wire.0.clone().unwrap();
    let mut input = received(&wire);
    input.count = 7;
    let mut echo = Stack::new(1, input.count as i32);
    echo.wire = WirePatch(Some(input.wire_patch().unwrap()));
    let echo = wire_roundtrip(echo);
    assert_eq!(echo.count, 7);
    assert_eq!(echo.wire.0.as_ref().unwrap().as_ref(), original_patch.as_ref());
    assert!(echo.component(component::ITEM_NAME).is_some());
    assert!(echo.removed.contains(&component::ITEM_NAME));
}

#[test]
fn editing_native_baseline_selects_a_distinct_canonical_proposal() {
    let mut input = removed();
    input.damage = 2;
    assert!(input.wire_patch().is_none());
    let out = send(&Some(input)).unwrap();
    assert_removed(&out);
    assert_eq!(out.component(component::DAMAGE), Some(&Component::Int(2)));
}

#[test]
fn callback_panic_does_not_poison_later_native_conversion() {
    let input = Some(removed());
    assert!(catch_unwind(AssertUnwindSafe(|| {
        crate::guest::stack_to_network(|_| panic!("native item lookup"), |_, _| None, &input)
    })).is_err());
    let calls = Cell::new(0);
    let output = crate::guest::stack_to_network(|_| { calls.set(calls.get()+1); 1 }, |_, _| None, &input).unwrap();
    assert_removed(&output);
    assert_eq!(calls.get(), 1);
}

#[test]
fn nested_callback_panic_unwinds_all_entry_scopes() {
    let mut root = removed();
    root.data_mut().bundle_contents.push(removed());
    let input = Some(root);
    let calls = Cell::new(0);
    assert!(catch_unwind(AssertUnwindSafe(|| {
        crate::guest::stack_to_network(|_| { calls.set(calls.get()+1); if calls.get()==2 { panic!("nested lookup") } 1 }, |_, _| None, &input)
    })).is_err());
    calls.set(0);
    let out = crate::guest::stack_to_network(|_| { calls.set(calls.get()+1); 1 }, |_, _| None, &input).unwrap();
    assert_removed(&out);
    assert_eq!(calls.get(), 2);
}

#[test]
fn parallel_native_conversions_keep_entry_state_thread_local() {
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
    let workers: Vec<_> = (0..4).map(|_| {
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            let input = Some(removed());
            let calls = Cell::new(0);
            let out = crate::guest::stack_to_network(|_| { barrier.wait(); calls.set(calls.get()+1); 1 }, |_, _| None, &input).unwrap();
            assert_removed(&out);
            assert_eq!(calls.get(), 1);
            assert_removed(&send(&input).unwrap());
        })
    }).collect();
    for worker in workers { worker.join().unwrap(); }
}

#[test]
fn none_retains_native_no_callback_behavior() {
    let out = crate::guest::stack_to_network(|_| panic!("None must not call registry"), |_, _| None, &None);
    assert!(out.is_none());
}

#[test]
fn duplicate_and_contradictory_removals_have_one_canonical_name_removal() {
    let mut wire = named_patch(); wire.removed.push(component::ITEM_NAME);
    let input = received(&wire_roundtrip(wire));
    assert_removed(&send(&Some(input)).unwrap());
}

#[test]
fn explicit_custom_name_remains_independent_from_removed_item_name() {
    let mut wire = named_patch();
    wire.added.push((component::CUSTOM_NAME, Component::Text(Tag::String("Anvil name".into()))));
    let input = received(&wire_roundtrip(wire));
    assert_eq!(input.data().unwrap().custom_name_plain().as_deref(), Some("Anvil name"));
    assert!(input.item_name().is_none());
    let out = send(&Some(input)).unwrap();
    assert_removed(&out);
    assert_eq!(out.component(component::CUSTOM_NAME), Some(&Component::Text(Tag::String("Anvil name".into()))));
}

#[test]
fn ordinary_slot_codec_also_carries_explicit_removal() {
    let output = send(&Some(removed()));
    let mut writer = Writer::new(); Slot::write(&output, &mut writer);
    let mut reader = Reader::new(&writer.data);
    assert_removed(&Slot::read(&mut reader).unwrap().unwrap());
    assert_eq!(reader.remaining(), 0);
}
