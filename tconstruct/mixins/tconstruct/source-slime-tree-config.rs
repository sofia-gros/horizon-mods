// Decode the original simple-state slime-tree configuration without inventing
// a vanilla tree or a fallback for an unsupported provider.
use serde_json::Value;
use std::collections::BTreeMap;
use super::tconstruct_slime_tree_world::TreeConfig;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SlimeTreeSpec {
    pub id: String,
    pub base_height: i32,
    pub random_height: u32,
    pub double_height: bool,
    pub vines: bool,
    pub trunk: String,
    pub trunk_properties: BTreeMap<String, String>,
    pub leaves: String,
    pub leaves_properties: BTreeMap<String, String>,
    pub vine: Option<String>,
    pub vine_properties: BTreeMap<String, String>,
}

impl SlimeTreeSpec {
    pub(crate) fn from_json(id: &str, source: &Value) -> Option<Self> {
        if source.get("type")?.as_str()? != "tconstruct:slime_tree" { return None; }
        let config = source.get("config")?;
        let state = |key: &str| {
            let provider = config.get(key)?;
            if provider.get("type")?.as_str()? != "minecraft:simple_state_provider" { return None; }
            let state = provider.get("state")?;
            let name = state.get("Name")?.as_str()?.to_owned();
            let mut properties = BTreeMap::new();
            if let Some(values) = state.get("Properties") {
                for (name, value) in values.as_object()? { properties.insert(name.clone(), value.as_str()?.to_owned()); }
            }
            Some((name, properties))
        };
        let base_height = i32::try_from(config.get("base_height")?.as_i64()?).ok()?;
        let random_height = u32::try_from(config.get("random_height")?.as_u64()?).ok()?;
        if base_height <= 0 || random_height == 0 || random_height > i32::MAX as u32 { return None; }
        let double_height = config.get("can_double_height")?.as_bool()?;
        let vines = config.get("has_vines")?.as_bool()?;
        let (trunk, trunk_properties) = state("trunk_provider")?;
        let (leaves, leaves_properties) = state("leaves_provider")?;
        let (vine, vine_properties) = if vines { let (name, props) = state("vines_provider")?; (Some(name), props) } else { (None, BTreeMap::new()) };
        Some(Self { id: id.to_owned(), base_height, random_height, double_height, vines,
            trunk, trunk_properties, leaves, leaves_properties, vine, vine_properties })
    }

    pub(crate) fn config(&self) -> TreeConfig<'_> {
        TreeConfig { base_height: self.base_height, random_height: self.random_height,
            double_height: self.double_height, vines: self.vines, trunk: &self.trunk,
            trunk_properties: &self.trunk_properties, leaves: &self.leaves,
            leaves_properties: &self.leaves_properties, vine: self.vine.as_deref(),
            vine_properties: &self.vine_properties }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_earth_and_sky_configs_keep_height_and_providers() {
        let read = |file: &str| -> Value {
            let base = std::path::Path::new("tconstruct/content/reference/slime-islands/data/tconstruct/worldgen/configured_feature");
            serde_json::from_slice(&std::fs::read(base.join(file)).unwrap()).unwrap()
        };
        let earth = SlimeTreeSpec::from_json("tconstruct:earth_slime_island_tree", &read("earth_slime_island_tree.json")).unwrap();
        let sky = SlimeTreeSpec::from_json("tconstruct:sky_slime_island_tree", &read("sky_slime_island_tree.json")).unwrap();
        assert_eq!((earth.base_height, earth.random_height, earth.double_height, earth.vines), (4, 3, false, false));
        assert_eq!((sky.base_height, sky.random_height, sky.double_height, sky.vines), (5, 4, true, true));
        assert_eq!(earth.trunk, "tconstruct:greenheart_log");
        assert_eq!(sky.vine.as_deref(), Some("tconstruct:sky_slime_vine"));
        assert_eq!(sky.vine_properties.get("stage").map(String::as_str), Some("middle"));
        assert_eq!(earth.trunk_properties.get("axis").map(String::as_str), Some("y"));
        assert_eq!(earth.leaves_properties.get("distance").map(String::as_str), Some("7"));
        assert!(SlimeTreeSpec::from_json("invalid", &serde_json::json!({"type":"minecraft:tree"})).is_none());
    }
}
