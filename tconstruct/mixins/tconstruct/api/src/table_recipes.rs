//! The tinker station's table-level recipe types (SlimeKnights, MIT; see LICENSE): the
//! fieldless repair gate (`tconstruct:tinker_station_repair`, TinkerStationRepairRecipe's
//! registration — the repair itself is the station's own), the part swapping recipes
//! (`tconstruct:tinker_station_part_swapping`, TinkerStationPartSwapping) that gate the
//! station's part swap, and the tool building recipes (`tconstruct:tool_building`,
//! ToolBuildingRecipe, and its subclass `tconstruct:tipped_tool_transform`,
//! TippedToolTransformRecipe) that build a named tool from fixed materials and extra
//! requirement items instead of loose parts.
use crate::{
    material_recipes::Ingredient,
    materials::Conditions,
    modifier_recipes::Sized,
    tool_materials::MaterialVariant,
};
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::sync::{Arc, RwLock};

pub(crate) const TABLES_FOLDER: &str = "recipes/tables";
pub(crate) const BUILDING_FOLDER: &str = "recipes/tools/building";

/// One family's load over a folder, shaped like the other recipe listeners.
macro_rules! table_family {
    ($family:ident, $recipe:ident, $folder:expr, $types:expr, $listener:ident, $label:literal) => {
        #[derive(Clone, Debug, Default)]
        pub(crate) struct $family { recipes: Vec<$recipe>, pub problems: Vec<String>, pub loaded: bool }
        impl $family {
            pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
                let mut result = Self { loaded: true, ..Self::default() };
                let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                for (id, value) in sorted {
                    let outcome = (|| -> Result<Option<$recipe>, String> {
                        let fields = value.as_object().ok_or("recipe must be an object")?;
                        if !$types.contains(&fields.get("type").and_then(Value::as_str).unwrap_or("")) { return Ok(None); }
                        if let Some(list) = fields.get("conditions") {
                            let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
                            for condition in &parsed { if !conditions.test(condition)? { return Ok(None); } }
                        }
                        $recipe::decode(id, value).map(Some)
                    })();
                    match outcome {
                        Ok(Some(recipe)) => result.recipes.push(recipe),
                        Ok(None) => {}
                        Err(error) => result.problems.push(format!("{id}: {error}")),
                    }
                }
                result
            }
            pub fn len(&self) -> usize { self.recipes.len() }
            pub fn iter(&self) -> impl Iterator<Item = &$recipe> { self.recipes.iter() }
        }
        pub(crate) fn $listener<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<$family>>, String> {
            let state = Arc::new(RwLock::new($family::default()));
            let output = state.clone();
            ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
                Identifier::new("tconstruct", $label).unwrap(), $folder, move |files| {
                    let context = context();
                    let mut state = output.write().map_err(|_| "table recipes poisoned")?;
                    match context {
                        Ok(context) => {
                            *state = $family::load(files, &context);
                            for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                            Ok(())
                        }
                        Err(error) => { *state = $family { problems: vec![error.clone()], ..$family::default() }; Err(error) }
                    }
                })?;
            let stopped = state.clone();
            ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("table recipes poisoned") = $family::default());
            Ok(state)
        }
    };
}

/// A part swapping recipe: which tools may swap parts at the station, the result's stack
/// size cap, and any extra inputs the swap also takes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PartSwap {
    pub id: Identifier,
    pub tools: Ingredient,
    pub max_stack_size: i32,
    pub extra: Vec<Sized>,
}

impl PartSwap {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let tools = Ingredient::decode(fields.get("tools").ok_or("tools missing")?)?;
        let max_stack_size = match fields.get("max_stack_size") {
            None => 1,
            Some(Value::Number(n)) => {
                let n = crate::materials::number_int(&n.to_string());
                if n >= 1 { n } else { return Err("max_stack_size must be at least 1".into()); }
            }
            Some(_) => return Err("max_stack_size must be a number".into()),
        };
        let extra = match fields.get("extra_requirements") {
            None => Vec::new(),
            Some(Value::Array(list)) => list.iter().map(Sized::decode).collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err("extra_requirements must be an array".into()),
        };
        Ok(Self { id: id.clone(), tools, max_stack_size, extra })
    }
}

table_family!(PartSwapping, PartSwap, TABLES_FOLDER, ["tconstruct:tinker_station_part_swapping"], register_part_swapping, "part_swapping_recipes");

impl PartSwapping {
    /// The first recipe (in id order) whose tools accept this tool item.
    pub fn find(&self, tool: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&PartSwap> {
        self.recipes.iter().find(|recipe| recipe.tools.matches(tool, in_tag))
    }
}

/// The repair gate: whether the fieldless `tconstruct:tinker_station_repair` recipe is
/// loaded at all. The station's material repair runs only while it is present.
#[derive(Clone, Debug, Default)]
pub(crate) struct RepairGate {
    pub present: bool,
    pub problems: Vec<String>,
    pub loaded: bool,
}

impl RepairGate {
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<bool, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if fields.get("type").and_then(Value::as_str) != Some("tconstruct:tinker_station_repair") { return Ok(false); }
                if let Some(list) = fields.get("conditions") {
                    let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed { if !conditions.test(condition)? { return Ok(false); } }
                }
                Ok(true)
            })();
            match outcome {
                Ok(true) => result.present = true,
                Ok(false) => {}
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }
}

/// Reload listener for the repair gate, shaped like the other recipe listeners.
pub(crate) fn register_repair_gate<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<RepairGate>>, String> {
    let state = Arc::new(RwLock::new(RepairGate::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "repair_gate").unwrap(), TABLES_FOLDER, move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "repair gate poisoned")?;
            match context {
                Ok(context) => {
                    *state = RepairGate::load(files, &context);
                    for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                    Ok(())
                }
                Err(error) => { *state = RepairGate { problems: vec![error.clone()], ..RepairGate::default() }; Err(error) }
            }
        })?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("repair gate poisoned") = RepairGate::default());
    Ok(state)
}

/// A tool building recipe. `parts_override` is None when the file does not name parts (the
/// tool definition's parts are used); the tipped transform always overrides with no parts,
/// takes its single `input` as the only extra requirement, and names the modifier whose
/// persistent data receives the input's potion.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BuildingRecipe {
    pub id: Identifier,
    pub result: Identifier,
    pub result_count: i32,
    pub extra_requirements: Vec<Ingredient>,
    pub parts_override: Option<Vec<Identifier>>,
    pub extra_materials: Vec<MaterialVariant>,
    pub tipped: Option<Tipped>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Tipped {
    pub modifier: Identifier,
}

impl BuildingRecipe {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let kind = fields.get("type").and_then(Value::as_str).ok_or("type missing")?;
        let result = Identifier::parse(fields.get("result").and_then(Value::as_str).ok_or("result missing")?).map_err(|e| e.to_string())?;
        let result_count = match fields.get("result_count") {
            None => 1,
            Some(Value::Number(n)) => crate::materials::number_int(&n.to_string()),
            Some(_) => return Err("result_count must be a number".into()),
        };
        let materials = |key: &str| -> Result<Vec<MaterialVariant>, String> {
            match fields.get(key) {
                None => Ok(Vec::new()),
                Some(Value::Array(list)) => list.iter().map(|item| {
                    let text = item.as_str().ok_or("a material must be an ID string")?;
                    MaterialVariant::parse(text).ok_or_else(|| format!("invalid material variant {text}"))
                }).collect(),
                Some(_) => Err(format!("{key} must be an array")),
            }
        };
        if kind == "tconstruct:tipped_tool_transform" {
            let input = Ingredient::decode(fields.get("input").ok_or("input missing")?)?;
            let modifier = Identifier::parse(fields.get("modifier").and_then(Value::as_str).ok_or("modifier missing")?).map_err(|e| e.to_string())?;
            return Ok(Self {
                id: id.clone(), result, result_count,
                extra_requirements: vec![input],
                parts_override: Some(Vec::new()),
                extra_materials: materials("materials")?,
                tipped: Some(Tipped { modifier }),
            });
        }
        let extra_requirements = match fields.get("extra_requirements") {
            None => Vec::new(),
            Some(Value::Array(list)) => list.iter().map(Ingredient::decode).collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err("extra_requirements must be an array".into()),
        };
        let parts_override = match fields.get("parts_override") {
            None => None,
            Some(Value::Array(list)) => Some(list.iter().map(|item| {
                Identifier::parse(item.as_str().ok_or("a part must be an item ID string")?).map_err(|e| e.to_string())
            }).collect::<Result<Vec<_>, _>>()?),
            Some(_) => return Err("parts_override must be an array".into()),
        };
        Ok(Self {
            id: id.clone(), result, result_count, extra_requirements, parts_override,
            extra_materials: materials("extra_materials")?,
            tipped: None,
        })
    }
}

table_family!(Building, BuildingRecipe, BUILDING_FOLDER, ["tconstruct:tool_building", "tconstruct:tipped_tool_transform"], register_building, "building_recipes");
