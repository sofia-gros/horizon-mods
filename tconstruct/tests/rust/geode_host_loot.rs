//! Limited host loot fixture: not a world, mod-content build, or LAN test.
use horizon_core::random::LegacyRandom;
use horizon_items::items::{Items, ItemStack};
use horizon_world::{loot::PackOverlay, registry::{BlockRegistry, BlockStateId}};
use std::{fs, path::PathBuf};

#[test]
fn cluster_branches_and_overlay_use_host_loot_and_tags() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp = std::env::temp_dir().join(format!("geode-host-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let loot = temp.join("data/tconstruct/loot_table/blocks");
    let tag = temp.join("data/minecraft/tags/item");
    fs::create_dir_all(&loot).unwrap(); fs::create_dir_all(&tag).unwrap();
    fs::write(temp.join("items.json"), "{}").unwrap();
    // A synthetic prepared root: one vanilla tool plus the actual mod additions.
    let path = source.join("content/data/minecraft/tags/item/cluster_max_harvestables.json");
    let mut data: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    data["values"].as_array_mut().unwrap().push("minecraft:diamond_pickaxe".into());
    fs::write(tag.join("cluster_max_harvestables.json"), serde_json::to_vec(&data).unwrap()).unwrap();
    let clusters = ["earth_slime_crystal_cluster", "sky_slime_crystal_cluster", "ender_slime_crystal_cluster", "ichor_slime_crystal_cluster", "cobalt_cluster"];
    let mut states = vec![serde_json::json!(["air",true,false,"","minecraft:air"])];
    let mut defaults = serde_json::Map::new(); defaults.insert("minecraft:air".into(), 0.into());
    for (index, name) in clusters.iter().enumerate() {
        fs::copy(source.join(format!("content/data/tconstruct/loot_table/blocks/{name}.json")), loot.join(format!("{name}.json"))).unwrap();
        defaults.insert(format!("tconstruct:{name}"), (index as u64 + 1).into());
        states.push(serde_json::json!([format!("s{index}"),false,false,"",format!("tconstruct:{name}")]));
    }
    let facts=serde_json::to_vec(&serde_json::json!({"defaults":defaults,"states":states})).unwrap();
    let registry=BlockRegistry::from_facts_json(&facts[..]).unwrap();
    let items=Items::catalog(&temp,&PackOverlay::NONE).unwrap();
    let tinker=ItemStack::new("tconstruct:pickaxe",1);
    let vanilla=ItemStack::new("minecraft:diamond_pickaxe",1);
    let stick=ItemStack::new("minecraft:stick",1);
    let mut silk=vanilla.clone(); silk.enchant("minecraft:silk_touch",1);
    let mut fortune=vanilla.clone(); fortune.enchant("minecraft:fortune",3);
    for (index, name) in clusters.iter().enumerate() {
        let state=BlockStateId(index as u32 + 1);
        let item=if *name=="cobalt_cluster" { "tconstruct:cobalt_shard".to_string() } else {format!("tconstruct:{}", name.replace("_cluster", ""))};
        for (held, expected) in [(Some(&tinker),4),(Some(&vanilla),4),(Some(&stick),2),(None,2)] {
            let d=items.block_drops(&registry,state,held,&mut LegacyRandom::new(123));
            assert_eq!(d.len(),1,"{name}"); assert_eq!(d[0].item.as_ref(),item); assert_eq!(d[0].count,expected);
        }
        let d=items.block_drops(&registry,state,Some(&silk),&mut LegacyRandom::new(123));
        assert_eq!((d[0].item.as_ref(),d[0].count),(format!("tconstruct:{name}").as_str(),1));
        let mut saw_bonus=false;
        for seed in 0..40 { let d=items.block_drops(&registry,state,Some(&fortune),&mut LegacyRandom::new(seed*1_000_003)); assert_eq!(d[0].count%4,0); saw_bonus |= d[0].count > 4; }
        assert!(saw_bonus,"{name}");
    }
    // A world datapack replacement deliberately removes both prepared tools.
    let files=vec![("tags/minecraft:item/cluster_max_harvestables".to_owned(),vec![br#"{"replace":true,"values":["example:tool"]}"#.to_vec()])];
    let replaced=Items::catalog(&temp,&PackOverlay{files:&files}).unwrap();
    assert_eq!(replaced.block_drops(&registry,BlockStateId(1),Some(&tinker),&mut LegacyRandom::new(123))[0].count,2);
    fs::remove_dir_all(temp).unwrap();
}
