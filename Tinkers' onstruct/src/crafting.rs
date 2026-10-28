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

/// Plan a pickaxe material replacement without consuming a part. The caller
/// must validate and atomically consume the actual part stack at commit time.
/// Broken tools whose new maximum exceeds their damage are rejected: upstream
/// preserves the broken flag on rebuild, a state this subset cannot represent.
pub fn plan_part_replacement(
    catalog: &Catalog,
    tool: &ToolState,
    part: &Part,
) -> Result<ToolState, String> {
    catalog.evaluate(tool)?;
    let index = match part.kind {
        PartKind::Head => 0,
        PartKind::Handle => 1,
        PartKind::Binding => 2,
    };
    let mut output = tool.clone();
    output.materials[index] = part.material.clone();
    // Evaluate the new definition independently of the old durability state.
    output.damage = 0;
    output.broken = false;
    let max = catalog.evaluate(&output)?.stats.durability()?;
    if tool.broken && tool.damage < max {
        return Err("broken state after increasing durability is unsupported".into());
    }
    output.damage = tool.damage.min(max);
    output.broken = output.damage == max;
    catalog.evaluate(&output)?;
    Ok(output)
}

/// A single-slot 1:1 material repair plan. No inventory is mutated here.
#[derive(Clone, Debug, PartialEq)]
pub struct RepairPlan {
    pub material_items: u32,
    pub repaired_points: u32,
    pub output: ToolState,
}

/// Plan one Station repair using the pickaxe's repairable head material.
/// Only the selected wood/flint 1:1 recipes are supported. Other ingredients,
/// multiple input slots, repair kits and modifier-dependent recipes are outside
/// this function's contract and must not be treated as matching.
fn repair_per_item(catalog: &Catalog, tool: &ToolState, material: &str) -> Result<f32, String> {
    let evaluation = catalog.evaluate(tool)?;
    let name = material.strip_prefix("tconstruct:").unwrap_or(material);
    let (recipe, stats) = match name {
        "wood" => (
            WOOD,
            include_str!(
                "../third_party/tconstruct/src/generated/resources/data/tconstruct/tinkering/materials/stats/wood.json"
            ),
        ),
        "flint" => (
            FLINT,
            include_str!(
                "../third_party/tconstruct/src/generated/resources/data/tconstruct/tinkering/materials/stats/flint.json"
            ),
        ),
        _ => return Err("unsupported repair material".into()),
    };
    if tool.materials[0] != format!("tconstruct:{name}") {
        return Err("material does not match the repairable head".into());
    }
    let recipe: Value = serde_json::from_str(recipe).map_err(|e| e.to_string())?;
    let stats: Value = serde_json::from_str(stats).map_err(|e| e.to_string())?;
    if recipe["needed"] != 1
        || recipe["value"] != 1
        || recipe["material"] != format!("tconstruct:{name}")
    {
        return Err("unsupported material repair recipe".into());
    }
    let head = stats["stats"]["tconstruct:head"]["durability"]
        .as_u64()
        .ok_or("missing repairable head durability")? as f32;
    let per_item = (head / 3.0) * evaluation.repair_factor(catalog)?;
    if !per_item.is_finite() || per_item <= 0.0 {
        return Err("invalid repair value".into());
    }
    Ok(per_item)
}

pub fn plan_repair(
    catalog: &Catalog,
    tool: &ToolState,
    material: &str,
    available: u32,
) -> Result<RepairPlan, String> {
    if tool.damage == 0 || available == 0 {
        return Err("nothing to repair or no material".into());
    }
    let per_item = repair_per_item(catalog, tool, material)?;
    let used = available.min(((tool.damage as f32 / per_item).ceil() as u32).max(1));
    let points = (used as f32 * per_item) as u32;
    if points == 0 {
        return Err("material would not repair durability".into());
    }
    let mut output = tool.clone();
    let repaired_points = output.repair_points(catalog, points)?;
    Ok(RepairPlan {
        material_items: used,
        repaired_points,
        output,
    })
}

/// Plan a Station repair from the three ordered material slots. Every nonempty
/// stack must match before any result is offered; consumption is recalculated
/// against the actual repaired durability, as in the Station input update.
#[derive(Clone, Debug, PartialEq)]
pub struct MultiRepairPlan {
    pub consumed: [u32; 3],
    pub repaired_points: u32,
    pub output: ToolState,
}

pub fn plan_repair_slots(
    catalog: &Catalog,
    tool: &ToolState,
    slots: [Option<(&str, u32)>; 3],
) -> Result<MultiRepairPlan, String> {
    if tool.damage == 0 {
        return Err("nothing to repair".into());
    }
    let mut material = None;
    for (name, count) in slots.iter().flatten() {
        if *count == 0 {
            return Err("empty repair stack".into());
        }
        let name = name.strip_prefix("tconstruct:").unwrap_or(name);
        if material.is_some_and(|previous| previous != name) {
            return Err("mixed repair materials".into());
        }
        material = Some(name);
    }
    let per_item = repair_per_item(catalog, tool, material.ok_or("no material")?)?;
    let apply = |needed: u32, available: u32| -> (u32, u32) {
        let used = available.min((needed as f32 / per_item).ceil() as u32);
        (used, (used as f32 * per_item) as u32)
    };
    let mut remaining = tool.damage;
    for (_, count) in slots.iter().flatten() {
        if remaining == 0 {
            break;
        }
        remaining = remaining.saturating_sub(apply(remaining, *count).1);
    }
    let points = tool.damage - remaining;
    if points == 0 {
        return Err("material would not repair durability".into());
    }
    let mut output = tool.clone();
    let repaired_points = output.repair_points(catalog, points)?;
    let mut remaining = repaired_points;
    let mut consumed = [0; 3];
    for (index, slot) in slots.iter().enumerate() {
        if remaining == 0 {
            break;
        }
        if let Some((_, count)) = slot {
            let (used, repaired) = apply(remaining, *count);
            consumed[index] = used;
            remaining = remaining.saturating_sub(repaired);
        }
    }
    if remaining != 0 {
        return Err("repair consumption does not cover result".into());
    }
    Ok(MultiRepairPlan {
        consumed,
        repaired_points,
        output,
    })
}
