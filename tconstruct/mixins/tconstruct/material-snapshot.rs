use std::collections::BTreeMap;

// Keep material and tool-definition data in pack order. Material definitions
// and tool definitions select the highest layer; stats and traits merge fields
// across layers. Supported material effects resolve against this snapshot.
pub(super) fn merge_pack(
    materials: &mut BTreeMap<String, Vec<Vec<u8>>>,
    tags: &mut BTreeMap<String, Vec<Vec<u8>>>,
    files: impl IntoIterator<Item = String>,
    read: impl Fn(&str) -> Option<Vec<u8>>,
) {
    // As before, missing reads do not append a layer. Definition/trait/tag
    // interpretation and exact Tinkering directory selection stay here.
    let _ = pluto_api::resources::visit_json_pack_with_policy(
        files,
        read,
        |resource| {
            let path = resource.path;
            let is_tag = path.starts_with("tinkering/tags/materials/");
            let is_material = ["definition", "stats", "traits"].iter().any(|kind| path.starts_with(&format!("tinkering/materials/{kind}/")))
                || path.starts_with("tinkering/tool_definitions/")
                || path.starts_with("tinkering/station_layouts/")
                || path.starts_with("tinkering/fluid_effects/");
            (is_tag || is_material).then_some(is_tag)
        },
        |is_tag, id, bytes| {
            let layers = if is_tag { &mut *tags } else { &mut *materials };
            layers.entry(id).or_default().push(bytes);
        },
        pluto_api::resources::UnreadablePolicy::Skip,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn material_and_tag_files_preserve_all_pack_layers() {
        let material = "data/tconstruct/tinkering/materials/definition/wood.json";
        let tag = "data/tconstruct/tinkering/tags/materials/wood.json";
        let tool = "data/tconstruct/tinkering/tool_definitions/pickaxe.json";
        let layout = "data/tconstruct/tinkering/station_layouts/pickaxe.json";
        let mut values = BTreeMap::new(); let mut tags = BTreeMap::new();
        merge_pack(&mut values, &mut tags, [material.into(), tag.into(), tool.into(), layout.into()], |_| Some(b"base".to_vec()));
        merge_pack(&mut values, &mut tags, [material.into(), tag.into(), tool.into(), layout.into()], |_| Some(b"override".to_vec()));
        assert_eq!(values["tconstruct:tinkering/materials/definition/wood"], [b"base".to_vec(), b"override".to_vec()]);
        assert_eq!(tags["tconstruct:tinkering/tags/materials/wood"], [b"base".to_vec(), b"override".to_vec()]);
        assert_eq!(values["tconstruct:tinkering/tool_definitions/pickaxe"], [b"base".to_vec(), b"override".to_vec()]);
        assert_eq!(values["tconstruct:tinkering/station_layouts/pickaxe"], [b"base".to_vec(), b"override".to_vec()]);
    }
    #[test]
    fn unreadable_layer_is_skipped_without_dropping_later_raw_files() {
        let material = "data/x/tinkering/materials/stats/example.json";
        let tag = "data/x/tinkering/tags/materials/example.json";
        let fluid = "data/x/tinkering/fluid_effects/example.json";
        let mut values = BTreeMap::new();
        let mut tags = BTreeMap::new();
        merge_pack(&mut values, &mut tags, [material.into()], |_| Some(b"base".to_vec()));
        merge_pack(&mut values, &mut tags, [material.into(), tag.into(), fluid.into()], |file| {
            (file != material).then(|| b"not JSON".to_vec())
        });
        assert_eq!(values["x:tinkering/materials/stats/example"], [b"base".to_vec()]);
        assert_eq!(tags["x:tinkering/tags/materials/example"], [b"not JSON".to_vec()]);
        assert_eq!(values["x:tinkering/fluid_effects/example"], [b"not JSON".to_vec()]);
    }
    #[test]
    fn invalid_and_unrelated_paths_never_reach_the_reader() {
        let mut values = BTreeMap::new();
        let mut tags = BTreeMap::new();
        merge_pack(&mut values, &mut tags, [
            "data/x/tinkering/materials/stats/../bad.json",
            "data/x/tinkering/materials/stats/.json",
            "data/x/tinkering/materials/stats//bad.json",
            "data/x/tinkering/materials/unknown/good.json",
            "data/x/tinkering/modifiers/good.json",
            "assets/x/tinkering/materials/stats/good.json",
        ].map(str::to_owned), |_| panic!("invalid or unselected path was read"));
        assert!(values.is_empty());
        assert!(tags.is_empty());
    }
}
