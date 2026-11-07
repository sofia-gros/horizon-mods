use std::collections::BTreeMap;

/// Merge one pack's original-format recipe files by namespaced ID. Higher
/// priority packs call this later. Invalid names are ignored; bytes stay raw
/// until the source recipe loader validates them. This is not yet used for
/// selecting or taking Part Builder recipes.
pub(super) fn merge_pack(
    recipes: &mut BTreeMap<String, Vec<u8>>,
    files: impl IntoIterator<Item = String>,
    read: impl Fn(&str) -> Option<Vec<u8>>,
) {
    // This loader historically ignores unreadable files. Keep that policy
    // separate from the strict modifier snapshot, and retain raw last-wins data.
    let _ = pluto_api::resources::visit_json_pack_with_policy(
        files,
        read,
        |resource| {
            let path = resource.path.strip_prefix("recipes/")?;
            Some(format!("{}:{path}", resource.namespace))
        },
        |id, _, bytes| { recipes.insert(id, bytes); },
        pluto_api::resources::UnreadablePolicy::Skip,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_forge_metadata_until_the_recipe_load_phase() {
        let mut recipes = BTreeMap::new();
        merge_pack(&mut recipes, ["data/tconstruct/recipes/_metadata.json".into()], |_| Some(b"{}".to_vec()));
        assert!(recipes.contains_key("tconstruct:_metadata"));
    }
    #[test]
    fn later_pack_overrides_by_id_and_world_snapshots_are_independent() {
        let path = "data/tconstruct/recipes/tools/materials/flint/flint.json";
        let mut first = BTreeMap::new();
        let mut second = BTreeMap::new();
        merge_pack(&mut first, [path.into()], |_| Some(b"base".to_vec()));
        merge_pack(&mut second, [path.into()], |_| Some(b"base".to_vec()));
        merge_pack(&mut first, [path.into(), "data/tconstruct/recipes/../bad.json".into()], |name| Some(if name == path { b"override".to_vec() } else { b"bad".to_vec() }));
        assert_eq!(first.get("tconstruct:tools/materials/flint/flint"), Some(&b"override".to_vec()));
        assert_eq!(second.get("tconstruct:tools/materials/flint/flint"), Some(&b"base".to_vec()));
        assert_eq!(first.len(), 1);
    }
    #[test]
    fn unreadable_override_preserves_prior_recipe_and_continues() {
        let first = "data/x/recipes/first.json";
        let second = "data/x/recipes/second.json";
        let mut recipes = BTreeMap::new();
        merge_pack(&mut recipes, [first.into()], |_| Some(b"base".to_vec()));
        merge_pack(&mut recipes, [first.into(), second.into()], |file| {
            (file == second).then(|| b"not JSON".to_vec())
        });
        assert_eq!(recipes["x:first"], b"base");
        assert_eq!(recipes["x:second"], b"not JSON");
        merge_pack(&mut recipes, [first.into()], |_| Some(b"malformed override".to_vec()));
        assert_eq!(recipes["x:first"], b"malformed override");
    }
    #[test]
    fn invalid_and_unrelated_paths_never_reach_the_reader() {
        let mut recipes = BTreeMap::new();
        merge_pack(&mut recipes, [
            "data/x/recipes/../bad.json",
            "data/x/recipes/.json",
            "data/x/recipes//bad.json",
            "data/x/Recipes/bad.json",
            "data/x/tags/items/good.json",
            "assets/x/recipes/good.json",
        ].map(str::to_owned), |_| panic!("invalid or unselected path was read"));
        assert!(recipes.is_empty());
    }
}
