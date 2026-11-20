//! Loads the smeltery domain's recipe files: melting, ore and damageable melting, material melting, alloys, melting fuel,
//! entity melting, casting, material fluids and molding (SlimeKnights Tinkers' Construct, MIT; see LICENSE).
//! The recipe manager of the original keeps recipes in an unordered map; this loader keeps them in file ID order, so the
//! first match is deterministic. Recipe types that need the tool domain are listed, not decoded.
#![allow(dead_code)]
use crate::{
    casting_recipes::{Casting, MaterialCasting, MaterialFluid, Molding},
    fluid_ingredient::FluidTags,
    materials::Conditions,
    melting_recipes::{Alloy, EntityMelting, Melting, MeltingFuel, MaterialMelting},
    tool_materials::MaterialVariant,
};
use pluto_lib::Identifier;
use serde_json::{Map, Value};

pub(crate) const FOLDER: &str = "recipes/fluids";

/// Types this loader decodes into one of its recipe lists.
const MELTING: &[&str] = &["tconstruct:melting", "tconstruct:ore_melting", "tconstruct:damagable_melting"];
/// Types of the smeltery domain that depend on tool definitions, materials' part stats or potion NBT
/// (ToolCastingRecipe, PartSwapCastingRecipe, TippingCastingRecipe, TipClearingCastingRecipe): they are left out, by name.
pub(crate) const LEFT_OUT: &[&str] = &[
    "tconstruct:basin_tool_casting", "tconstruct:table_tool_casting",
    "tconstruct:basin_casting_part_swapping", "tconstruct:table_casting_part_swapping",
    "tconstruct:casting_table_tipping", "tconstruct:casting_basin_tipping",
    "tconstruct:casting_table_tipped_clearing", "tconstruct:casting_basin_tipped_clearing",
];

#[derive(Clone, Debug, Default)]
pub(crate) struct Recipes {
    pub melting: Vec<Melting>,
    pub material_melting: Vec<MaterialMelting>,
    /// Fluid fuels, and the one solid fuel recipe (MeltingFuelLookup).
    pub fuels: Vec<MeltingFuel>,
    pub solid_fuel: Option<MeltingFuel>,
    pub entity_melting: Vec<EntityMelting>,
    pub alloys: Vec<Alloy>,
    pub casting: Vec<Casting>,
    pub molding: Vec<Molding>,
    pub material_casting: MaterialCasting,
    /// Recipes of a type listed in `LEFT_OUT`, by ID and type.
    pub left_out: Vec<(Identifier, String)>,
    /// The cast pattern recipes of the part builder (`tconstruct:item_part_builder`, the sand and clay casts in the smeltery's
    /// folder): part_builder_recipes decodes those; they are only recorded here.
    pub part_builder: Vec<Identifier>,
    pub problems: Vec<String>,
}

/// The recipe a file stands for: checks its top-level conditions, and for `forge:conditional` picks the first entry whose
/// conditions hold (None when none does). Ok(None) means the recipe is switched off.
pub(crate) fn effective<'a, C: Conditions>(value: &'a Value, conditions: &C) -> Result<Option<&'a Map<String, Value>>, String> {
    let fields = value.as_object().ok_or("recipe must be an object")?;
    // Every condition is decoded; only those that can still change the outcome are evaluated.
    let passes = |list: Option<&Value>, evaluate: bool| -> Result<bool, String> {
        let Some(list) = list else { return Ok(true) };
        let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
        if !evaluate { return Ok(false); }
        for condition in &parsed { if !conditions.test(condition)? { return Ok(false); } }
        Ok(true)
    };
    if !passes(fields.get("conditions"), true)? { return Ok(None); }
    if fields.get("type").and_then(Value::as_str) != Some("forge:conditional") { return Ok(Some(fields)); }
    let entries = fields.get("recipes").and_then(Value::as_array).ok_or("conditional recipe needs recipes")?;
    // every entry's conditions are decoded before any is chosen, so a malformed later entry is still reported
    let mut chosen = None;
    for entry in entries {
        let entry = entry.as_object().ok_or("conditional entry must be an object")?;
        let inner = entry.get("recipe").and_then(Value::as_object).ok_or("conditional entry needs a recipe")?;
        if passes(entry.get("conditions"), chosen.is_none())? && chosen.is_none() { chosen = Some(inner); }
    }
    Ok(chosen)
}

impl Recipes {
    pub(crate) fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self::default();
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            match effective(value, conditions).and_then(|fields| fields.map_or(Ok(()), |fields| result.add(id, fields))) {
                Ok(()) => {}
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }

    fn add(&mut self, id: &Identifier, fields: &Map<String, Value>) -> Result<(), String> {
        let kind = fields.get("type").and_then(Value::as_str).ok_or("recipe needs a type")?;
        match kind {
            kind if MELTING.contains(&kind) => self.melting.push(Melting::decode(id, fields)?),
            "tconstruct:material_melting" => self.material_melting.push(MaterialMelting::decode(id, fields)?),
            "tconstruct:melting_fuel" => {
                let fuel = MeltingFuel::decode(id, fields)?;
                if fuel.fluid.is_some() {
                    self.fuels.push(fuel);
                } else if self.solid_fuel.is_none() {
                    self.solid_fuel = Some(fuel);
                } else {
                    return Err("a second solid fuel recipe; the first one is kept".into());
                }
            }
            "tconstruct:entity_melting" => self.entity_melting.push(EntityMelting::decode(id, fields)?),
            "tconstruct:alloy" => self.alloys.push(Alloy::decode(id, fields)?),
            "tconstruct:material_fluid" => self.material_casting.add_fluid(MaterialFluid::decode(id, fields)?),
            "tconstruct:molding_table" | "tconstruct:molding_basin" => {
                let mut molding = Molding::decode(id, fields)?;
                molding.table = kind == "tconstruct:molding_table";
                self.molding.push(molding);
            }
            "tconstruct:item_part_builder" => self.part_builder.push(id.clone()),
            kind if LEFT_OUT.contains(&kind) => self.left_out.push((id.clone(), kind.to_owned())),
            kind if kind.starts_with("tconstruct:") && (kind.contains("casting") || kind.contains("duplication") || kind.contains("filling")) => {
                let recipe = Casting::decode(id, fields)?;
                // MaterialCastingRecipe and CompositeCastingRecipe register their part's cost as they are built.
                if let crate::casting_recipes::CastingKind::Material { result, item_cost } | crate::casting_recipes::CastingKind::Composite { result, item_cost, .. } = &recipe.kind {
                    if !self.material_casting.register_item_cost(result, *item_cost) {
                        self.problems.push(format!("{id}: inconsistent cost for item {result}"));
                    }
                }
                self.casting.push(recipe);
            }
            other => return Err(format!("unimplemented recipe type {other}")),
        }
        Ok(())
    }

    pub(crate) fn len(&self) -> usize {
        self.melting.len() + self.material_melting.len() + self.fuels.len() + usize::from(self.solid_fuel.is_some()) + self.entity_melting.len()
            + self.alloys.len() + self.casting.len() + self.molding.len() + self.part_builder.len() + self.material_casting.casting.len() + self.material_casting.composite.len()
    }

    /// The first melting recipe whose ingredient the item satisfies, in ID order (RecipeManager.getRecipeFor).
    pub(crate) fn melting_count(&self) -> usize { self.melting.len() }

    pub(crate) fn find_melting(&self, item: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, has_remainder: bool) -> Option<&Melting> {
        self.melting.iter().find(|recipe| recipe.matches(item, in_tag, has_remainder))
    }

    /// The first material melting recipe for a casting part of the material, and the part's casting cost (MaterialMeltingRecipe.matches).
    pub(crate) fn find_material_melting(&self, item: &Identifier, material: &MaterialVariant) -> Option<(&MaterialMelting, i32)> {
        let cost = self.material_casting.item_cost(item);
        self.material_melting.iter().find(|recipe| recipe.matches(cost, material)).map(|recipe| (recipe, cost))
    }

    /// MeltingFuelLookup.findFuel: the first fluid fuel that takes the fluid.
    pub(crate) fn find_fuel(&self, fluid: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&MeltingFuel> {
        self.fuels.iter().find(|fuel| fuel.matches(fluid, in_tag))
    }

    /// Every alloy recipe whose inputs are all present in the tank, in ID order (the original shuffles them).
    pub(crate) fn matching_alloys(&self, tank: &dyn crate::melting_recipes::AlloyTank, tags: &FluidTags) -> Vec<&Alloy> {
        self.alloys.iter().filter(|alloy| alloy.matches(tank, &|fluid, tag| tags.contains(fluid, tag))).collect()
    }
}
