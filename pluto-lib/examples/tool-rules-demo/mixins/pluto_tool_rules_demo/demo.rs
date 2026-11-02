use pluto_api::creative::{register_tabs, Entry, Tab};
use std::{error::Error, fs, path::PathBuf};

/// The demo opts in with both paths. It creates a fresh Linux resource overlay.
pub fn prepare() -> Result<(), Box<dyn Error>> {
    let (source, output) = match (
        std::env::var_os("PLUTO_DEMO_RESOURCES"),
        std::env::var_os("PLUTO_DEMO_OVERLAY"),
    ) {
        (None, None) => return Ok(()),
        (Some(source), Some(output)) => (fs::canonicalize(source)?, PathBuf::from(output)),
        _ => return Err("set both PLUTO_DEMO_RESOURCES and PLUTO_DEMO_OVERLAY".into()),
    };
    let name = output.file_name().ok_or("overlay must have a file name")?;
    let parent = fs::canonicalize(output.parent().ok_or("overlay needs a parent directory")?)?;
    let output = parent.join(name);
    if output.exists() || output.starts_with(&source) || source.starts_with(&output) {
        return Err("overlay must be new and separate from source".into());
    }
    let doc = serde_json::from_slice(&fs::read(source.join("creative-inventory.json"))?)?;
    let tool = horizon_client::pluto_tool_codec::prepare_tool(
        r##"{rules:[{blocks:"#minecraft:mineable/pickaxe",correct_for_drops:0b},{blocks:"#minecraft:mineable/pickaxe",speed:2.0f,correct_for_drops:1b}],default_mining_speed:1.0f,damage_per_block:1}"##,
        |_| None,
    )?;
    let stack = format!(
        r#"{{id:"minecraft:diamond_pickaxe",count:1,components:{{"minecraft:max_damage":60,"minecraft:damage":1,"minecraft:tool":{tool},"minecraft:custom_data":{{"example:marker":7}}}}}}"#
    );
    let tabs = vec![Tab {
        id: "pluto_tool_rules_demo:tools".into(),
        title: "Pluto Tool Rules".into(),
        icon: r#"{id:"minecraft:diamond_pickaxe",count:1}"#.into(),
        items: std::iter::once(Entry {
            snbt: stack,
            searchable: true,
            permissions_only: false,
            search_lines: vec!["Pluto Rules Pickaxe".into()],
            search_tags: Vec::new(),
        })
        .chain(
            std::env::var_os("PLUTO_DEMO_STACK_LIMIT")
                .into_iter()
                .flat_map(|_| [199, 200, 201])
                .map(|count| Entry {
                    snbt: format!(r#"{{id:"pluto_wand:wand",count:{count}}}"#),
                    searchable: false,
                    permissions_only: false,
                    search_lines: Vec::new(),
                    search_tags: Vec::new(),
                }),
        )
        .collect(),
    }];
    let updated = register_tabs(&doc, &tabs, |s| {
        let mut item = horizon_items::item_data::parse_stack(s)?.ok_or("empty stack")?;
        item.count = 1;
        Ok(horizon_items::item_data::canon_stack(Some(&item)))
    })?;
    fs::create_dir(&output)?;
    for entry in fs::read_dir(&source)? {
        let entry = entry?;
        if entry.file_name() != "creative-inventory.json" {
            std::os::unix::fs::symlink(entry.path(), output.join(entry.file_name()))?;
        }
    }
    fs::write(
        output.join("creative-inventory.json"),
        serde_json::to_vec(&updated)?,
    )?;
    Ok(())
}
