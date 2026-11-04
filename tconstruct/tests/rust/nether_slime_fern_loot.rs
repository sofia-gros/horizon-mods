use horizon_core::random::LegacyRandom;
use horizon_items::items::Items;
use horizon_world::{loot::PackOverlay, registry::{BlockRegistry, BlockStateId}};
use std::{fs, path::PathBuf};

#[test]
fn nether_ferns_drop_without_shears_other_ferns_do_not() {
    let root = PathBuf::from(std::env::var("FERN_FIXTURE_ROOT").unwrap());
    let temp = std::env::temp_dir().join(format!("fern-loot-{}", std::process::id()));
    fs::create_dir_all(&temp).unwrap();
    fs::write(temp.join("items.json"), "{}").unwrap();
    let items = Items::catalog(&temp, &PackOverlay::NONE).unwrap();
    for color in ["earth", "sky", "ender", "ichor", "blood"] {
        for kind in ["fern", "tall_grass"] {
            let name = format!("{color}_slime_{kind}");
            let json: serde_json::Value = serde_json::from_slice(&fs::read(root.join(format!("content/data/tconstruct/loot_table/blocks/{name}.json"))).unwrap()).unwrap();
            let got = items.inline_loot_table(&json, &mut LegacyRandom::new(42));
            if (color == "blood" || color == "ichor") && kind == "fern" {
                assert_eq!(got.len(), 1, "{name}");
                assert_eq!(got[0].item.as_ref(), format!("tconstruct:{name}"));
                assert_eq!(got[0].count, 1);
            } else { assert!(got.is_empty(), "{name}"); }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn nether_fern_explosion_condition_is_evaluated_by_host() {
    let root = PathBuf::from(std::env::var("FERN_FIXTURE_ROOT").unwrap());
    let temp = std::env::temp_dir().join(format!("fern-explosion-{}", std::process::id()));
    let loot = temp.join("data/tconstruct/loot_table/blocks");
    fs::create_dir_all(&loot).unwrap();
    fs::write(temp.join("items.json"), "{}").unwrap();
    for name in ["blood_slime_fern", "ichor_slime_fern"] {
        fs::copy(root.join(format!("content/data/tconstruct/loot_table/blocks/{name}.json")), loot.join(format!("{name}.json"))).unwrap();
    }
    let facts = br#"{"defaults":{"minecraft:air":0,"tconstruct:blood_slime_fern":1,"tconstruct:ichor_slime_fern":2},"states":[["air",true,false,"","minecraft:air"],["blood",false,false,"","tconstruct:blood_slime_fern"],["ichor",false,false,"","tconstruct:ichor_slime_fern"]]}"#;
    let registry = BlockRegistry::from_facts_json(&facts[..]).unwrap();
    let items = Items::catalog(&temp, &PackOverlay::NONE).unwrap();
    for (id, name) in [(1, "blood_slime_fern"), (2, "ichor_slime_fern")] {
        let mut survived = 0;
        for seed in 0..100 {
            let drops = items.block_drops_exploded(&registry, BlockStateId(id), None, &[], &mut LegacyRandom::new(seed * 1_000_003), None, Some(4.0), false);
            if !drops.is_empty() { assert_eq!(drops[0].item.as_ref(), format!("tconstruct:{name}")); survived += 1; }
        }
        assert!(survived > 0 && survived < 100, "{name}: {survived}");
    }
    fs::remove_dir_all(temp).unwrap();
}
