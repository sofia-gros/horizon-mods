//! Run as an example of the pinned Horizon client library; see docs/component-transport.md.
//! The registry and wire round trip follow Horizon's item_sync tests.
use horizon_client_lib::{item_sync, items::ItemStack, lan::guest::stack_to_network};
use horizon_protocol::{
    buf::{Codec, Reader, Writer},
    item::{component, Slot},
    registry::{Builtin, Dynamic, Ids},
};
use horizon_world::nbt::Tag;

mod max_damage_adapter {
    use horizon_client_lib::items::ItemStack;
    use horizon_protocol::item::Stack;
    include!("../mixins/pluto_transport_3a/max.rs");
    pub fn apply(value: Option<Stack>, source: &Option<ItemStack>) -> Option<Stack> {
        add_max_damage(value, source)
    }
}

fn main() {
    let mut names = vec![String::new(); 128];
    for (id, name) in [
        (component::CUSTOM_DATA, "minecraft:custom_data"),
        (component::LORE, "minecraft:lore"),
        (component::ITEM_MODEL, "minecraft:item_model"),
        (component::DAMAGE, "minecraft:damage"),
        (component::MAX_DAMAGE, "minecraft:max_damage"),
        (28, "minecraft:tool"),
        (component::UNBREAKABLE, "minecraft:unbreakable"),
    ] {
        names[id as usize] = name.into();
    }
    let builtin = Builtin {
        registries: [
            (
                "minecraft:item".into(),
                Ids::from_names(vec![
                    "minecraft:air".into(),
                    "example:tool".into(),
                    "minecraft:stick".into(),
                ]),
            ),
            (
                "minecraft:data_component_type".into(),
                Ids::from_names(names),
            ),
        ]
        .into(),
    };
    let nested = Tag::compound([
        ("other:owner", Tag::String("kept".into())),
        (
            "example:nested",
            Tag::compound([("values", Tag::List(vec![Tag::Int(4), Tag::Int(9)]))]),
        ),
    ]);
    let lore = Tag::List(vec![Tag::compound([(
        "text",
        Tag::String("Example lore".into()),
    )])]);
    let model = Tag::String("example:item/tool".into());
    let mut stack = ItemStack::new("example:tool", 3);
    stack.damage = 5;
    stack
        .data_mut()
        .set_other("minecraft:custom_data", Some(nested.clone()));
    stack
        .data_mut()
        .set_other("minecraft:lore", Some(lore.clone()));
    stack
        .data_mut()
        .set_other("minecraft:item_model", Some(model.clone()));
    stack.data_mut().set_other(
        "minecraft:unbreakable",
        Some(Tag::Compound(Default::default())),
    );
    let ids = |name: &str| {
        builtin
            .get("minecraft:item")
            .and_then(|r| r.id(name))
            .unwrap_or(0)
    };
    // Both directions share the underlying conversion; run two encoded hops.
    for _ in 0..2 {
        let wire = stack_to_network(ids, |_, _| None, &Some(stack)).unwrap();
        for id in [
            component::CUSTOM_DATA,
            component::LORE,
            component::ITEM_MODEL,
            component::DAMAGE,
            component::UNBREAKABLE,
        ] {
            assert_eq!(
                wire.added.iter().filter(|(key, _)| *key == id).count(),
                1,
                "component {id} must appear once"
            );
        }
        let mut writer = Writer::new();
        Slot::write(&Some(wire), &mut writer);
        let mut reader = Reader::new(&writer.data);
        let wire = Slot::read(&mut reader).unwrap().unwrap();
        assert_eq!(reader.remaining(), 0);
        stack = item_sync::from_network(&wire, &builtin, &Dynamic::default());
        assert_eq!(
            (&*stack.item, stack.count, stack.damage),
            ("example:tool", 3, 5)
        );
        let data = stack.data().unwrap();
        assert_eq!(data.other("minecraft:custom_data"), Some(&nested));
        assert_eq!(data.other("minecraft:lore"), Some(&lore));
        assert_eq!(data.other("minecraft:item_model"), Some(&model));
        assert!(data.other("minecraft:unbreakable").is_some());
    }
    let patch = stack.wire_patch().expect("decoded server patch");
    stack.count = 4;
    assert_eq!(stack.wire_patch().as_deref(), Some(patch.as_ref()));
    // These valid item components are not encoded from a newly built stack by this host.
    let mut tool = ItemStack::new("example:tool", 1);
    tool.data_mut()
        .set_other("minecraft:max_damage", Some(Tag::Int(60)));
    tool.data_mut().set_other(
        "minecraft:tool",
        Some(Tag::compound([("damage_per_block", Tag::Int(1))])),
    );
    let source = Some(tool);
    let wire = stack_to_network(ids, |_, _| None, &source).unwrap();
    assert!(!wire
        .added
        .iter()
        .any(|(id, _)| *id == component::MAX_DAMAGE));
    assert!(!wire.added.iter().any(|(id, _)| *id == 28));
    let restored = item_sync::from_network(&wire, &builtin, &Dynamic::default());
    // The narrow adapter adds the official MAX_DAMAGE component only once.
    let supplied = max_damage_adapter::apply(Some(wire.clone()), &source).unwrap();
    assert_eq!(
        supplied
            .added
            .iter()
            .filter(|(id, _)| *id == component::MAX_DAMAGE)
            .count(),
        1
    );
    let supplied_twice = max_damage_adapter::apply(Some(supplied.clone()), &source).unwrap();
    assert_eq!(
        supplied_twice
            .added
            .iter()
            .filter(|(id, _)| *id == component::MAX_DAMAGE)
            .count(),
        1
    );
    let mut bytes = Writer::new();
    Slot::write(&Some(supplied), &mut bytes);
    let received = Slot::read(&mut Reader::new(&bytes.data)).unwrap().unwrap();
    let untouched_patch = received.wire.0.clone();
    let patched = max_damage_adapter::apply(Some(received.clone()), &source).unwrap();
    assert_eq!(patched.wire.0, untouched_patch);
    assert_eq!(patched.added, received.added);
    let mut removed = ItemStack::new("example:tool", 1);
    removed.data_mut().set_other(
        "!minecraft:max_damage",
        Some(Tag::Compound(Default::default())),
    );
    let removed_source = Some(removed);
    let removed_wire = stack_to_network(ids, |_, _| None, &removed_source).unwrap();
    let removed_wire = max_damage_adapter::apply(Some(removed_wire), &removed_source).unwrap();
    assert_eq!(
        removed_wire
            .removed
            .iter()
            .filter(|id| **id == component::MAX_DAMAGE)
            .count(),
        1
    );
    assert!(!removed_wire
        .added
        .iter()
        .any(|(id, _)| *id == component::MAX_DAMAGE));
    let decoded = item_sync::from_network(&received, &builtin, &Dynamic::default());
    assert_eq!(
        decoded.data().and_then(|d| d.other("minecraft:max_damage")),
        Some(&Tag::Int(60))
    );
    println!("MAX_DAMAGE protocol and decoder accept Int(60)");
    // A simple 26.3 Tool stream fixture: zero rules, default speed 1, damage 1,
    // and can-destroy-in-creative. Horizon's decoder retains its raw patch.
    let mut tool_bytes = Writer::new();
    tool_bytes.varint(0);
    tool_bytes.f32(1.0);
    tool_bytes.varint(1);
    tool_bytes.bool(true);
    let mut supplied_tool = wire.clone();
    supplied_tool
        .added
        .push((28, horizon_protocol::item::Component::Raw(tool_bytes.data)));
    let mut bytes = Writer::new();
    Slot::write(&Some(supplied_tool), &mut bytes);
    let received = Slot::read(&mut Reader::new(&bytes.data)).unwrap().unwrap();
    assert!(matches!(
        received.component(28),
        Some(horizon_protocol::item::Component::Raw(_))
    ));
    let decoded = item_sync::from_network(&received, &builtin, &Dynamic::default());
    assert!(decoded
        .data()
        .and_then(|d| d.other("minecraft:tool"))
        .is_none());
    assert!(decoded.wire_patch().is_some());
    println!("TOOL protocol retains raw patch; item decoder does not reconstruct tool");
    for key in ["minecraft:max_damage", "minecraft:tool"] {
        assert!(
            restored.data().and_then(|d| d.other(key)).is_none(),
            "host behavior changed for {key}; reassess the workaround"
        );
    }
    println!("Observed host gap: max_damage and tool absent after encoding");
    let plain = ItemStack::new("minecraft:stick", 2);
    let wire = stack_to_network(ids, |_, _| None, &Some(plain)).unwrap();
    assert!(wire.added.is_empty());
    println!("Pinned Horizon component transport fixture passed");
    if let Some(path) = std::env::args().nth(1) {
        let input: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        for case in input["cases"].as_array().unwrap() {
            let id = case["id"].as_str().unwrap();
            let original =
                horizon_client_lib::item_data::parse_stack(case["snbt"].as_str().unwrap())
                    .unwrap()
                    .unwrap();
            let case_builtin = Builtin {
                registries: [
                    (
                        "minecraft:item".into(),
                        Ids::from_names(vec!["minecraft:air".into(), original.item.to_string()]),
                    ),
                    (
                        "minecraft:data_component_type".into(),
                        builtin
                            .get("minecraft:data_component_type")
                            .unwrap()
                            .clone(),
                    ),
                ]
                .into(),
            };
            let wire = stack_to_network(|_| 1, |_, _| None, &Some(original.clone())).unwrap();
            for id in [
                component::CUSTOM_DATA,
                component::LORE,
                component::ITEM_MODEL,
            ] {
                assert!(wire.added.iter().filter(|(key, _)| *key == id).count() <= 1);
            }
            let mut writer = Writer::new();
            Slot::write(&Some(wire.clone()), &mut writer);
            let received = Slot::read(&mut Reader::new(&writer.data)).unwrap().unwrap();
            let back = item_sync::from_network(&received, &case_builtin, &Dynamic::default());
            assert_eq!(
                (&original.item, original.count, original.damage),
                (&back.item, back.count, back.damage)
            );
            for key in [
                "minecraft:custom_data",
                "minecraft:lore",
                "minecraft:item_model",
            ] {
                assert_eq!(
                    original.data().and_then(|d| d.other(key)),
                    back.data().and_then(|d| d.other(key)),
                    "{id}: {key}"
                );
            }
            let patched = max_damage_adapter::apply(Some(wire), &Some(original.clone())).unwrap();
            let mut patch_bytes = Writer::new();
            Slot::write(&Some(patched), &mut patch_bytes);
            let patched_wire = Slot::read(&mut Reader::new(&patch_bytes.data))
                .unwrap()
                .unwrap();
            let patched_back =
                item_sync::from_network(&patched_wire, &case_builtin, &Dynamic::default());
            assert_eq!(
                original
                    .data()
                    .and_then(|d| d.other("minecraft:max_damage")),
                patched_back
                    .data()
                    .and_then(|d| d.other("minecraft:max_damage")),
                "{id}: max_damage"
            );
            if original
                .data()
                .and_then(|d| d.other("minecraft:tool"))
                .is_some()
            {
                assert!(
                    patched_back
                        .data()
                        .and_then(|d| d.other("minecraft:tool"))
                        .is_none(),
                    "{id}: tool gap has changed"
                );
                println!("{id}: max_damage restored, actual tool component still missing");
            }
            let equal = original.same_item_same_components(&back) && original.count == back.count;
            let before = original
                .data()
                .map(|d| d.other.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            let after = back
                .data()
                .map(|d| d.other.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            println!("{id}: equal={equal}, original components={before:?}, restored components={after:?}");
        }
    }
}
