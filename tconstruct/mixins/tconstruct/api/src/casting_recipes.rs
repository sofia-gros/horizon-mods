//! Casting and molding recipes and the material fluid recipes that tell which fluid casts which material, from SlimeKnights
//! ItemCastingRecipe, RetexturedCastingRecipe, CastDuplicationRecipe, PotionCastingRecipe, ContainerFillingRecipe,
//! MaterialCastingRecipe, CompositeCastingRecipe, MaterialFluidRecipe, MaterialCastingLookup and MoldingRecipe
//! (MIT; see LICENSE). Tool casting, part swapping and potion tipping are not decoded here.
#![allow(dead_code)]
use crate::{
    fluid_ingredient::{id_of, int_field, FluidIngredient, InTag, ItemOutput},
    fluid_values::calc_cooling_time,
    material_recipes::Ingredient,
    melting_recipes::recipe_keys,
    tool_materials::MaterialVariant,
};
use pluto_lib::Identifier;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// The block a casting recipe runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Station { Table, Basin }

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CastingKind {
    /// `casting_table` / `casting_basin`: the fluid, with the cast if there is one, makes the item.
    Item { result: ItemOutput },
    /// `retextured_casting_basin` / `_table`: as Item, and the result takes the texture of the cast block.
    Retextured { result: ItemOutput },
    /// `table_duplication` / `basin_duplication`: the cast comes out again, a copy of itself.
    Duplication,
    /// `casting_table_potion` / `casting_basin_potion`: a bottle takes the potion fluid's NBT and becomes the result item.
    Potion { result: Identifier },
    /// `table_filling` / `basin_filling`: fills the container item with the casting fluid; the amount comes from the container.
    Filling { container: Identifier, fluid_amount: i32 },
    /// `table_casting_material` / `basin_casting_material`: a part of whichever material the fluid casts into.
    Material { result: Identifier, item_cost: i32 },
    /// `table_casting_composite` / `basin_casting_composite`: a part of one material coated in another.
    Composite { result: Identifier, item_cost: i32, stat_conflict: Option<Identifier> },
}

/// A casting recipe: the cast on the station, the fluid poured on it, how long it cools and what comes out.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Casting {
    pub id: Identifier,
    pub station: Station,
    /// The ingredient the item on the station must satisfy; None means the station holds nothing.
    pub cast: Option<Ingredient>,
    pub fluid: Option<FluidIngredient>,
    /// Ticks, for the kinds that fix it; material kinds compute it from the material fluid.
    pub cooling_time: i32,
    pub cast_consumed: bool,
    pub switch_slots: bool,
    pub kind: CastingKind,
}

fn flag(fields: &Map<String, Value>, key: &str) -> Result<bool, String> {
    match fields.get(key) { None => Ok(false), Some(Value::Bool(value)) => Ok(*value), Some(_) => Err(format!("{key} must be a boolean")) }
}

/// A cast ingredient that may be empty (`{}` or `[]`), which means nothing is on the station.
fn optional_ingredient(value: Option<&Value>) -> Result<Option<Ingredient>, String> {
    match value {
        None => Ok(None),
        Some(Value::Array(members)) if members.is_empty() => Ok(None),
        Some(Value::Object(fields)) if fields.is_empty() => Ok(None),
        Some(other) => Ingredient::decode(other).map(Some),
    }
}

fn item_id(fields: &Map<String, Value>, key: &str) -> Result<Identifier, String> { id_of(fields.get(key).ok_or_else(|| format!("{key} missing"))?) }

impl Casting {
    /// Decodes the casting kinds this port reads; any other type is an error naming it.
    pub(crate) fn decode(id: &Identifier, fields: &Map<String, Value>) -> Result<Self, String> {
        let kind_name = fields.get("type").and_then(Value::as_str).ok_or("recipe needs a type")?;
        // `casting_table`, `table_filling`, `retextured_casting_basin`...: the station is the word table or basin, the rest names the kind.
        let name = kind_name.strip_prefix("tconstruct:").ok_or("not a Tinkers recipe")?;
        let station = if name.contains("table") { Station::Table } else if name.contains("basin") { Station::Basin } else { return Err(format!("unimplemented casting recipe type {kind_name}")); };
        let base = name.replace("table_", "").replace("basin_", "").replace("_table", "").replace("_basin", "");
        let make = |cast: Option<Ingredient>, fluid: Option<FluidIngredient>, cooling_time, cast_consumed, switch_slots, kind| Self {
            id: id.clone(), station, cast, fluid, cooling_time, cast_consumed, switch_slots, kind,
        };
        let fluid = || FluidIngredient::decode(fields.get("fluid").ok_or("fluid missing")?);
        match base.as_str() {
            "casting" | "retextured_casting" => {
                recipe_keys(fields, &["cast", "fluid", "result", "cooling_time", "cast_consumed", "switch_slots"])?;
                let result = ItemOutput::decode(fields.get("result").ok_or("result missing")?, false)?;
                let kind = if base == "casting" { CastingKind::Item { result } } else { CastingKind::Retextured { result } };
                Ok(make(optional_ingredient(fields.get("cast"))?, Some(fluid()?), int_field(fields, "cooling_time", 1, None)?, flag(fields, "cast_consumed")?, flag(fields, "switch_slots")?, kind))
            }
            "duplication" => {
                recipe_keys(fields, &["cast", "fluid", "cooling_time"])?;
                let cast = optional_ingredient(fields.get("cast"))?.ok_or("cast must not be empty")?;
                Ok(make(Some(cast), Some(fluid()?), int_field(fields, "cooling_time", 1, None)?, false, false, CastingKind::Duplication))
            }
            "casting_potion" => {
                recipe_keys(fields, &["bottle", "fluid", "result", "cooling_time"])?;
                let bottle = optional_ingredient(fields.get("bottle"))?.ok_or("bottle must not be empty")?;
                Ok(make(Some(bottle), Some(fluid()?), int_field(fields, "cooling_time", 1, Some(5))?, true, false, CastingKind::Potion { result: item_id(fields, "result")? }))
            }
            "filling" => {
                recipe_keys(fields, &["fluid_amount", "container"])?;
                Ok(make(None, None, 5, true, false, CastingKind::Filling { container: item_id(fields, "container")?, fluid_amount: int_field(fields, "fluid_amount", 1, None)? }))
            }
            "casting_material" => {
                recipe_keys(fields, &["cast", "item_cost", "result", "cast_consumed", "switch_slots"])?;
                Ok(make(optional_ingredient(fields.get("cast"))?, None, 1, flag(fields, "cast_consumed")?, flag(fields, "switch_slots")?,
                    CastingKind::Material { result: item_id(fields, "result")?, item_cost: int_field(fields, "item_cost", 1, None)? }))
            }
            "casting_composite" => {
                recipe_keys(fields, &["item_cost", "result", "casting_stat_conflict"])?;
                let result = item_id(fields, "result")?;
                let stat_conflict = match fields.get("casting_stat_conflict") { None | Some(Value::Null) => None, Some(value) => Some(id_of(value)?) };
                // The cast is the part itself, always consumed.
                Ok(make(Some(Ingredient::Item(result.clone())), None, 1, true, false,
                    CastingKind::Composite { result, item_cost: int_field(fields, "item_cost", 1, None)?, stat_conflict }))
            }
            _ => Err(format!("unimplemented casting recipe type {kind_name}")),
        }
    }

    /// Whether the item on the station (None when empty) satisfies the cast; an empty cast wants an empty station.
    pub(crate) fn cast_matches(&self, cast: Option<&Identifier>, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, has_remainder: bool) -> bool {
        match (&self.cast, cast) {
            (None, None) => true,
            (Some(ingredient), Some(item)) => ingredient.matches_with_remainder(item, in_tag, has_remainder),
            _ => false,
        }
    }

    /// matches for the kinds that name their fluid: the cast and the poured fluid (whatever its amount).
    pub(crate) fn matches(&self, cast: Option<&Identifier>, fluid: &Identifier, in_tag: InTag, has_remainder: bool) -> bool {
        self.fluid.as_ref().is_some_and(|wanted| wanted.matches(fluid, in_tag)) && self.cast_matches(cast, in_tag, has_remainder)
    }

    /// getFluidAmount for the kinds that name their fluid.
    pub(crate) fn fluid_amount(&self, fluid: &Identifier, in_tag: InTag) -> i32 {
        self.fluid.as_ref().map_or(0, |wanted| wanted.amount(fluid, in_tag))
    }
}

/// MaterialFluidRecipe: a fluid that casts a material (no input) or coats one material in another (with an input).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MaterialFluid {
    pub id: Identifier,
    pub fluid: FluidIngredient,
    /// Celsius; sets how long casting takes.
    pub temperature: i32,
    pub input: Option<MaterialVariant>,
    /// The unknown material when the file names none.
    pub output: MaterialVariant,
    pub hide_in_book: bool,
}

impl MaterialFluid {
    pub(crate) fn decode(id: &Identifier, fields: &Map<String, Value>) -> Result<Self, String> {
        recipe_keys(fields, &["fluid", "temperature", "input", "output", "hide_in_book"])?;
        let variant = |key: &str| match fields.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(text)) => MaterialVariant::parse(text).map(Some).ok_or_else(|| format!("invalid material variant {text}")),
            Some(_) => Err(format!("{key} must be a material ID")),
        };
        Ok(Self {
            id: id.clone(),
            fluid: FluidIngredient::decode(fields.get("fluid").ok_or("fluid missing")?)?,
            temperature: int_field(fields, "temperature", 0, None)?,
            input: variant("input")?,
            output: variant("output")?.unwrap_or_else(MaterialVariant::unknown),
            hide_in_book: flag(fields, "hide_in_book")?,
        })
    }
    /// matches(Fluid): a known output and a fluid the ingredient names.
    pub(crate) fn matches_fluid(&self, fluid: &Identifier, in_tag: InTag) -> bool {
        self.output != MaterialVariant::unknown() && self.fluid.matches(fluid, in_tag)
    }
    /// matches(Fluid, MaterialVariantId): also the input material, and never casting a material into itself.
    pub(crate) fn matches_material(&self, fluid: &Identifier, material: &MaterialVariant, in_tag: InTag) -> bool {
        self.matches_fluid(fluid, in_tag) && self.input.as_ref().is_none_or(|input| input.matches(material)) && &self.output != material
    }
}

/// MaterialCastingLookup: which fluids cast which materials, and what each material part costs.
#[derive(Clone, Debug, Default)]
pub(crate) struct MaterialCasting {
    /// Fluids that cast a material, in recipe order; the first match wins.
    pub casting: Vec<MaterialFluid>,
    /// Fluids that coat a material in another.
    pub composite: Vec<MaterialFluid>,
    costs: BTreeMap<Identifier, i32>,
}

impl MaterialCasting {
    pub(crate) fn add_fluid(&mut self, recipe: MaterialFluid) {
        if recipe.input.is_none() { self.casting.push(recipe); } else { self.composite.push(recipe); }
    }
    /// registerItemCost: two recipes that disagree keep the smaller cost.
    pub(crate) fn register_item_cost(&mut self, item: &Identifier, cost: i32) -> bool {
        match self.costs.get_mut(item) {
            Some(existing) if *existing != cost => { *existing = (*existing).min(cost); false }
            Some(_) => true,
            None => { self.costs.insert(item.clone(), cost); true }
        }
    }
    /// getItemCost: 0 for an item that is not a casting part.
    pub(crate) fn item_cost(&self, item: &Identifier) -> i32 { self.costs.get(item).copied().unwrap_or(0) }
    pub(crate) fn casting_fluid(&self, fluid: &Identifier, in_tag: InTag) -> Option<&MaterialFluid> {
        self.casting.iter().find(|recipe| recipe.matches_fluid(fluid, in_tag))
    }
    pub(crate) fn composite_fluid(&self, fluid: &Identifier, material: &MaterialVariant, in_tag: InTag) -> Option<&MaterialFluid> {
        self.composite.iter().find(|recipe| recipe.matches_material(fluid, material, in_tag))
    }
    /// The recipe fluid for a material casting of `kind` poured with `fluid` over the item on the station (its material, if any).
    /// A composite recipe with a casting stat conflict finds nothing while the fluid already casts a material the stat uses.
    pub(crate) fn fluid_recipe(&self, kind: &CastingKind, fluid: &Identifier, stack_material: Option<&MaterialVariant>, in_tag: InTag,
        can_cast_with_stat: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&MaterialFluid> {
        match kind {
            CastingKind::Material { .. } => self.casting_fluid(fluid, in_tag),
            CastingKind::Composite { stat_conflict, .. } => {
                if let Some(stat) = stat_conflict {
                    if let Some(recipe) = self.casting_fluid(fluid, in_tag) {
                        if recipe.output.material_id().is_some_and(|material| can_cast_with_stat(stat, &material)) { return None; }
                    }
                }
                self.composite_fluid(fluid, stack_material?, in_tag)
            }
            _ => None,
        }
    }
    /// getFluidAmount of a material casting: the fluid's amount times the part's item cost.
    pub(crate) fn fluid_amount(recipe: &MaterialFluid, item_cost: i32, fluid: &Identifier, in_tag: InTag) -> i32 {
        recipe.fluid.amount(fluid, in_tag).wrapping_mul(item_cost)
    }
    /// getCoolingTime of a material casting.
    pub(crate) fn cooling_time(recipe: &MaterialFluid, item_cost: i32, fluid: &Identifier, in_tag: InTag) -> i32 {
        calc_cooling_time(recipe.temperature, Self::fluid_amount(recipe, item_cost, fluid, in_tag))
    }
}

/// MoldingRecipe: a material item on the table, with an optional pattern, becomes the result (sand casts, clay casts).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Molding { pub table: bool, pub id: Identifier, pub material: Ingredient, pub pattern: Option<Ingredient>, pub pattern_consumed: bool, pub result: ItemOutput }

impl Molding {
    pub(crate) fn decode(id: &Identifier, fields: &Map<String, Value>) -> Result<Self, String> {
        recipe_keys(fields, &["material", "pattern", "pattern_consumed", "result"])?;
        let pattern = optional_ingredient(fields.get("pattern"))?;
        let consumed = flag(fields, "pattern_consumed")?;
        Ok(Self {
            table: true,
            id: id.clone(),
            material: Ingredient::decode(fields.get("material").ok_or("material missing")?)?,
            // a recipe with no pattern never consumes one
            pattern_consumed: pattern.is_some() && consumed,
            pattern,
            result: ItemOutput::decode(fields.get("result").ok_or("result missing")?, false)?,
        })
    }
    pub(crate) fn matches(&self, material: &Identifier, pattern: Option<&Identifier>, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        self.material.matches(material, in_tag) && match (&self.pattern, pattern) {
            (None, None) => true,
            (Some(wanted), Some(item)) => wanted.matches(item, in_tag),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn fields(value: Value) -> Map<String, Value> { value.as_object().unwrap().clone() }
    fn none(_: &Identifier, _: &Identifier) -> bool { false }

    fn iron_casting() -> MaterialCasting {
        let mut lookup = MaterialCasting::default();
        lookup.add_fluid(MaterialFluid::decode(&id("a:iron"), &fields(json!({"type": "tconstruct:material_fluid", "temperature": 800,
            "fluid": {"amount": 90, "fluid": "a:molten_iron"}, "output": "tconstruct:iron"}))).unwrap());
        lookup.add_fluid(MaterialFluid::decode(&id("a:coat"), &fields(json!({"type": "tconstruct:material_fluid", "temperature": 800,
            "fluid": {"amount": 90, "fluid": "a:molten_iron"}, "input": "tconstruct:wood", "output": "tconstruct:iron_wood"}))).unwrap());
        lookup
    }

    #[test]
    fn a_material_casting_costs_the_part_cost_times_the_fluid_and_cools_by_that_amount() {
        let mut lookup = iron_casting();
        assert!(lookup.register_item_cost(&id("a:pick_head"), 2));
        assert!(!lookup.register_item_cost(&id("a:pick_head"), 3), "an inconsistent cost is reported");
        assert_eq!(lookup.item_cost(&id("a:pick_head")), 2, "and the smaller one is kept");
        assert_eq!(lookup.item_cost(&id("a:stick")), 0);
        let part = CastingKind::Material { result: id("a:pick_head"), item_cost: 2 };
        let recipe = lookup.fluid_recipe(&part, &id("a:molten_iron"), None, &none, &|_, _| false).unwrap();
        assert_eq!(recipe.output.material_id(), Some(id("tconstruct:iron")));
        assert_eq!(MaterialCasting::fluid_amount(recipe, 2, &id("a:molten_iron"), &none), 180);
        // 800 C and two ingots: sqrt(2) * 1100^0.585 = 85
        assert_eq!(MaterialCasting::cooling_time(recipe, 2, &id("a:molten_iron"), &none), 85);
        assert!(lookup.fluid_recipe(&part, &id("a:water"), None, &none, &|_, _| false).is_none());
    }

    #[test]
    fn composite_casting_needs_the_input_material_and_defers_to_a_stat_conflict() {
        let lookup = iron_casting();
        let wood = MaterialVariant::parse("tconstruct:wood").unwrap();
        let stone = MaterialVariant::parse("tconstruct:stone").unwrap();
        let composite = CastingKind::Composite { result: id("a:pick_head"), item_cost: 2, stat_conflict: Some(id("tconstruct:head")) };
        let iron = id("a:molten_iron");
        assert!(lookup.fluid_recipe(&composite, &iron, Some(&wood), &none, &|_, _| false).is_some());
        assert!(lookup.fluid_recipe(&composite, &iron, Some(&stone), &none, &|_, _| false).is_none(), "stone is not the input");
        assert!(lookup.fluid_recipe(&composite, &iron, None, &none, &|_, _| false).is_none(), "an item with no material has nothing to coat");
        // iron already casts a head material, so the composite recipe yields to tool casting
        assert!(lookup.fluid_recipe(&composite, &iron, Some(&wood), &none, &|stat, material| stat.path() == "head" && material.path() == "iron").is_none());
        // a material is never cast into itself
        let coat = MaterialFluid::decode(&id("a:self"), &fields(json!({"type": "tconstruct:material_fluid", "temperature": 800,
            "fluid": {"amount": 90, "fluid": "a:molten_iron"}, "input": "a:x", "output": "a:x#coated"}))).unwrap();
        assert!(coat.matches_material(&iron, &MaterialVariant::parse("a:x").unwrap(), &none));
        assert!(!coat.matches_material(&iron, &MaterialVariant::parse("a:x#coated").unwrap(), &none));
    }

    #[test]
    fn item_casting_needs_the_cast_the_fluid_and_decodes_every_station_form() {
        let table = Casting::decode(&id("a:ingot"), &fields(json!({"type": "tconstruct:casting_table", "cast": {"tag": "tconstruct:casts/multi_use/ingot"},
            "cooling_time": 60, "fluid": {"amount": 90, "fluid": "a:molten_iron"}, "result": {"item": "a:iron_ingot"}}))).unwrap();
        assert_eq!(table.station, Station::Table);
        let cast = |item: &Identifier, tag: &Identifier| item.path() == "gold_cast" && tag.path() == "casts/multi_use/ingot";
        assert!(table.matches(Some(&id("a:gold_cast")), &id("a:molten_iron"), &none, false) == false, "no tag membership");
        assert!(table.matches(Some(&id("a:gold_cast")), &id("a:molten_iron"), &cast, false));
        assert!(!table.matches(None, &id("a:molten_iron"), &cast, false), "the cast is missing");
        assert_eq!(table.fluid_amount(&id("a:molten_iron"), &none), 90);
        // no cast: the station must be empty
        let block = Casting::decode(&id("a:block"), &fields(json!({"type": "tconstruct:casting_basin", "cooling_time": 180,
            "fluid": {"amount": 810, "fluid": "a:molten_iron"}, "result": "a:iron_block"}))).unwrap();
        assert!(block.station == Station::Basin && block.matches(None, &id("a:molten_iron"), &none, false) && !block.matches(Some(&id("a:gold_cast")), &id("a:molten_iron"), &none, false));
        let duplicate = Casting::decode(&id("a:dup"), &fields(json!({"type": "tconstruct:table_duplication", "cast": [{"item": "a:template"}],
            "cooling_time": 186, "fluid": {"amount": 500, "fluid": "a:diamond"}}))).unwrap();
        assert_eq!(duplicate.kind, CastingKind::Duplication);
        let potion = Casting::decode(&id("a:potion"), &fields(json!({"type": "tconstruct:casting_table_potion", "bottle": {"item": "a:bottle"},
            "fluid": {"amount": 250, "fluid": "a:potion"}, "result": "a:potion_item"}))).unwrap();
        assert_eq!((potion.cooling_time, potion.cast_consumed), (5, true));
        let fill = Casting::decode(&id("a:fill"), &fields(json!({"type": "tconstruct:table_filling", "container": "a:bucket", "fluid_amount": 1000}))).unwrap();
        assert_eq!(fill.kind, CastingKind::Filling { container: id("a:bucket"), fluid_amount: 1000 });
        for bad in [json!({"type": "tconstruct:casting_table", "cooling_time": 0, "fluid": {"amount": 1, "fluid": "a:b"}, "result": "a:c"}),
            json!({"type": "tconstruct:casting_table", "cooling_time": 1, "fluid": {"amount": 1, "fluid": "a:b"}, "result": "a:c", "allowed_materials": {}}),
            json!({"type": "tconstruct:table_tool_casting", "cast": {"item": "a:b"}})] {
            assert!(Casting::decode(&id("a:bad"), &fields(bad.clone())).is_err(), "{bad}");
        }
    }

    #[test]
    fn molding_with_no_pattern_needs_an_empty_pattern_slot_and_consumes_nothing() {
        let sand = Molding::decode(&id("a:sand"), &fields(json!({"type": "tconstruct:molding_table", "material": {"tag": "a:sand_casts"},
            "pattern_consumed": true, "result": "a:blank_sand_cast"}))).unwrap();
        assert!(!sand.pattern_consumed, "no pattern, so none to consume");
        let sand_cast = |item: &Identifier, tag: &Identifier| item.path() == "sand_cast" && tag.path() == "sand_casts";
        assert!(sand.matches(&id("a:sand_cast"), None, &sand_cast) && !sand.matches(&id("a:sand_cast"), Some(&id("a:pattern")), &sand_cast));
        let patterned = Molding::decode(&id("a:p"), &fields(json!({"type": "tconstruct:molding_table", "material": {"item": "a:clay"},
            "pattern": {"item": "a:pattern"}, "pattern_consumed": true, "result": "a:out"}))).unwrap();
        assert!(patterned.pattern_consumed && patterned.matches(&id("a:clay"), Some(&id("a:pattern")), &none));
    }
}
