//! Explicit placement of the original configured Ender trees through PlutoLib.
//! This translates data schemas; native simulation and provider sampling remain
//! in PlutoLib, and original custom/native decoration order stays explicit.
use pluto_lib::{ConfiguredFeature, FeatureTypeRegistry, Identifier, RootPlacement, RootPlacementTypeRegistry, TreeDecoration, TreeDecoratorTypeRegistry};
use serde_json::{Map, Value};

#[path = "source_tree_data.rs"]
mod data;

pub(super) fn grow(world: &mut dyn pluto_lib::BlockEditor, origin: [i32; 3], tall: bool) -> bool {
    let source = serde_json::from_str(if tall { data::TALL } else { data::SMALL }).map_err(|e| e.to_string());
    match source.and_then(|source| PreparedTree::decode(&source)) {
        Ok(tree) => world.place_tree_with_plan(&tree.native, origin, Some(&tree.roots), &tree.decorations),
        Err(error) => { eprintln!("tconstruct: original Ender configuration failed: {error}"); false }
    }
}

pub(super) fn register() {
    assert!(FeatureTypeRegistry::register(Identifier::parse("tconstruct:source_tree").expect("fixed id"), |source| {
        let tree = PreparedTree::decode(source)?;
        Ok(ConfiguredFeature::new(move |context| context.world.place_tree_with_plan(
            &tree.native, context.origin, Some(&tree.roots), &tree.decorations,
        )))
    }), "source tree registration failed");
}

struct PreparedTree {
    native: Value,
    roots: RootPlacement,
    decorations: Vec<TreeDecoration>,
}

impl PreparedTree {
    fn decode(source: &Value) -> Result<Self, String> {
        if source.get("type").and_then(Value::as_str) != Some("minecraft:tree") {
            return Err("expected original minecraft:tree configuration".into());
        }
        let config = source.get("config").and_then(Value::as_object).ok_or("tree config missing")?;
        // The Ender configs use an upwards-branching trunk, which does not use
        // the native below-trunk write. Do not generalize force-dirt semantics.
        if config.get("force_dirt").and_then(Value::as_bool) != Some(false)
            || config.get("trunk_placer").and_then(|v| v.get("type")).and_then(Value::as_str) != Some("minecraft:upwards_branching_trunk_placer") {
            return Err("source tree requires Ender upwards-branching/force_dirt=false settings".into());
        }
        let mut remaining = 65536;
        let mut native = normalize(&source["config"], 0, &mut remaining)?;
        let root = native.get_mut("root_placer").and_then(Value::as_object_mut).ok_or("root placer missing")?;
        if root.get("type").and_then(Value::as_str) != Some("tconstruct:extra_root_variants") {
            return Err("expected extra_root_variants".into());
        }
        let roots = RootPlacementTypeRegistry::decode(&Identifier::parse("tconstruct:extra_root_variants").unwrap(), &Value::Object(root.clone()))?
            .ok_or("extra root codec not registered")?;
        root.insert("type".into(), Value::String("minecraft:mangrove_root_placer".into()));
        root.remove("root_variants");
        let original = native.get_mut("decorators").and_then(Value::as_array_mut).ok_or("decorators missing")?;
        if original.len() > 256 { return Err("source decorator count".into()) }
        let mut ordinary = Vec::new();
        let mut decorations = Vec::new();
        for entry in std::mem::take(original) {
            let kind = entry.get("type").and_then(Value::as_str).ok_or("decorator type missing")?;
            let id = Identifier::parse(kind).map_err(|e| e.to_string())?;
            match TreeDecoratorTypeRegistry::decode(&id, &entry)? {
                Some(custom) => decorations.push(TreeDecoration::Custom(custom)),
                None => {
                    // Unknown custom decorators must not silently become native
                    // approximations. The native decoder validates vanilla ones.
                    if !kind.starts_with("minecraft:") { return Err(format!("unknown source decorator {kind}")) }
                    decorations.push(TreeDecoration::Native(ordinary.len()));
                    ordinary.push(entry);
                }
            }
        }
        *original = ordinary;
        let object = native.as_object_mut().expect("normalized object");
        object.insert("type".into(), Value::String("minecraft:tree".into()));
        object.remove("force_dirt");
        Ok(Self { native, roots, decorations })
    }
}

/// Translate the old Java codec spelling without sampling, dropping properties,
/// changing list order or flattening providers. Unsupported provider types fail.
fn normalize(source: &Value, depth: usize, remaining: &mut usize) -> Result<Value, String> {
    if depth > 128 || *remaining == 0 { return Err("source tree data budget".into()) }
    *remaining -= 1;
    match source {
        Value::Array(values) => values.iter().map(|v| normalize(v, depth + 1, remaining)).collect(),
        Value::Object(values) => {
            let mut out = Map::new();
            for (key, value) in values {
                let key = match key.as_str() { "Name" => "id", "Properties" => "properties", "dirt_provider" => "below_trunk_provider", _ => key };
                if out.insert(key.into(), normalize(value, depth + 1, remaining)?).is_some() { return Err(format!("ambiguous source field {key}")) }
            }
            if let Some(kind) = out.get("type").and_then(Value::as_str) {
                let translated = match kind {
                    "minecraft:simple_state_provider" => Some("minecraft:simple"),
                    "minecraft:randomized_int_state_provider" => Some("minecraft:randomized_int"),
                    other if other.ends_with("_state_provider") => return Err(format!("unsupported source provider {other}")),
                    _ => None,
                };
                if let Some(kind) = translated { out.insert("type".into(), Value::String(kind.into())); }
            }
            if out.get("type").and_then(Value::as_str) == Some("minecraft:uniform") {
                let bounds = out.remove("value").and_then(|v| v.as_object().cloned()).ok_or("source uniform bounds missing")?;
                for (key, value) in bounds {
                    if out.insert(key.clone(), value).is_some() { return Err(format!("ambiguous uniform bound {key}")) }
                }
            }
            Ok(Value::Object(out))
        }
        other => Ok(other.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_schema_keeps_providers_properties_bounds_root_order_and_decorator_order() {
        super::super::register();
        let simple = |name| serde_json::json!({"type":"minecraft:simple_state_provider","state":{"Name":name,"Properties":{"axis":"z"}}});
        let source = serde_json::json!({"type":"minecraft:tree","config":{
            "force_dirt":false,"trunk_placer":{"type":"minecraft:upwards_branching_trunk_placer","base_height":2},
            "trunk_provider":simple("tconstruct:enderbark_log"),"dirt_provider":simple("minecraft:dirt"),
            "root_placer":{"type":"tconstruct:extra_root_variants","root_variants":[
                {"matches":["tconstruct:sky_congealed_slime"],"state_provider":simple("tconstruct:sky_enderbark_roots")},
                {"matches":["tconstruct:ichor_congealed_slime"],"state_provider":simple("tconstruct:ichor_enderbark_roots")}
            ],"trunk_offset_y":{"type":"minecraft:uniform","value":{"min_inclusive":1,"max_inclusive":3}}},
            "decorators":[{"type":"tconstruct:leave_vines","vines":"tconstruct:ender_slime_vine","probability":0.125},
                {"type":"minecraft:attached_to_leaves","block_provider":{"type":"minecraft:randomized_int_state_provider","source":simple("tconstruct:ender_slime_sapling"),"property":"age","values":{"type":"minecraft:uniform","value":{"min_inclusive":0,"max_inclusive":4}}}}]
        }});
        let tree = PreparedTree::decode(&source).unwrap();
        assert_eq!(tree.native["trunk_provider"]["type"], "minecraft:simple");
        assert_eq!(tree.native["trunk_provider"]["state"]["id"], "tconstruct:enderbark_log");
        assert_eq!(tree.native["trunk_provider"]["state"]["properties"]["axis"], "z");
        assert_eq!(tree.native["root_placer"]["type"], "minecraft:mangrove_root_placer");
        assert_eq!(tree.native["root_placer"]["trunk_offset_y"]["min_inclusive"], 1);
        assert_eq!(tree.native["root_placer"]["trunk_offset_y"]["max_inclusive"], 3);
        assert_eq!(tree.roots.bindings().block_sets, vec![serde_json::json!(["tconstruct:sky_congealed_slime"]), serde_json::json!(["tconstruct:ichor_congealed_slime"])]);
        assert_eq!(tree.roots.bindings().state_providers[1]["state"]["id"], "tconstruct:ichor_enderbark_roots");
        assert!(matches!(&tree.decorations[..], [TreeDecoration::Custom(_), TreeDecoration::Native(0)]));
        let provider = &tree.native["decorators"][0]["block_provider"];
        assert_eq!(provider["type"], "minecraft:randomized_int");
        assert_eq!(provider["values"]["max_inclusive"], 4);
        assert!(tree.native.get("below_trunk_provider").is_some());
        let mut bad = source.clone();
        bad["config"]["decorators"][0]["type"] = "example:unknown".into();
        assert!(PreparedTree::decode(&bad).is_err());
        bad = source.clone(); bad["config"]["force_dirt"] = true.into();
        assert!(PreparedTree::decode(&bad).is_err());
        bad = source; bad["config"]["trunk_provider"]["type"] = "example:unknown_state_provider".into();
        assert!(PreparedTree::decode(&bad).is_err());
    }
}
