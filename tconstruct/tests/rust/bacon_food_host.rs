#[cfg(test)]
use horizon_items::items::Items;
#[cfg(test)]
use horizon_world::loot::PackOverlay;
#[cfg(test)]
use std::{fs, path::PathBuf};

#[test]
fn host_catalog_reads_bacon_saturation_as_points() {
    let source = PathBuf::from(std::env::var_os("BACON_MOD_ROOT").expect("BACON_MOD_ROOT"));
    let native: serde_json::Value = serde_json::from_slice(&fs::read(source.join("content/data/tconstruct/horizon/item/bacon.json")).unwrap()).unwrap();
    let dir = PathBuf::from(std::env::var_os("BACON_FIXTURE_ROOT").expect("BACON_FIXTURE_ROOT"));
    fs::write(dir.join("items.json"), serde_json::to_vec(&serde_json::json!({"tconstruct:bacon":native["components"]})).unwrap()).unwrap();
    let items = Items::catalog(&dir, &PackOverlay::NONE).unwrap();
    let info = items.items.get("tconstruct:bacon").unwrap();
    assert_eq!(info.food, Some((4, 4.8_f32)));
    let source_points = 4_f32 * 0.6_f32 * 2_f32;
    assert_eq!(info.food.unwrap().1.to_bits(), source_points.to_bits());
}

#[test]
fn host_catalog_reads_cheese_saturation_as_points() {
    let source = PathBuf::from(std::env::var_os("BACON_MOD_ROOT").expect("BACON_MOD_ROOT"));
    let native: serde_json::Value = serde_json::from_slice(&fs::read(source.join("content/data/tconstruct/horizon/item/cheese_ingot.json")).unwrap()).unwrap();
    let dir = PathBuf::from(std::env::var_os("BACON_FIXTURE_ROOT").expect("BACON_FIXTURE_ROOT")).join("cheese");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("items.json"), serde_json::to_vec(&serde_json::json!({"tconstruct:cheese_ingot":native["components"]})).unwrap()).unwrap();
    let items = Items::catalog(&dir, &PackOverlay::NONE).unwrap();
    let info = items.items.get("tconstruct:cheese_ingot").unwrap();
    assert_eq!(info.food, Some((3, 2.4_f32)));
    let source_points = 3_f32 * 0.4_f32 * 2_f32;
    assert_eq!(info.food.unwrap().1.to_bits(), source_points.to_bits());
}
