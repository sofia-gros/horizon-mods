use pluto_lib::{add_creative_tabs, CreativeTab, Registry};
use std::{env, fs, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        return Err("usage: wand_demo OUTPUT_DIR PREPARED_CREATIVE_JSON".into());
    }
    let inventory = serde_json::from_slice(&fs::read(&args[2])?)?;
    let wand = "{count:1,id:\"pluto_wand:wand\"}";
    let inventory = add_creative_tabs(
        &inventory,
        &[CreativeTab {
            id: "pluto_wand:items",
            title: "Wand Demo",
            icon: wand,
            row: "TOP",
            column: 5,
            items: vec![wand],
        }],
    )?;
    let root = Path::new(&args[1]);
    let mut registry = Registry::new("pluto_wand")?;
    registry.item("wand", "minecraft:stick")?;
    registry.write_to(root)?;
    fs::write(root.join("horizon-mod.toml"), "id = \"pluto_wand\"\nname = \"Wand Demo\"\nversion = \"0.1.0\"\napi = \"0.2\"\nside = \"both\"\ncapabilities = [\"registries\"]\n")?;
    fs::write(
        root.join("creative-inventory.json"),
        serde_json::to_vec_pretty(&inventory)?,
    )?;
    for (path, content) in [
        (
            "assets/pluto_wand/items/wand.json",
            serde_json::json!({"model":{"type":"minecraft:model","model":"pluto_wand:item/wand"}}),
        ),
        (
            "assets/pluto_wand/models/item/wand.json",
            serde_json::json!({"parent":"minecraft:item/handheld","textures":{"layer0":"minecraft:item/blaze_rod"}}),
        ),
        (
            "assets/pluto_wand/lang/en_us.json",
            serde_json::json!({"item.pluto_wand.wand":"Demo Wand"}),
        ),
    ] {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, serde_json::to_vec_pretty(&content)?)?;
    }
    Ok(())
}
