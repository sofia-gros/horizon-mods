//! Pure recipe plans for the pinned wood/flint pickaxe subset.
//!
//! These functions do not consume items or create game stacks. A future host
//! transaction must recheck every input and apply consumption and output atomically.
use crate::{Catalog, ToolState};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const HEAD: &str = include_str!(
    "../third_party/tconstruct/src/generated/resources/data/tconstruct/recipes/tools/parts/builder/pick_head.json"
);
const HANDLE: &str = include_str!(
    "../third_party/tconstruct/src/generated/resources/data/tconstruct/recipes/tools/parts/builder/tool_handle.json"
);
const BINDING: &str = include_str!(
    "../third_party/tconstruct/src/generated/resources/data/tconstruct/recipes/tools/parts/builder/tool_binding.json"
);
const WOOD: &str = include_str!(
    "../third_party/tconstruct/src/generated/resources/data/tconstruct/recipes/tools/materials/wood/planks/default.json"
);
const FLINT: &str = include_str!(
    "../third_party/tconstruct/src/generated/resources/data/tconstruct/recipes/tools/materials/flint/flint.json"
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartKind {
    Head,
    Handle,
    Binding,
}

impl PartKind {
    fn recipe(self) -> (&'static str, &'static str) {
        match self {
            Self::Head => ("tconstruct:pick_head", HEAD),
            Self::Handle => ("tconstruct:tool_handle", HANDLE),
            Self::Binding => ("tconstruct:tool_binding", BINDING),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    pub kind: PartKind,
    pub material: String,
}

/// A plan only: the pattern is reusable and is not consumed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartPlan {
    pub material_items: u32,
    pub output: Part,
}

/// `material` is already resolved by the caller from the material recipe.
/// `pattern_matches` is the result of checking the pinned recipe's pattern
/// ingredient (default pattern tag or the part's cast). Other material values,
/// remainders and item variants are intentionally unsupported in this subset.
pub fn plan_part(
    catalog: &Catalog,
    kind: PartKind,
    material: &str,
    available: u32,
    pattern_matches: bool,
) -> Result<PartPlan, String> {
    if !pattern_matches {
        return Err("pattern does not match part recipe".into());
    }
    let material = material.strip_prefix("tconstruct:").unwrap_or(material);
    let input = match material {
        "wood" => WOOD,
        "flint" => FLINT,
        _ => return Err("unsupported part material".into()),
    };
    let material_recipe: Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let (part_id, recipe) = kind.recipe();
    let part_recipe: Value = serde_json::from_str(recipe).map_err(|e| e.to_string())?;
    if material_recipe["needed"] != 1
        || material_recipe["value"] != 1
        || material_recipe["material"] != format!("tconstruct:{material}")
        || part_recipe["pattern"] != part_id
        || part_recipe["result"]["item"] != part_id
    {
        return Err("unsupported pinned recipe format".into());
    }
    let cost = part_recipe["cost"]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .filter(|v| *v > 0)
        .ok_or("invalid part cost")?;
    if available < cost {
        return Err("not enough material items".into());
    }
    let state = ToolState::pickaxe([material, material, material]);
    catalog.evaluate(&state)?;
    Ok(PartPlan {
        material_items: cost,
        output: Part {
            kind,
            material: format!("tconstruct:{material}"),
        },
    })
}

/// Validate ordered head, handle and binding, then plan one fresh pickaxe.
/// The caller must atomically consume one of each actual input stack.
pub fn plan_pickaxe(catalog: &Catalog, parts: &[Part; 3]) -> Result<ToolState, String> {
    if parts
        .iter()
        .map(|p| p.kind)
        .ne([PartKind::Head, PartKind::Handle, PartKind::Binding])
    {
        return Err("pickaxe parts are in the wrong slots".into());
    }
    let state = ToolState::pickaxe([&parts[0].material, &parts[1].material, &parts[2].material]);
    catalog.evaluate(&state)?;
    Ok(state)
}
