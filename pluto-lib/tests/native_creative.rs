use pluto_lib::native_creative::{register_tabs, Entry, Tab};
use serde_json::{json, Value};
fn key(raw: &str) -> Result<String, String> {
    let mut stack = horizon_items::item_data::parse_stack(raw)?.ok_or("empty stack")?;
    stack.count = 1;
    Ok(horizon_items::item_data::canon_stack(Some(&stack)))
}
fn entry(s: &str) -> Entry {
    Entry {
        snbt: s.into(),
        searchable: true,
        permissions_only: false,
        search_lines: vec!["Demo".into()],
        search_tags: vec!["example:tag".into()],
    }
}
fn tab(id: &str, e: Entry) -> Tab {
    Tab {
        id: id.into(),
        title: "Demo".into(),
        icon: "{count:1,id:\"minecraft:stick\"}".into(),
        items: vec![e],
    }
}
fn base() -> Value {
    json!({"tabs":[{"id":"minecraft:search","type":"SEARCH","row":"TOP","column":6,"items":["{count:1,id:\"minecraft:stone\"}"],"lines":[["Stone"]],"tags":[["minecraft:stone"]],"permissions_only":[0]}]})
}
#[test]
fn preserves_metadata_components_and_deduplicates() {
    let source = base();
    let first = "{count:1,id:\"minecraft:stick\",components:{\"minecraft:custom_data\":{x:1}}}";
    let same = "{id:\"minecraft:stick\",count:2,components:{\"minecraft:custom_data\":{x:1}}}";
    let different = "{count:1,id:\"minecraft:stick\",components:{\"minecraft:custom_data\":{x:2}}}";
    let result = register_tabs(
        &source,
        &[
            tab("demo:a", entry(first)),
            tab("demo:b", entry(same)),
            tab("demo:c", entry(different)),
        ],
        key,
    )
    .unwrap();
    assert_eq!(source["tabs"].as_array().unwrap().len(), 1);
    assert_eq!(
        result["tabs"][0]["items"],
        json!(["{count:1,id:\"minecraft:stone\"}", first, different])
    );
    assert_eq!(
        result["tabs"][0]["lines"],
        json!([["Stone"], ["Demo"], ["Demo"]])
    );
    assert_eq!(
        result["tabs"][0]["tags"],
        json!([["minecraft:stone"], ["example:tag"], ["example:tag"]])
    );
    assert_eq!(result["tabs"][0]["permissions_only"], json!([0]));
    assert_eq!(result["tabs"][1]["items"], json!([first]));
}
#[test]
fn pages_and_permission_conflicts_are_checked_atomically() {
    let source = base();
    let groups = (0..12)
        .map(|n| {
            tab(
                &format!("demo:t{n}"),
                entry("{count:1,id:\"minecraft:stick\"}"),
            )
        })
        .collect::<Vec<_>>();
    let out = register_tabs(&source, &groups, key).unwrap();
    assert_eq!(
        (
            out["tabs"][1]["page"].as_u64(),
            out["tabs"][1]["row"].as_str(),
            out["tabs"][1]["column"].as_u64()
        ),
        (Some(1), Some("TOP"), Some(0))
    );
    assert_eq!(
        (
            out["tabs"][11]["page"].as_u64(),
            out["tabs"][11]["column"].as_u64()
        ),
        (Some(2), Some(0))
    );
    let mut denied = entry("{count:1,id:\"minecraft:stone\"}");
    denied.permissions_only = false;
    assert!(register_tabs(&source, &[tab("demo:conflict", denied)], key).is_err());
    assert!(register_tabs(&source, &[tab("demo:a", entry("-"))], key).is_err());
    assert_eq!(source, base());
}
#[test]
fn respects_search_exclusion_and_existing_page_slots() {
    let mut source = base();
    source["tabs"].as_array_mut().unwrap().push(
        json!({"id":"other:first","type":"CATEGORY","row":"TOP","column":0,"page":1,"visible":false,"items":["x"]}),
    );
    let mut hidden = entry("{count:1,id:\"minecraft:stick\"}");
    hidden.searchable = false;
    hidden.permissions_only = true;
    let out = register_tabs(&source, &[tab("demo:private", hidden)], key).unwrap();
    assert_eq!(out["tabs"][0], source["tabs"][0]);
    assert_eq!(out["tabs"][2]["permissions_only"], json!([0]));
    assert_eq!(out["tabs"][2]["column"], 1);
}

#[test]
fn rejects_ambiguity_and_records_restricted_search_entry() {
    let source = base();
    let mut restricted = entry("{count:1,id:\"minecraft:stick\"}");
    restricted.permissions_only = true;
    let result = register_tabs(&source, &[tab("demo:restricted", restricted)], key).unwrap();
    assert_eq!(result["tabs"][0]["permissions_only"], json!([0, 1]));
    assert_eq!(result["tabs"][1]["permissions_only"], json!([0]));
    let duplicate = [
        tab("demo:same", entry("{count:1,id:\"minecraft:stick\"}")),
        tab("demo:same", entry("{count:1,id:\"minecraft:diamond\"}")),
    ];
    assert!(register_tabs(&source, &duplicate, key).is_err());
    let mut malformed = source.clone();
    malformed["tabs"][0]["page"] = json!(-1);
    assert!(register_tabs(&malformed, &[], key).is_err());
    assert_eq!(source, base());
}
