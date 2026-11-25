//! Material swapping recipes for the tinker station (SlimeKnights, MIT; see LICENSE):
//! `tconstruct:fixed_material_swapping`, `tconstruct:material_value_swapping` and
//! `tconstruct:tool_material_swapping` (MaterialSwappingRecipe and its subclasses).
//! Loaded from the pack's recipes tree; the station dispatch in tool_station.rs runs them.
//! Java revision: f4e77d43f39ba1047bf93ae816b7e1beaab514c2.
use crate::{
    material_recipes::Ingredient, materials::Conditions,
    shaped_materials::{decode_predicate, Predicate},
    tool_materials::MaterialVariant,
};
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::sync::{Arc, RwLock};

/// The whole recipes tree: the three swapping types live under tables/ and tools/armor/,
/// and every other recipe file is ignored by type.
pub(crate) const FOLDER: &str = "recipes";

/// SizedIngredient: an ingredient plus how many items one use takes (default 1).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SizedIngredient {
    pub ingredient: Ingredient,
    pub amount: i32,
}

impl SizedIngredient {
    fn decode(value: &Value) -> Result<Self, String> {
        if let Some(fields) = value.as_object() {
            if let Some(inner) = fields.get("ingredient") {
                let amount = match fields.get("amount") {
                    None => 1,
                    Some(Value::Number(n)) => {
                        let n = crate::materials::number_int(&n.to_string());
                        if n >= 1 { n } else { return Err("sized ingredient amount must be at least 1".into()) }
                    }
                    Some(_) => return Err("sized ingredient amount must be a number".into()),
                };
                return Ok(Self { ingredient: Ingredient::decode(inner)?, amount });
            }
        }
        Ok(Self { ingredient: Ingredient::decode(value)?, amount: 1 })
    }
}

/// One loaded swapping recipe; the fields are the three serializers' own.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Recipe {
    /// FixedMaterialSwappingRecipe: a named ingredient sets a fixed material.
    Fixed {
        tools: Ingredient,
        max_stack_size: i32,
        extra: Vec<SizedIngredient>,
        ingredient: SizedIngredient,
        material: MaterialVariant,
        indices: Vec<i32>,
        /// IntLoadable.FROM_ZERO, default 0.
        repair_value: i32,
    },
    /// MaterialValueSwappingRecipe: a material item (by material recipe) whose variant
    /// satisfies the predicate sets its own material, paying `cost` of the item's value.
    Value {
        tools: Ingredient,
        max_stack_size: i32,
        extra: Vec<SizedIngredient>,
        predicate: Predicate,
        cost: i32,
        indices: Vec<i32>,
    },
    /// ToolMaterialSwappingRecipe: a copy of the tool itself is the input; max stack is 1.
    Tool { tools: Ingredient, extra: Vec<SizedIngredient> },
}

impl Recipe {
    pub fn tools(&self) -> &Ingredient {
        match self { Self::Fixed { tools, .. } | Self::Value { tools, .. } | Self::Tool { tools, .. } => tools }
    }
    pub fn max_stack_size(&self) -> i32 {
        match self {
            Self::Fixed { max_stack_size, .. } | Self::Value { max_stack_size, .. } => *max_stack_size,
            // ToolMaterialSwappingRecipe's loader passes a fixed 1 to the base class.
            Self::Tool { .. } => 1,
        }
    }
    pub fn extra(&self) -> &[SizedIngredient] {
        match self { Self::Fixed { extra, .. } | Self::Value { extra, .. } | Self::Tool { extra, .. } => extra }
    }

    fn decode(kind: &str, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let tools = Ingredient::decode(fields.get("tools").ok_or("tools missing")?)?;
        // STACK_SIZE_FIELD: FROM_ONE with ITinkerStationRecipe.DEFAULT_TOOL_STACK_SIZE (16).
        let max_stack_size = match fields.get("max_stack_size") {
            None => 16,
            Some(Value::Number(n)) => {
                let n = crate::materials::number_int(&n.to_string());
                if n >= 1 { n } else { return Err("max_stack_size must be at least 1".into()) }
            }
            Some(_) => return Err("max_stack_size must be a number".into()),
        };
        let extra = match fields.get("extra_requirements") {
            None => Vec::new(),
            Some(Value::Array(list)) => list.iter().map(SizedIngredient::decode).collect::<Result<_, _>>()?,
            Some(_) => return Err("extra_requirements must be an array".into()),
        };
        Ok(match kind {
            "tconstruct:fixed_material_swapping" => Self::Fixed {
                tools,
                max_stack_size,
                extra,
                ingredient: SizedIngredient::decode(fields.get("ingredient").ok_or("ingredient missing")?)?,
                material: MaterialVariant::parse(fields.get("material").and_then(Value::as_str).ok_or("material must be a string")?)
                    .ok_or("invalid material variant")?,
                indices: decode_indices(fields.get("index"))?,
                repair_value: match fields.get("repair_value") {
                    None => 0,
                    Some(Value::Number(n)) => {
                        let n = crate::materials::number_int(&n.to_string());
                        if n >= 0 { n } else { return Err("repair_value must be at least 0".into()) }
                    }
                    Some(_) => return Err("repair_value must be a number".into()),
                },
            },
            "tconstruct:material_value_swapping" => Self::Value {
                tools,
                max_stack_size,
                extra,
                predicate: decode_predicate(fields.get("material"))?,
                cost: match fields.get("cost") {
                    Some(Value::Number(n)) => {
                        let n = crate::materials::number_int(&n.to_string());
                        if n >= 1 { n } else { return Err("cost must be at least 1".into()) }
                    }
                    _ => return Err("cost must be a number".into()),
                },
                indices: decode_indices(fields.get("index"))?,
            },
            "tconstruct:tool_material_swapping" => Self::Tool { tools, extra },
            _ => return Err(format!("not a swapping recipe type: {kind}")),
        })
    }
}

/// IntArrayLoadable COMPACT: a bare int or an array of ints, each FROM_ZERO.
fn decode_indices(value: Option<&Value>) -> Result<Vec<i32>, String> {
    let one = |value: &Value| -> Result<i32, String> { match value {
        Value::Number(n) => {
            let n = crate::materials::number_int(&n.to_string());
            if n >= 0 { Ok(n) } else { Err("index must be at least 0".into()) }
        }
        _ => Err("index must be a number".into()),
    } };
    let indices = match value {
        Some(Value::Array(list)) => list.iter().map(one).collect::<Result<Vec<_>, _>>()?,
        Some(single) => vec![one(single)?],
        None => return Err("index missing".into()),
    };
    if indices.is_empty() { return Err("index must not be empty".into()); }
    Ok(indices)
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Recipes {
    recipes: Vec<Recipe>,
    pub problems: Vec<String>,
    pub loaded: bool,
}

impl Recipes {
    /// Decodes every file of the three swapping types; other types are ignored, a file whose
    /// condition is false is skipped, and a failing one is a problem.
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<Option<Recipe>, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                let kind = fields.get("type").and_then(Value::as_str).unwrap_or("");
                if !matches!(kind, "tconstruct:fixed_material_swapping" | "tconstruct:material_value_swapping" | "tconstruct:tool_material_swapping") {
                    return Ok(None);
                }
                if let Some(list) = fields.get("conditions") {
                    let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed { if !conditions.test(condition)? { return Ok(None); } }
                }
                Recipe::decode(kind, value).map(Some)
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
    pub fn recipes(&self) -> &[Recipe] { &self.recipes }
}

/// Installs the reload listener, mirroring material_recipes::register.
pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Recipes>>, String> {
    let state = Arc::new(RwLock::new(Recipes::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "swapping_recipes").unwrap(), FOLDER, move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "swapping recipes poisoned")?;
            match context {
                Ok(context) => {
                    *state = Recipes::load(files, &context);
                    for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                    Ok(())
                }
                Err(error) => { *state = Recipes { problems: vec![error.clone()], ..Recipes::default() }; Err(error) }
            }
        })?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("swapping recipes poisoned") = Recipes::default());
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Always;
    impl Conditions for Always {
        type Parsed = ();
        fn decode(&self, value: &Value) -> Result<(), String> { value.as_bool().map(|_| ()).ok_or("condition must be boolean".into()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    #[test]
    fn the_three_types_load_with_their_defaults_and_other_types_are_ignored() {
        let files = vec![
            (id("tconstruct:tables/tool_material_swapping"), json!({"type":"tconstruct:tool_material_swapping","tools":{"type":"forge:difference","base":{"tag":"tconstruct:modifiable/multipart"},"subtracted":{"tag":"tconstruct:modifiable/unswappable/tools"}}})),
            (id("tconstruct:tools/armor/slime_skull/swapping/iron"), json!({"type":"tconstruct:fixed_material_swapping","index":0,"ingredient":{"item":"tconstruct:husk_head"},"material":"tconstruct:iron","max_stack_size":16,"repair_value":6,"tools":{"tag":"tconstruct:modifiable/skulls/swappable"}})),
            (id("tconstruct:tools/armor/travelers/shield_wood"), json!({"type":"tconstruct:material_value_swapping","cost":2,"index":0,"tools":{"item":"tconstruct:travelers_shield"},"material":{"type":"tconstruct:stat_type","stat_type":"tconstruct:shield_core"},"max_stack_size":16})),
            (id("tconstruct:tools/materials/iron_ingot"), json!({"type":"tconstruct:material","ingredient":{"item":"minecraft:iron_ingot"},"material":"tconstruct:iron"})),
            (id("tconstruct:tools/armor/travelers/bad"), json!({"type":"tconstruct:material_value_swapping","index":0,"tools":{"item":"tconstruct:travelers_shield"},"material":{"type":"tconstruct:stat_type","stat_type":"tconstruct:shield_core"}})),
        ];
        let recipes = Recipes::load(&files, &Always);
        assert_eq!(recipes.len(), 3, "{:?}", recipes.problems);
        assert_eq!(recipes.problems.len(), 1, "the cost-less value recipe is a problem: {:?}", recipes.problems);
        // Load order is by id: tables/..., then slime_skull/iron, then travelers/shield_wood.
        let Recipe::Tool { tools, extra } = &recipes.recipes()[0] else { panic!("tool recipe first") };
        assert!(extra.is_empty());
        assert!(matches!(tools, Ingredient::Difference { .. }), "the unswappable exclusion survives as a difference ingredient");
        assert_eq!(recipes.recipes()[0].max_stack_size(), 1, "tool swapping is fixed at 1");
        let Recipe::Fixed { repair_value, indices, .. } = &recipes.recipes()[1] else { panic!("fixed recipe second") };
        assert_eq!((*repair_value, indices.as_slice()), (6, [0].as_slice()), "a bare index decodes as a one-element list");
        let Recipe::Value { cost, indices, max_stack_size, .. } = &recipes.recipes()[2] else { panic!("value recipe third") };
        assert_eq!((*cost, indices.as_slice(), *max_stack_size), (2, [0].as_slice(), 16));
    }

    #[test]
    fn defaults_apply_and_invalid_fields_are_problems() {
        let files = vec![
            // No max_stack_size, no repair_value, an index list: defaults 16 and 0.
            (id("tconstruct:a"), json!({"type":"tconstruct:fixed_material_swapping","index":[0,2],"ingredient":{"item":"minecraft:bone"},"material":"tconstruct:bone","tools":{"tag":"tconstruct:modifiable/skulls/swappable"}})),
            (id("tconstruct:b"), json!({"type":"tconstruct:fixed_material_swapping","index":-1,"ingredient":{"item":"minecraft:bone"},"material":"tconstruct:bone","tools":{"tag":"tconstruct:modifiable/skulls/swappable"}})),
            (id("tconstruct:c"), json!({"type":"tconstruct:tool_material_swapping"})),
        ];
        let recipes = Recipes::load(&files, &Always);
        assert_eq!(recipes.len(), 1, "{:?}", recipes.problems);
        assert_eq!(recipes.problems.len(), 2);
        let Recipe::Fixed { repair_value, indices, max_stack_size, .. } = &recipes.recipes()[0] else { panic!() };
        assert_eq!((*repair_value, *max_stack_size), (0, 16));
        assert_eq!(indices.as_slice(), [0, 2]);
    }
}
