//! Run as an example of the pinned Horizon client library; see docs/component-transport.md.
//! The registry and wire round trip follow Horizon's item_sync tests.
use horizon_client_lib::{item_sync, items::ItemStack, lan::guest::stack_to_network};
use horizon_protocol::{
    buf::{Codec, Reader, Writer},
    item::{component, Slot},
    registry::{Builtin, Dynamic, Ids},
};
use horizon_world::nbt::Tag;

fn main() {
    let mut names = vec![String::new(); 128];
    for (id, name) in [
        (component::CUSTOM_DATA, "minecraft:custom_data"),
        (component::LORE, "minecraft:lore"),
        (component::ITEM_MODEL, "minecraft:item_model"),
        (component::DAMAGE, "minecraft:damage"),
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
    let wire = stack_to_network(ids, |_, _| None, &Some(tool)).unwrap();
    let restored = item_sync::from_network(&wire, &builtin, &Dynamic::default());
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
            Slot::write(&Some(wire), &mut writer);
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
