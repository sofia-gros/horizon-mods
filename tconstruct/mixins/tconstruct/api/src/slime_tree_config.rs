//! Original SlimeTreeConfig codec for the simple providers used by the shipped
//! Earth/Sky configurations. Unsupported providers fail before world writes;
//! never flatten a weighted/random provider into its first state.
use super::{grow_tree, TreeConfig};
use pluto_lib::{BlockState, ConfiguredFeature, FeatureTypeRegistry, Identifier};
use serde_json::Value;

pub(super) struct Prepared {
    trunk: String,
    leaves: String,
    vines: String,
    base_height: i32,
    random_height: i32,
    can_double_height: bool,
    has_vines: bool,
}

fn provider(value: &Value, field: &str) -> Result<String, String> {
    let provider = value.get(field).ok_or_else(|| format!("missing {field}"))?;
    if provider.get("type").and_then(Value::as_str) != Some("minecraft:simple_state_provider") {
        return Err(format!("unsupported {field}: expected original simple_state_provider"));
    }
    let source = provider.get("state").and_then(Value::as_object).ok_or_else(|| format!("missing {field} state"))?;
    let name = source.get("Name").and_then(Value::as_str).ok_or_else(|| format!("missing {field} state Name"))?;
    let mut state = BlockState::new(Identifier::parse(name).map_err(|e| e.to_string())?);
    if let Some(properties) = source.get("Properties") {
        for (key, value) in properties.as_object().ok_or("state Properties must be an object")? {
            let value = value.as_str().ok_or("state property must be a string")?;
            // Use the normal textual parser to reject malformed property names
            // and values before a registered feature can place any blocks.
            state = state.with(key, value);
        }
    }
    let text = state.to_string();
    BlockState::parse(&text).map_err(|e| e.to_string())?;
    Ok(text)
}

fn integer(value: &Value, field: &str, default: i32) -> Result<i32, String> {
    value.get(field).map_or(Ok(default), |v| v.as_i64().and_then(|v| i32::try_from(v).ok()).ok_or_else(|| format!("invalid {field}")))
}
fn boolean(value: &Value, field: &str) -> Result<bool, String> {
    value.get(field).map_or(Ok(false), |v| v.as_bool().ok_or_else(|| format!("invalid {field}")))
}

impl Prepared {
    pub(super) fn decode(value: &Value) -> Result<Self, String> {
        if !value.is_object() { return Err("slime tree config must be an object".into()); }
        let trunk = provider(value, "trunk_provider")?;
        let leaves = provider(value, "leaves_provider")?;
        let vines = provider(value, "vines_provider")?;
        let base_height = integer(value, "base_height", 0)?;
        let random_height = integer(value, "random_height", 0)?;
        // Reject an invalid nextInt bound or overflowing geometry before RNG
        // draws/writes. All original shipped values fit these work bounds.
        if !(0..=256).contains(&base_height) || !(1..=256).contains(&random_height) {
            return Err("unsupported slime tree height bounds".into());
        }
        let can_double_height = boolean(value, "can_double_height")?;
        let has_vines = boolean(value, "has_vines")?;
        let _planted = boolean(value, "planted")?; // Java's tree feature does not read this flag.
        Ok(Self { trunk, leaves, vines, base_height, random_height, can_double_height, has_vines })
    }

    pub(super) fn place(&self, world: &mut dyn pluto_lib::BlockEditor, origin: [i32; 3]) -> bool {
        grow_tree(world, origin, &TreeConfig {
            trunk: &self.trunk, leaves: &self.leaves,
            vines: self.has_vines.then_some(self.vines.as_str()),
            base_height: self.base_height, random_height: self.random_height,
            can_double_height: self.can_double_height,
        })
    }
}

pub(super) fn register() {
    assert!(FeatureTypeRegistry::register(Identifier::parse("tconstruct:slime_tree").expect("fixed id"), |value| {
        let tree = Prepared::decode(value)?;
        Ok(ConfiguredFeature::new(move |context| tree.place(context.world, context.origin)))
    }), "slime tree codec registration failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::MemoryBlockView;

    #[test]
    fn shipped_tree_settings_preserve_properties_height_draws_and_vines() {
        for (text, doubled, expected_rolls, trunk) in [
            (include_str!("../../../../content/data/tconstruct/island_feature/earth_slime_island_tree.json"), false, vec![2, 91], "tconstruct:greenheart_log"),
            (include_str!("../../../../content/data/tconstruct/island_feature/sky_slime_island_tree.json"), true, vec![2, 0, 91], "tconstruct:skyroot_log"),
        ] {
            let value: Value = serde_json::from_str(text).unwrap();
            let prepared = Prepared::decode(&value["config"]).unwrap();
            let mut world = MemoryBlockView::default();
            world.blocks.insert([0,0,0], BlockState::parse("tconstruct:earth_slime_dirt").unwrap());
            world.tags.insert([0,0,0], ["tconstruct:slimy_soil".into()].into());
            let height = if doubled { 14 } else { 6 };
            for x in -5..=5 { for y in 1..=20 { for z in -5..=5 {
                if x != 0 || z != 0 || y >= height + 1 {
                    world.tags.insert([x,y,z], ["minecraft:leaves".into()].into());
                }
            }}}
            world.rolls = expected_rolls;
            assert!(prepared.place(&mut world, [0,1,0]));
            assert_eq!(world.blocks.values().filter(|state| state.block.to_string() == trunk).count(), height as usize);
            assert!(world.blocks.values().filter(|state| state.block.to_string() == trunk).all(|state| state.property("axis") == Some("y")));
            assert!(world.blocks.values().filter(|state| state.block.to_string().ends_with("_leaves")).all(|state| state.property("persistent") == Some("false") && state.property("waterlogged") == Some("false")));
            let vines = world.blocks.values().filter(|state| state.block.to_string().ends_with("_vine")).count();
            assert_eq!(vines, if doubled { 12 } else { 0 });
            if !doubled { assert_eq!(world.rolls, vec![91]); }
        }
    }

    #[test]
    fn unsupported_providers_and_invalid_configs_reject_before_placement() {
        let source: Value = serde_json::from_str(include_str!("../../../../content/data/tconstruct/island_feature/sky_slime_island_tree.json")).unwrap();
        for field in ["trunk_provider", "leaves_provider", "vines_provider"] {
            let mut config = source["config"].clone();
            config[field]["type"] = Value::from("minecraft:weighted_state_provider");
            assert!(Prepared::decode(&config).is_err());
        }
        let mut config = source["config"].clone();
        config["random_height"] = Value::from(0);
        assert!(Prepared::decode(&config).is_err());
        config = source["config"].clone();
        config["has_vines"] = Value::from("false");
        assert!(Prepared::decode(&config).is_err());
    }
}
