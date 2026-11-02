use pluto_lib::{
    add_creative_tabs, add_paged_creative_tabs, CreativeTab, PagedCreativeTab, Registry,
};
use std::{env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 && !(args.len() == 4 && args[3] == "--overflow") {
        return Err("usage: demo OUTPUT_DIR PREPARED_CREATIVE_JSON [--overflow]".into());
    }
    let root = Path::new(&args[1]);
    let inventory = serde_json::from_slice(&fs::read(&args[2])?)?;
    let gem = "{count:1,id:\"pluto_demo:gem\"}";
    let block = "{count:1,id:\"pluto_demo:glowing_block\"}";
    let tabs = if args.len() == 4 {
        let ids: Vec<_> = (1..=17).map(|n| format!("pluto_demo:page_{n}")).collect();
        let titles: Vec<_> = (1..=17).map(|n| format!("Pluto Demo {n}")).collect();
        let groups: Vec<_> = ids
            .iter()
            .zip(&titles)
            .enumerate()
            .map(|(i, (id, title))| PagedCreativeTab {
                id,
                title,
                icon: if i % 2 == 0 { gem } else { block },
                items: if i % 2 == 0 { vec![gem] } else { vec![block] },
            })
            .collect();
        add_paged_creative_tabs(&inventory, &groups)?
    } else {
        add_creative_tabs(
            &inventory,
            &[CreativeTab {
                id: "pluto_demo:things",
                title: "Pluto Demo",
                icon: gem,
                row: "TOP",
                column: 5,
                items: vec![gem, block],
            }],
        )?
    };
    let mut registry = Registry::new("pluto_demo")?;
    registry.item_with_max_stack("gem", "minecraft:amethyst_shard", 16)?;
    registry.block("glowing_block", "minecraft:amethyst_block")?;
    registry.write_to(root)?;
    fs::write(
        root.join("creative-inventory.json"),
        serde_json::to_vec_pretty(&tabs)?,
    )?;
    fs::write(root.join("horizon-mod.toml"), "id = \"pluto_demo\"\nname = \"Pluto Demo\"\nversion = \"0.1.0\"\napi = \"0.2\"\nside = \"both\"\ncapabilities = [\"registries\"]\n")?;
    for (path, value) in [
        (
            "assets/pluto_demo/items/gem.json",
            serde_json::json!({"model":{"type":"minecraft:model","model":"pluto_demo:item/gem"}}),
        ),
        (
            "assets/pluto_demo/models/item/gem.json",
            serde_json::json!({"parent":"minecraft:item/generated","textures":{"layer0":"minecraft:item/amethyst_shard"}}),
        ),
        (
            "assets/pluto_demo/items/glowing_block.json",
            serde_json::json!({"model":{"type":"minecraft:model","model":"pluto_demo:block/glowing_block"}}),
        ),
        (
            "assets/pluto_demo/blockstates/glowing_block.json",
            serde_json::json!({"variants":{"":{"model":"pluto_demo:block/glowing_block"}}}),
        ),
        (
            "assets/pluto_demo/models/block/glowing_block.json",
            serde_json::json!({"parent":"minecraft:block/cube_all","textures":{"all":"minecraft:block/amethyst_block"}}),
        ),
        (
            "assets/pluto_demo/lang/en_us.json",
            serde_json::json!({"item.pluto_demo.gem":"Pluto Gem", "block.pluto_demo.glowing_block":"Pluto Block"}),
        ),
    ] {
        let file = root.join(path);
        fs::create_dir_all(file.parent().unwrap())?;
        fs::write(file, serde_json::to_vec_pretty(&value)?)?;
    }
    println!(
        "Generated content and Creative inventory in {}",
        root.display()
    );
    Ok(())
}
