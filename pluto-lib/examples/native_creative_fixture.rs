//! Run the shared API on a fixture containing `inventory` and `groups`.
use pluto_lib::native_creative::{register_tabs, Entry, Tab};
use serde_json::Value;
use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        return Err("usage: native_creative_fixture INPUT OUTPUT".into());
    }
    let input: Value = serde_json::from_slice(&fs::read(&args[1])?)?;
    let groups = input["groups"]
        .as_array()
        .ok_or("missing groups")?
        .iter()
        .map(|tab| {
            let string = |key: &str| {
                tab[key]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or("invalid group field")
            };
            let items = tab["items"]
                .as_array()
                .ok_or("invalid group items")?
                .iter()
                .map(|item| {
                    Ok(Entry {
                        snbt: item.as_str().ok_or("invalid group stack")?.into(),
                        searchable: true,
                        permissions_only: false,
                        search_lines: Vec::new(),
                        search_tags: Vec::new(),
                    })
                })
                .collect::<Result<Vec<_>, &str>>()?;
            Ok(Tab {
                id: string("id")?,
                title: string("title")?,
                icon: string("icon")?,
                items,
            })
        })
        .collect::<Result<Vec<_>, &str>>()?;
    let out = register_tabs(&input["inventory"], &groups, |raw| {
        let mut stack = horizon_items::item_data::parse_stack(raw)?.ok_or("empty stack")?;
        stack.count = 1;
        Ok(horizon_items::item_data::canon_stack(Some(&stack)))
    })?;
    fs::write(&args[2], serde_json::to_vec_pretty(&out)?)?;
    Ok(())
}
