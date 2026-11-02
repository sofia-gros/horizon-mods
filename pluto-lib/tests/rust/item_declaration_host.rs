//! Check generated demo declarations with the pinned Horizon content parser.
use horizon_mods::{Content, Package};
use std::{env, fs, path::PathBuf};

#[test]
fn generated_item_limit_reaches_horizon_content() {
    let root = PathBuf::from(env::var_os("PLUTO_DEMO_DIR").expect("set PLUTO_DEMO_DIR"));
    let output = PathBuf::from(env::var_os("PLUTO_PROBE_HMOD").expect("set PLUTO_PROBE_HMOD"));
    let manifest = fs::read_to_string(root.join("horizon-mod.toml")).unwrap();
    let item = fs::read(root.join("data/pluto_demo/horizon/item/gem.json")).unwrap();
    let bytes = Package::write_files(
        &manifest,
        None,
        &[("data/pluto_demo/horizon/item/gem.json", &item)],
    )
    .unwrap();
    fs::write(&output, bytes).unwrap();
    let package = Package::read(&output).unwrap();
    let content = Content::read(&output, &package.manifest).unwrap();
    assert_eq!(content.items.len(), 1);
    assert_eq!(content.items[0].id, "pluto_demo:gem");
    assert_eq!(content.items[0].template, "minecraft:amethyst_shard");
    assert_eq!(content.items[0].max_stack_size, Some(16));
}
