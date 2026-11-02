//! Positive fixture: run against a Horizon client library with PlutoLib's
//! transport hooks already applied, not against a pristine host.
use horizon_client_lib::{item_sync, items::ItemStack, lan::guest::stack_to_network};
use horizon_protocol::{
    buf::{Codec, Reader, Writer},
    item::{component, Slot},
    registry::{Builtin, Dynamic, Ids},
};
use horizon_world::nbt::Tag;

fn main() {
    let mut components = vec![String::new(); 128];
    components[component::DAMAGE as usize] = "minecraft:damage".into();
    components[component::MAX_DAMAGE as usize] = "minecraft:max_damage".into();
    components[28] = "minecraft:tool".into();
    let mut blocks = vec![String::new(); 8];
    blocks[7] = "minecraft:stone".into();
    let builtin = Builtin {
        registries: [
            (
                "minecraft:item".into(),
                Ids::from_names(vec!["minecraft:air".into(), "example:tool".into()]),
            ),
            (
                "minecraft:data_component_type".into(),
                Ids::from_names(components),
            ),
            ("minecraft:block".into(), Ids::from_names(blocks)),
        ]
        .into(),
    };
    let item_id = |name: &str| {
        builtin
            .get("minecraft:item")
            .and_then(|r| r.id(name))
            .unwrap_or(0)
    };
    let block_id = |registry: &str, name: &str| builtin.get(registry).and_then(|r| r.id(name));
    let mut tool = ItemStack::new("example:tool", 3);
    tool.damage = 4;
    tool.data_mut()
        .set_other("minecraft:max_damage", Some(Tag::Int(60)));
    tool.data_mut().set_other(
        "minecraft:tool",
        Some(Tag::compound([
            (
                "rules",
                Tag::List(vec![Tag::compound([
                    (
                        "blocks",
                        Tag::List(vec![Tag::String("minecraft:stone".into())]),
                    ),
                    ("speed", Tag::Float(2f32.to_bits())),
                    ("correct_for_drops", Tag::Byte(1)),
                ])]),
            ),
            ("default_mining_speed", Tag::Float(1f32.to_bits())),
            ("damage_per_block", Tag::Int(1)),
            ("can_destroy_blocks_in_creative", Tag::Byte(1)),
        ])),
    );
    let expected_tool = tool
        .data()
        .unwrap()
        .other("minecraft:tool")
        .unwrap()
        .clone();
    let wire = stack_to_network(item_id, block_id, &Some(tool)).unwrap();
    for id in [component::MAX_DAMAGE, 28] {
        assert_eq!(wire.added.iter().filter(|(key, _)| *key == id).count(), 1);
    }
    let mut writer = Writer::new();
    Slot::write(&Some(wire), &mut writer);
    let wire = Slot::read(&mut Reader::new(&writer.data)).unwrap().unwrap();
    let decoded = item_sync::from_network(&wire, &builtin, &Dynamic::default());
    assert_eq!(decoded.count, 3);
    assert_eq!(decoded.damage, 4);
    assert_eq!(
        decoded.data().unwrap().other("minecraft:max_damage"),
        Some(&Tag::Int(60))
    );
    assert_eq!(
        decoded.data().unwrap().other("minecraft:tool"),
        Some(&expected_tool)
    );
    println!("Applied host preserves Tool and max-damage components exactly once");
}
