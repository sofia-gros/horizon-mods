#[path = "../../mixins/tconstruct/source-tags.rs"]
mod tconstruct_source_tags;

#[test]
fn gold_platform_resolves_as_basin_cast_and_pack_replace_wins() {
    use std::collections::{BTreeMap, HashSet};
    let id = "tconstruct:casts/empty/basin".to_owned();
    let base = include_bytes!("../../content/reference/tconstruct-tags/data/tconstruct/tags/items/casts/empty/basin.json");
    let registry = HashSet::from(["tconstruct:gold_platform".to_owned()]);
    let mut tags = BTreeMap::from([(id.clone(), vec![base.to_vec()])]);
    assert_eq!(tconstruct_source_tags::resolve_tag(&id, &tags, &registry).unwrap(), vec!["tconstruct:gold_platform"]);
    tags.get_mut(&id).unwrap().push(br#"{"replace":true,"values":[]}"#.to_vec());
    assert!(tconstruct_source_tags::resolve_tag(&id, &tags, &registry).unwrap().is_empty());
}
