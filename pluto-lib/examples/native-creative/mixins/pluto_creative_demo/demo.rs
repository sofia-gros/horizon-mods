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
    let icon = "{count:1,id:\"minecraft:stick\"}";
    let stacks = [
        "{count:1,id:\"minecraft:stick\",components:{\"minecraft:custom_data\":{\"demo:value\":1}}}",
        "{count:1,id:\"minecraft:diamond\"}",
    ];
    let tabs: Vec<_> = (1..=12)
        .map(|n| Tab {
            id: format!("pluto_demo:tab_{n}"),
            title: format!("Pluto Demo {n}"),
            icon: icon.into(),
            items: vec![Entry {
                snbt: stacks[n % 2].into(),
                searchable: true,
                permissions_only: false,
                search_lines: vec![format!("Pluto Demo {n}")],
                search_tags: Vec::new(),
            }],
        })
        .collect();
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
