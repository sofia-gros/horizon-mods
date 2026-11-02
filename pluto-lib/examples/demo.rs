use pluto_lib::native_creative::{register_tabs, Entry, Tab};
use pluto_lib::{
    add_creative_tabs, add_paged_creative_tabs, BlockPresentation, BlockProperties, CreativeTab,
    PagedCreativeTab, Registry,
};
use std::{env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3
        && !(args.len() == 4 && matches!(args[3].as_str(), "--overflow" | "--native-pages"))
    {
        return Err(
            "usage: demo OUTPUT_DIR PREPARED_CREATIVE_JSON [--overflow|--native-pages]".into(),
        );
    }
    let root = Path::new(&args[1]);
    let inventory = serde_json::from_slice(&fs::read(&args[2])?)?;
    let gem = "{count:1,id:\"pluto_demo:gem\"}";
    let inherited = "{count:1,id:\"pluto_demo:gem_inherited\"}";
    let block = "{count:1,id:\"pluto_demo:glowing_block\"}";
    let free_block = "{count:1,id:\"pluto_demo:free_block\"}";
    let inherited_block = "{count:1,id:\"pluto_demo:inherited_block\"}";
    let unbreakable = "{count:1,id:\"pluto_demo:unbreakable_block\"}";
    let tabs = if args.get(3).is_some_and(|arg| arg == "--overflow") {
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
    } else if args.get(3).is_some_and(|arg| arg == "--native-pages") {
        let items = [
            gem,
            inherited,
            block,
            free_block,
            inherited_block,
            unbreakable,
        ]
        .into_iter()
        .map(|snbt| Entry {
            snbt: snbt.into(),
            searchable: true,
            permissions_only: false,
            search_lines: vec!["Pluto Demo".into()],
            search_tags: Vec::new(),
        })
        .collect();
        register_tabs(
            &inventory,
            &[Tab {
                id: "pluto_demo:things".into(),
                title: "Pluto Demo".into(),
                icon: gem.into(),
                items,
            }],
            |s| {
                let mut item = horizon_items::item_data::parse_stack(s)?.ok_or("empty stack")?;
                item.count = 1;
                Ok(horizon_items::item_data::canon_stack(Some(&item)))
            },
        )?
    } else {
        add_creative_tabs(
            &inventory,
            &[CreativeTab {
                id: "pluto_demo:things",
                title: "Pluto Demo",
                icon: gem,
                row: "TOP",
                column: 5,
                items: vec![
                    gem,
                    inherited,
                    block,
                    free_block,
                    inherited_block,
                    unbreakable,
                ],
            }],
        )?
    };
    let mut registry = Registry::new("pluto_demo")?;
    registry.item_with_max_stack("gem", "minecraft:amethyst_shard", 16)?;
    registry.item("gem_inherited", "minecraft:amethyst_shard")?;
    registry.block_with_settings(
        "glowing_block",
        "minecraft:amethyst_block",
        BlockProperties {
            hardness: Some(2.0),
            explosion_resistance: Some(1200.0),
            requires_tool: Some(true),
        },
        BlockPresentation {
            light: Some(14),
            sounds: Some("minecraft:anvil"),
        },
    )?;
    registry.block_with_settings(
        "free_block",
        "minecraft:amethyst_block",
        BlockProperties {
            hardness: Some(2.0),
            explosion_resistance: Some(0.0),
            requires_tool: Some(false),
        },
        BlockPresentation {
            light: Some(0),
            sounds: None,
        },
    )?;
    registry.block_with_properties(
        "blast_low",
        "minecraft:amethyst_block",
        BlockProperties {
            hardness: Some(2.0),
            explosion_resistance: Some(0.0),
            requires_tool: Some(true),
        },
    )?;
    registry.block("inherited_block", "minecraft:amethyst_block")?;
    registry.block_with_properties(
        "unbreakable_block",
        "minecraft:amethyst_block",
        BlockProperties {
            hardness: Some(-1.0),
            ..Default::default()
        },
    )?;
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
            "assets/pluto_demo/items/gem_inherited.json",
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
            serde_json::json!({"item.pluto_demo.gem":"Pluto Gem", "item.pluto_demo.gem_inherited":"Template Gem", "block.pluto_demo.glowing_block":"Pluto Block", "block.pluto_demo.free_block":"Free Block", "block.pluto_demo.inherited_block":"Template Block", "block.pluto_demo.unbreakable_block":"Unbreakable Block"}),
        ),
    ] {
        let file = root.join(path);
        fs::create_dir_all(file.parent().unwrap())?;
        fs::write(file, serde_json::to_vec_pretty(&value)?)?;
    }
    // The demo supplies its own data assets; Registry does not create tags or loot.
    let tag = root.join("data/minecraft/tags/block/mineable/pickaxe.json");
    fs::create_dir_all(tag.parent().unwrap())?;
    fs::write(
        tag,
        serde_json::to_vec_pretty(
            &serde_json::json!({"replace":false,"values":["pluto_demo:glowing_block","pluto_demo:free_block","pluto_demo:inherited_block","pluto_demo:unbreakable_block","pluto_demo:blast_low"]}),
        )?,
    )?;
    for name in [
        "glowing_block",
        "free_block",
        "inherited_block",
        "unbreakable_block",
        "blast_low",
    ] {
        let entries = [
            (
                format!("assets/pluto_demo/items/{name}.json"),
                serde_json::json!({"model":{"type":"minecraft:model","model":"pluto_demo:block/glowing_block"}}),
            ),
            (
                format!("assets/pluto_demo/blockstates/{name}.json"),
                serde_json::json!({"variants":{"":{"model":"pluto_demo:block/glowing_block"}}}),
            ),
            (
                format!("data/pluto_demo/loot_table/blocks/{name}.json"),
                serde_json::json!({"type":"minecraft:block","pools":[{"rolls":1,"entries":[{"type":"minecraft:item","name":format!("pluto_demo:{name}")}]}]}),
            ),
        ];
        for (path, value) in entries {
            let file = root.join(path);
            fs::create_dir_all(file.parent().unwrap())?;
            fs::write(file, serde_json::to_vec_pretty(&value)?)?;
        }
    }
    println!(
        "Generated content and Creative inventory in {}",
        root.display()
    );
    Ok(())
}
