//! Run as an example of the pinned Horizon client library; see docs/component-transport.md.
//! The registry and wire round trip follow Horizon's item_sync tests.
use horizon_client_lib::{item_sync, items::ItemStack, lan::guest::stack_to_network};
use horizon_protocol::{
    buf::{Codec, Reader, Writer},
    item::{component, DelimitedSlot, Slot},
    registry::{Builtin, Dynamic, Ids},
};
use horizon_world::nbt::Tag;

#[path = "../mixins/pluto_transport_3a/tool_codec.rs"]
mod pluto_tool_codec;

mod item_data {
    pub use horizon_client_lib::item_data::*;
}
mod tool_encoder {
    use horizon_client_lib::items::ItemStack;
    use horizon_protocol::item::Stack;
    include!("../mixins/pluto_transport_3a/tool_encode.rs");
    pub fn apply(
        value: Option<Stack>,
        source: &Option<ItemStack>,
        registry: &impl Fn(&str, &str) -> Option<i32>,
    ) -> Option<Stack> {
        pluto_tool_encode(value, source, registry)
    }
}
mod tool_decoder {
    use horizon_client_lib::items::ItemStack;
    use horizon_protocol::{
        item::Stack,
        registry::{Builtin, Dynamic},
    };
    include!("../mixins/pluto_transport_3a/tool_decode.rs");
    pub fn apply(
        value: ItemStack,
        source: &Stack,
        builtin: &Builtin,
        synced: &Dynamic,
    ) -> ItemStack {
        pluto_tool_decode(value, source, builtin, synced)
    }
}

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
    let direct_tool = Tag::compound([(
        "rules",
        Tag::List(vec![Tag::compound([
            (
                "blocks",
                Tag::List(vec![Tag::String("minecraft:stone".into())]),
            ),
            ("speed", Tag::Float(2f32.to_bits())),
        ])]),
    )]);
    let direct_bytes = pluto_tool_codec::encode(&direct_tool, &|registry, name| {
        (registry == "minecraft:block" && name == "minecraft:stone").then_some(7)
    })
    .unwrap();
    let direct_back = pluto_tool_codec::decode(&direct_bytes, &|id| {
        (id == 7).then_some("minecraft:stone".into())
    })
    .unwrap();
    assert_eq!(direct_back.get("rules"), direct_tool.get("rules"));
    assert!(pluto_tool_codec::encode(&direct_tool, &|_, _| None).is_none());
    // All standard Tool fields, non-float numeric inputs, and optional false values.
    let all_fields = Tag::compound([
        (
            "rules",
            Tag::List(vec![Tag::compound([
                ("blocks", Tag::String("#example:mineable".into())),
                ("speed", Tag::Int(2)),
                ("correct_for_drops", Tag::Byte(0)),
            ])]),
        ),
        ("default_mining_speed", Tag::Double(1.5f64.to_bits())),
        ("damage_per_block", Tag::Short(2)),
        ("can_destroy_blocks_in_creative", Tag::Byte(0)),
    ]);
    let bytes = pluto_tool_codec::encode(&all_fields, &|_, _| None).unwrap();
    let decoded = pluto_tool_codec::decode(&bytes, &|_| None).unwrap();
    assert_eq!(
        decoded.get("can_destroy_blocks_in_creative"),
        Some(&Tag::Byte(0))
    );
    assert_eq!(decoded.get("damage_per_block"), Some(&Tag::Int(2)));
    assert_eq!(
        decoded.get("default_mining_speed"),
        Some(&Tag::Float(1.5f32.to_bits()))
    );
    let Some(Tag::List(rules)) = decoded.get("rules") else {
        panic!("rules")
    };
    assert_eq!(rules[0].get("correct_for_drops"), Some(&Tag::Byte(0)));
    assert_eq!(rules[0].get("speed"), Some(&Tag::Float(2f32.to_bits())));
    let mut unresolved = ItemStack::new("example:tool", 1);
    unresolved
        .data_mut()
        .set_other("minecraft:tool", Some(direct_tool));
    let unresolved = Some(unresolved);
    let host = stack_to_network(ids, |_, _| None, &unresolved).unwrap();
    let failed = tool_encoder::apply(Some(host), &unresolved, &|_, _| None).unwrap();
    assert!(
        failed.component(28).is_none(),
        "unresolved block IDs must not invent a value"
    );
    // Canonicalization makes wire-equivalent saved forms share stacking and search identity.
    let one = Tag::compound([(
        "rules",
        Tag::List(vec![Tag::compound([(
            "blocks",
            Tag::String("minecraft:stone".into()),
        )])]),
    )]);
    let list = Tag::compound([
        (
            "rules",
            Tag::List(vec![Tag::compound([(
                "blocks",
                Tag::List(vec![Tag::String("minecraft:stone".into())]),
            )])]),
        ),
        ("default_mining_speed", Tag::Float(1f32.to_bits())),
        ("damage_per_block", Tag::Int(1)),
        ("can_destroy_blocks_in_creative", Tag::Byte(1)),
    ]);
    let resolve = |registry: &str, name: &str| {
        (registry == "minecraft:block" && name == "minecraft:stone").then_some(7)
    };
    let canonical_one = pluto_tool_codec::canonicalize(&one, &resolve).unwrap();
    let canonical_list = pluto_tool_codec::canonicalize(&list, &resolve).unwrap();
    assert_eq!(canonical_one, canonical_list);
    let mut first = ItemStack::new("example:tool", 1);
    first
        .data_mut()
        .set_other("minecraft:tool", Some(canonical_one));
    let mut second = ItemStack::new("example:tool", 1);
    second
        .data_mut()
        .set_other("minecraft:tool", Some(canonical_list));
    let mut uncanonical = ItemStack::new("example:tool", 1);
    uncanonical
        .data_mut()
        .set_other("minecraft:tool", Some(one.clone()));
    assert!(!uncanonical.same_item_same_components(&first));
    assert!(first.same_item_same_components(&second));
    assert_eq!(
        horizon_client_lib::item_data::canon_stack(Some(&first)),
        horizon_client_lib::item_data::canon_stack(Some(&second))
    );
    let tag_tool = Tag::compound([(
        "rules",
        Tag::List(vec![Tag::compound([(
            "blocks",
            Tag::String("#minecraft:stone".into()),
        )])]),
    )]);
    assert_ne!(
        pluto_tool_codec::canonicalize(&tag_tool, &resolve),
        pluto_tool_codec::canonicalize(&one, &resolve)
    );
    // The public SNBT entry validates once and returns the canonical value to store.
    let single = r#"{rules:[{blocks:"minecraft:stone"}]}"#;
    let canonical_snbt =
        pluto_tool_codec::prepare_tool(single, |name| (name == "minecraft:stone").then_some(7))
            .unwrap();
    let explicit = r#"{rules:[{blocks:["minecraft:stone"]}],default_mining_speed:1.0f,damage_per_block:1,can_destroy_blocks_in_creative:1b}"#;
    assert_eq!(
        canonical_snbt,
        pluto_tool_codec::prepare_tool(explicit, |name| (name == "minecraft:stone").then_some(7))
            .unwrap()
    );
    assert_eq!(
        pluto_tool_codec::prepare_tool(single, |_| None),
        Err(pluto_tool_codec::ToolError::UnknownBlock(
            "minecraft:stone".into()
        ))
    );
    assert!(matches!(
        pluto_tool_codec::prepare_tool("{rules:[]} trailing", |_| None),
        Err(pluto_tool_codec::ToolError::InvalidSnbt)
    ));
    assert!(matches!(
        pluto_tool_codec::prepare_tool("{rules:[],unknown:1}", |_| None),
        Err(pluto_tool_codec::ToolError::InvalidOrUnsupported)
    ));
    assert!(matches!(
        pluto_tool_codec::prepare_tool(r##"{rules:[{blocks:"#Bad Tag"}]}"##, |_| None),
        Err(pluto_tool_codec::ToolError::InvalidOrUnsupported)
    ));
    let bare = r#"{rules:[{blocks:"stone"}]}"#;
    let qualified = r#"{rules:[{blocks:"minecraft:stone"}]}"#;
    let resolve = |name: &str| (name == "minecraft:stone").then_some(7);
    let bare = pluto_tool_codec::prepare_tool(bare, resolve).unwrap();
    let qualified = pluto_tool_codec::prepare_tool(qualified, resolve).unwrap();
    assert_eq!(bare, qualified);
    let (bare_tag, _) = horizon_world::nbt::parse_snbt(&bare).unwrap();
    let (qualified_tag, _) = horizon_world::nbt::parse_snbt(&qualified).unwrap();
    let mut bare_stack = ItemStack::new("example:tool", 1);
    bare_stack
        .data_mut()
        .set_other("minecraft:tool", Some(bare_tag));
    let mut qualified_stack = ItemStack::new("example:tool", 1);
    qualified_stack
        .data_mut()
        .set_other("minecraft:tool", Some(qualified_tag));
    assert!(bare_stack.same_item_same_components(&qualified_stack));
    assert_eq!(
        horizon_client_lib::item_data::canon_stack(Some(&bare_stack)),
        horizon_client_lib::item_data::canon_stack(Some(&qualified_stack))
    );
    let tag_bare = r##"{rules:[{blocks:"#mineable/pickaxe"}]}"##;
    let tag_qualified = r##"{rules:[{blocks:"#minecraft:mineable/pickaxe"}]}"##;
    assert_eq!(
        pluto_tool_codec::prepare_tool(tag_bare, |_| None),
        pluto_tool_codec::prepare_tool(tag_qualified, |_| None)
    );
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
            let patched =
                max_damage_adapter::apply(Some(wire.clone()), &Some(original.clone())).unwrap();
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
            if let Some(original_tool) = original.data().and_then(|d| d.other("minecraft:tool")) {
                let prepared = pluto_tool_codec::prepare_tool(
                    &horizon_client_lib::item_data::canon_tag(original_tool),
                    |_| None,
                )
                .expect("consumer preparation");
                let (prepared_tag, _) = horizon_world::nbt::parse_snbt(&prepared).unwrap();
                let mut prepared_stack = original.clone();
                prepared_stack
                    .data_mut()
                    .set_other("minecraft:tool", Some(prepared_tag));
                let transport =
                    stack_to_network(|_| 1, |_, _| None, &Some(prepared_stack.clone())).unwrap();
                let transport =
                    max_damage_adapter::apply(Some(transport), &Some(prepared_stack.clone()))
                        .unwrap();
                let transport =
                    tool_encoder::apply(Some(transport), &Some(prepared_stack.clone()), &|_, _| {
                        None
                    })
                    .unwrap();
                let mut bytes = Writer::new();
                Slot::write(&Some(transport), &mut bytes);
                let received = Slot::read(&mut Reader::new(&bytes.data)).unwrap().unwrap();
                let restored =
                    item_sync::from_network(&received, &case_builtin, &Dynamic::default());
                let restored =
                    tool_decoder::apply(restored, &received, &case_builtin, &Dynamic::default());
                assert_eq!(
                    prepared_stack
                        .data()
                        .and_then(|d| d.other("minecraft:tool")),
                    restored.data().and_then(|d| d.other("minecraft:tool"))
                );
                assert_eq!(
                    prepared_stack
                        .data()
                        .and_then(|d| d.other("minecraft:max_damage")),
                    restored
                        .data()
                        .and_then(|d| d.other("minecraft:max_damage"))
                );
                assert!(
                    prepared_stack.same_item_same_components(&restored),
                    "{id}: prepared identity"
                );
                assert_eq!(
                    horizon_client_lib::item_data::canon_stack(Some(&prepared_stack)),
                    horizon_client_lib::item_data::canon_stack(Some(&restored))
                );
                let encoded =
                    pluto_tool_codec::encode(original_tool, &|_, _| None).expect("tool encode");
                let mut official = wire.clone();
                official
                    .added
                    .push((28, horizon_protocol::item::Component::Raw(encoded.clone())));
                let mut raw = Writer::new();
                Slot::write(&Some(official), &mut raw);
                let read = Slot::read(&mut Reader::new(&raw.data)).unwrap().unwrap();
                assert!(
                    matches!(read.component(28), Some(horizon_protocol::item::Component::Raw(v)) if *v == encoded)
                );
                let decoded_tool =
                    pluto_tool_codec::decode(&encoded, &|_| None).expect("tool decode");
                assert_eq!(original_tool.get("rules"), decoded_tool.get("rules"));
                assert_eq!(
                    original_tool.get("default_mining_speed"),
                    decoded_tool.get("default_mining_speed")
                );
                assert_eq!(
                    original_tool.get("damage_per_block"),
                    decoded_tool.get("damage_per_block")
                );
                let all =
                    max_damage_adapter::apply(Some(wire.clone()), &Some(original.clone())).unwrap();
                let all =
                    tool_encoder::apply(Some(all), &Some(original.clone()), &|_, _| None).unwrap();
                let all =
                    tool_encoder::apply(Some(all), &Some(original.clone()), &|_, _| None).unwrap();
                assert_eq!(all.added.iter().filter(|(id, _)| *id == 28).count(), 1);
                let mut all = all;
                all.added
                    .push((component::RARITY, horizon_protocol::item::Component::Int(2)));
                all.removed.push(component::ITEM_NAME);
                let mut guest_wire = Writer::new();
                DelimitedSlot::write(&Some(all.clone()), &mut guest_wire);
                let guest_read = DelimitedSlot::read(&mut Reader::new(&guest_wire.data))
                    .unwrap()
                    .unwrap();
                assert!(guest_read.component(28).is_some());
                assert!(guest_read.component(component::MAX_DAMAGE).is_some());
                assert!(guest_read.component(component::RARITY).is_some());
                assert!(guest_read.removed.contains(&component::ITEM_NAME));
                let mut encoded_wire = Writer::new();
                Slot::write(&Some(all), &mut encoded_wire);
                let received = Slot::read(&mut Reader::new(&encoded_wire.data))
                    .unwrap()
                    .unwrap();
                assert!(received.component(component::RARITY).is_some());
                assert!(received.removed.contains(&component::ITEM_NAME));
                let raw_patch = received.wire.0.clone().unwrap();
                let decoded =
                    item_sync::from_network(&received, &case_builtin, &Dynamic::default());
                let mut decoded =
                    tool_decoder::apply(decoded, &received, &case_builtin, &Dynamic::default());
                let tool = decoded.data().unwrap().other("minecraft:tool").unwrap();
                assert_eq!(tool.get("rules"), original_tool.get("rules"));
                assert_eq!(
                    tool.get("default_mining_speed"),
                    original_tool.get("default_mining_speed")
                );
                assert_eq!(
                    tool.get("damage_per_block"),
                    original_tool.get("damage_per_block")
                );
                assert_eq!(decoded.wire_patch().as_deref(), Some(raw_patch.as_ref()));
                decoded.count += 1;
                assert_eq!(decoded.wire_patch().as_deref(), Some(raw_patch.as_ref()));
                decoded.damage += 1;
                assert!(
                    decoded.wire_patch().is_none(),
                    "a changed component invalidates the cached patch"
                );
                let changed = Some(decoded);
                let fallback = stack_to_network(|_| 1, |_, _| None, &changed).unwrap();
                // The host's normal re-encoder cannot reconstruct unrelated entries
                // absent from its item model after a changed cached patch.
                assert!(fallback.component(component::RARITY).is_none());
                assert!(!fallback.removed.contains(&component::ITEM_NAME));
                let repaired = max_damage_adapter::apply(Some(fallback), &changed).unwrap();
                let repaired = tool_encoder::apply(Some(repaired), &changed, &|_, _| None).unwrap();
                let mut changed_bytes = Writer::new();
                DelimitedSlot::write(&Some(repaired), &mut changed_bytes);
                let changed_wire = DelimitedSlot::read(&mut Reader::new(&changed_bytes.data))
                    .unwrap()
                    .unwrap();
                let changed_back =
                    item_sync::from_network(&changed_wire, &case_builtin, &Dynamic::default());
                let changed_back = tool_decoder::apply(
                    changed_back,
                    &changed_wire,
                    &case_builtin,
                    &Dynamic::default(),
                );
                let expected = changed.as_ref().unwrap();
                assert_eq!(changed_back.damage, expected.damage);
                for key in [
                    "minecraft:tool",
                    "minecraft:max_damage",
                    "minecraft:custom_data",
                    "minecraft:lore",
                    "minecraft:item_model",
                ] {
                    assert_eq!(
                        changed_back.data().and_then(|d| d.other(key)),
                        expected.data().and_then(|d| d.other(key)),
                        "{id}: changed {key}"
                    );
                }

                println!("{id}: standard tool bytes and actual mining rules round-trip");
            }
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
