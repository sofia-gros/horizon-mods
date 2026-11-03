// Parse the source melting and casting recipe forms used by the melter.
// Registry tags and recipe conditions are resolved by the caller against the
// loaded game data, rather than guessed from a legacy item list.
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Ingredient { Item(String), Tag(String), Any(Vec<Ingredient>), All(Vec<Ingredient>), Difference(Box<Ingredient>, Box<Ingredient>) }

impl Ingredient {
    pub(super) fn parse(value: &Value) -> Option<Self> {
        if let Some(values) = value.as_array() {
            let choices: Vec<_> = values.iter().map(Self::parse).collect::<Option<_>>()?;
            return (!choices.is_empty()).then_some(Self::Any(choices));
        }
        let obj = value.as_object()?;
        if obj.get("type").and_then(Value::as_str) == Some("forge:intersection") {
            if obj.keys().any(|key| key != "type" && key != "children") { return None; }
            let children = obj.get("children")?.as_array()?.iter().map(Self::parse).collect::<Option<Vec<_>>>()?;
            return (!children.is_empty()).then_some(Self::All(children));
        }
        if obj.get("type").and_then(Value::as_str) == Some("forge:difference") {
            if obj.keys().any(|key| key != "type" && key != "base" && key != "subtracted") { return None; }
            return Some(Self::Difference(Box::new(Self::parse(obj.get("base")?)?), Box::new(Self::parse(obj.get("subtracted")?)?)));
        }
        // With the default ANY predicate the source material ingredient only
        // tests its nested item or tag, without reading material NBT.
        if obj.get("type").and_then(Value::as_str) == Some("tconstruct:material") {
            if obj.keys().any(|key| key != "type" && key != "item" && key != "tag") { return None; }
            return match (obj.get("item"), obj.get("tag")) {
                (Some(id), None) => Some(Self::Item(id.as_str()?.to_owned())),
                (None, Some(id)) => Some(Self::Tag(id.as_str()?.to_owned())),
                _ => None,
            };
        }
        // NBT/custom ingredients need their own semantics; do not silently
        // accept them as a plain item or tag.
        if obj.keys().any(|key| key != "item" && key != "tag") { return None; }
        match (obj.get("item"), obj.get("tag")) {
            (Some(id), None) => Some(Self::Item(id.as_str()?.to_owned())),
            (None, Some(id)) => Some(Self::Tag(id.as_str()?.to_owned())),
            _ => None,
        }
    }
    pub(super) fn matches(&self, item: &str, in_tag: &impl Fn(&str, &str) -> bool) -> bool {
        match self {
            Self::Item(id) => id == item,
            Self::Tag(tag) => in_tag(tag, item),
            Self::Any(choices) => choices.iter().any(|choice| choice.matches(item, in_tag)),
            Self::All(choices) => choices.iter().all(|choice| choice.matches(item, in_tag)),
            Self::Difference(base, removed) => base.matches(item, in_tag) && !removed.matches(item, in_tag),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FluidSpec { pub id: String, pub tagged: bool, pub amount: u32 }
impl FluidSpec {
    fn parse(value: &Value) -> Option<Self> {
        let obj = value.as_object()?;
        if obj.keys().any(|key| key != "fluid" && key != "tag" && key != "amount") { return None; }
        let (id, tagged) = match (obj.get("fluid"), obj.get("tag")) {
            (Some(id), None) => (id.as_str()?, false),
            (None, Some(id)) => (id.as_str()?, true),
            _ => return None,
        };
        let amount = u32::try_from(obj.get("amount")?.as_u64()?).ok()?;
        (amount > 0).then(|| Self { id: id.to_owned(), tagged, amount })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AlloyRecipe {
    pub inputs: Vec<FluidSpec>, pub catalysts: Vec<bool>, pub output: FluidSpec, pub temperature: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FluidFuel {
    pub fluid: FluidSpec, pub duration: u32, pub temperature: u32, pub rate: u32,
}
impl FluidFuel {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let v: Value = serde_json::from_slice(bytes).ok()?;
        if v.get("type")?.as_str()? != "tconstruct:melting_fuel" || !enabled(&v, conditions) { return None; }
        let positive = |key: &str| -> Option<u32> {
            let value = u32::try_from(v.get(key)?.as_u64()?).ok()?;
            (value > 0).then_some(value)
        };
        Some(Self { fluid: FluidSpec::parse(v.get("fluid")?)?, duration: positive("duration")?,
            temperature: positive("temperature")?, rate: positive("rate")? })
    }
}
impl AlloyRecipe {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let v: Value = serde_json::from_slice(bytes).ok()?;
        if v.get("type")?.as_str()? != "tconstruct:alloy" || !enabled(&v, conditions) { return None; }
        let mut inputs = Vec::new();
        let mut catalysts = Vec::new();
        for entry in v.get("inputs")?.as_array()? {
            let mut object = entry.as_object()?.clone();
            let catalyst = match object.remove("catalyst") { None => false, Some(value) => value.as_bool()? };
            inputs.push(FluidSpec::parse(&Value::Object(object))?);
            catalysts.push(catalyst);
        }
        if inputs.len() < 2 { return None; }
        let temperature = u32::try_from(v.get("temperature")?.as_u64()?).ok()?;
        if temperature == 0 { return None; }
        Some(Self { inputs, catalysts, output: FluidSpec::parse(v.get("result")?)?, temperature })
    }
}

#[cfg(test)]
mod alloy_tests {
    use super::*;
    #[test]
    fn parses_source_bronze_and_honors_conditions() {
        let source = br#"{"type":"tconstruct:alloy","conditions":[{"type":"mantle:tag_filled","tag":"forge:ingots/tin"}],"inputs":[{"amount":270,"tag":"forge:molten_copper"},{"amount":90,"tag":"forge:molten_tin"}],"result":{"amount":360,"tag":"forge:molten_bronze"},"temperature":700}"#;
        let recipe = AlloyRecipe::parse(source, &|_| true).unwrap();
        assert_eq!(recipe.inputs.iter().map(|input| input.amount).collect::<Vec<_>>(), [270, 90]);
        assert_eq!((recipe.output.amount, recipe.temperature), (360, 700));
        assert!(AlloyRecipe::parse(source, &|_| false).is_none());
    }
    #[test]
    fn parses_lava_fuel_and_rejects_solid_fuel() {
        let lava = br#"{"type":"tconstruct:melting_fuel","duration":100,"fluid":{"amount":50,"fluid":"minecraft:lava"},"rate":10,"temperature":1000}"#;
        let fuel = FluidFuel::parse(lava, &|_| true).unwrap();
        assert_eq!((fuel.fluid.id.as_str(), fuel.fluid.amount, fuel.duration, fuel.rate, fuel.temperature), ("minecraft:lava", 50, 100, 10, 1000));
        assert!(FluidFuel::parse(br#"{"type":"tconstruct:melting_fuel","temperature":800,"rate":5}"#, &|_| true).is_none());
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MeltingRecipe {
    pub input: Ingredient, pub output: FluidSpec, pub temperature: u32, pub time: u32,
}
impl MeltingRecipe {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let v: Value = serde_json::from_slice(bytes).ok()?;
        let recipe_type = v.get("type")?.as_str()?;
        if !matches!(recipe_type, "tconstruct:melting" | "tconstruct:ore_melting") || !enabled(&v, conditions) { return None; }
        // The source Melter uses MeltingModuleInventory, not its foundry
        // subclass: byproducts are not collected. Ore rates are the source
        // default Melter settings (12 nuggets / 8 gem shards).
        let mut output = FluidSpec::parse(v.get("result")?)?;
        if recipe_type == "tconstruct:ore_melting" {
            output.amount = match v.get("rate")?.as_str()? {
                "metal" => output.amount.checked_mul(12)? / 9,
                "gem" => output.amount.checked_mul(8)? / 4,
                "none" | "default" => output.amount,
                _ => return None,
            };
            if output.amount == 0 { return None; }
        }
        let time = u32::try_from(v.get("time")?.as_u64()?).ok()?;
        let temperature = u32::try_from(v.get("temperature")?.as_u64()?).ok()?;
        if time == 0 || time.checked_mul(10).is_none_or(|required| required > i32::MAX as u32) { return None; }
        Some(Self { input: Ingredient::parse(v.get("ingredient")?)?, output, temperature, time })
    }
}

pub(super) fn material_melting(bytes: &[u8], item: &str, material: &str, cost: u32, conditions: &impl Fn(&[Value]) -> bool) -> Option<FoundryRecipe> {
    let mut value: Value = serde_json::from_slice(bytes).ok()?;
    if value.get("type")?.as_str()? != "tconstruct:material_melting" || !enabled(&value, conditions) || cost == 0 { return None; }
    let wanted = value.get("input")?.as_str()?;
    let (base, variant) = wanted.split_once('#').unwrap_or((wanted, ""));
    let (actual_base, actual_variant) = material.split_once('#').unwrap_or((material, ""));
    if base != actual_base || !(variant.is_empty() || variant == actual_variant || variant == "default" && actual_variant.is_empty()) { return None; }
    let object = value.as_object_mut()?;
    let result = object.get_mut("result")?.as_object_mut()?;
    let amount = u32::try_from(result.get("amount")?.as_u64()?).ok()?.checked_mul(cost)?;
    if amount == 0 || amount > i32::MAX as u32 { return None; }
    result.insert("amount".into(), Value::from(amount));
    if let Some(byproducts) = object.get_mut("byproducts") {
        for fluid in byproducts.as_array_mut()? {
            let fluid = fluid.as_object_mut()?;
            let amount = u32::try_from(fluid.get("amount")?.as_u64()?).ok()?.checked_mul(cost)?;
            if amount == 0 || amount > i32::MAX as u32 { return None; }
            fluid.insert("amount".into(), Value::from(amount));
        }
    }
    let temperature = u32::try_from(object.get("temperature")?.as_u64()?).ok()?;
    let factor = ((amount as f32) / 90.0_f32).sqrt();
    let time = (((temperature as f64 + 300.0).powf(0.585_f32 as f64)) * factor as f64).round();
    if !(1.0..=u32::MAX as f64).contains(&time) { return None; }
    object.insert("time".into(), Value::from(time as u32));
    object.insert("ingredient".into(), serde_json::json!({"item": item}));
    object.insert("type".into(), Value::String("tconstruct:melting".into()));
    FoundryRecipe::parse(&serde_json::to_vec(&value).ok()?, conditions)
}

/// Source damageable melting uses one unit minimum and Java int arithmetic.
pub(super) fn damaged_melting(bytes: &[u8], damage: u32, maximum: u32, conditions: &impl Fn(&[Value]) -> bool) -> Option<FoundryRecipe> {
    let mut value: Value = serde_json::from_slice(bytes).ok()?;
    if value.get("type")?.as_str()? != "tconstruct:damagable_melting" { return None; }
    let object = value.as_object_mut()?;
    object.insert("type".into(), Value::String("tconstruct:melting".into()));
    fn take_unit(value: &mut Value) -> Option<u32> {
        let unit = match value.as_object_mut()?.remove("unit_size") { None => 1, Some(v) => u32::try_from(v.as_u64()?).ok()? };
        (unit > 0 && unit <= i32::MAX as u32).then_some(unit)
    }
    let unit = take_unit(object.get_mut("result")?)?;
    let units = match object.get_mut("byproducts") {
        None => Vec::new(),
        Some(v) => v.as_array_mut()?.iter_mut().map(take_unit).collect::<Option<Vec<_>>>()?,
    };
    let mut recipe = FoundryRecipe::parse(&serde_json::to_vec(&value).ok()?, conditions)?;
    let scale = |amount: u32, unit: u32| -> Option<u32> {
        let amount = i32::try_from(amount).ok()?;
        let unit = i32::try_from(unit).ok()?;
        if maximum == 0 { return u32::try_from(amount).ok(); }
        let max = i32::try_from(maximum).ok()?;
        let damage = i32::try_from(damage).ok()?;
        let scaled = amount.wrapping_mul(max.wrapping_sub(damage)) / max;
        let scaled = if scaled <= unit { unit } else { scaled - scaled % unit };
        u32::try_from(scaled).ok()
    };
    recipe.melting.output.amount = scale(recipe.melting.output.amount, unit)?;
    for (fluid, unit) in recipe.byproducts.iter_mut().zip(units) { fluid.amount = scale(fluid.amount, unit)?; }
    Some(recipe)
}

/// Foundries use the source's separate ore and byproduct rates. Keep these
/// outputs distinct so the caller can insert the primary fluid first and
/// then fill byproducts in their original order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FoundryRecipe {
    pub melting: MeltingRecipe,
    pub byproducts: Vec<FluidSpec>,
}
impl FoundryRecipe {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let value: Value = serde_json::from_slice(bytes).ok()?;
        let mut melting = MeltingRecipe::parse(bytes, conditions)?;
        let ore = value.get("type")?.as_str()? == "tconstruct:ore_melting";
        let main_rate = if ore { value.get("rate")?.as_str()? } else { "none" };
        let adjust = |amount: u32, rate: &str, byproduct: bool| -> Option<u32> {
            let (numerator, denominator) = match rate {
                "metal" => (if byproduct { 3 } else { 9 }, 9),
                "gem" => (4, 4),
                "none" | "default" => (1, 1),
                _ => return None,
            };
            amount.checked_mul(numerator).map(|n| n / denominator).filter(|n| *n > 0)
        };
        let mut primary = FluidSpec::parse(value.get("result")?)?;
        primary.amount = adjust(primary.amount, main_rate, false)?;
        melting.output = primary;
        let mut byproducts = Vec::new();
        if let Some(raw) = value.get("byproducts") {
            for entry in raw.as_array()? {
                let mut object = entry.as_object()?.clone();
                let raw_rate = object.remove("rate");
                let rate = match raw_rate.as_ref().map(Value::as_str) {
                    None => main_rate,
                    Some(Some("default")) => main_rate,
                    Some(Some(rate)) if ore => rate,
                    _ => return None,
                };
                let mut fluid = FluidSpec::parse(&Value::Object(object))?;
                fluid.amount = adjust(fluid.amount, if ore { rate } else { "none" }, true)?;
                byproducts.push(fluid);
            }
        }
        Some(Self { melting, byproducts })
    }
}

#[cfg(test)]
mod foundry_tests {
    use super::*;
    #[test]
    fn foundry_dense_copper_requires_both_source_tags() {
        let source = br#"{"type":"tconstruct:ore_melting","ingredient":{"type":"forge:intersection","children":[{"tag":"forge:ores/copper"},{"tag":"forge:ore_rates/dense"}]},"result":{"amount":540,"tag":"forge:molten_copper"},"temperature":500,"time":225,"rate":"metal","byproducts":[{"tag":"forge:molten_gold","amount":180,"rate":"metal"}]}"#;
        let recipe = FoundryRecipe::parse(source, &|_| true).unwrap();
        let both = |tag: &str, item: &str| item == "minecraft:copper_ore" && matches!(tag, "forge:ores/copper" | "forge:ore_rates/dense");
        assert!(recipe.melting.input.matches("minecraft:copper_ore", &both));
        assert!(!recipe.melting.input.matches("minecraft:copper_ore", &|tag, _| tag == "forge:ores/copper"));
        assert_eq!((recipe.melting.output.amount, recipe.byproducts[0].amount), (540, 60));
    }
    #[test]
    fn foundry_uses_source_primary_and_byproduct_ore_rates() {
        let source = br#"{"type":"tconstruct:ore_melting","ingredient":{"item":"minecraft:copper_ore"},"result":{"amount":180,"fluid":"tconstruct:molten_copper"},"temperature":500,"time":125,"rate":"metal","byproducts":[{"fluid":"tconstruct:molten_gold","amount":90},{"fluid":"tconstruct:molten_quartz","amount":100,"rate":"gem"}]}"#;
        let foundry = FoundryRecipe::parse(source, &|_| true).unwrap();
        assert_eq!(foundry.melting.output.amount, 180);
        assert_eq!(foundry.byproducts.iter().map(|f| f.amount).collect::<Vec<_>>(), [30, 100]);
        assert_eq!(MeltingRecipe::parse(source, &|_| true).unwrap().output.amount, 240);
    }
    #[test]
    fn foundry_keeps_non_ore_byproduct_amounts_and_conditions() {
        let source = br#"{"type":"tconstruct:melting","conditions":[{}],"ingredient":{"item":"minecraft:amethyst_shard"},"result":{"amount":100,"fluid":"tconstruct:molten_amethyst"},"temperature":950,"time":65,"byproducts":[{"fluid":"tconstruct:molten_quartz","amount":20}]}"#;
        assert_eq!(FoundryRecipe::parse(source, &|_| true).unwrap().byproducts[0].amount, 20);
        assert!(FoundryRecipe::parse(source, &|_| false).is_none());
    }
}

/// Source table molding with an optional hand-held pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MoldingRecipe {
    pub material: Ingredient, pub pattern: Option<Ingredient>,
    pub pattern_consumed: bool, pub result: String,
}
impl MoldingRecipe {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let v: Value = serde_json::from_slice(bytes).ok()?;
        if v.get("type")?.as_str()? != "tconstruct:molding_table" || !enabled(&v, conditions) { return None; }
        let pattern = match v.get("pattern") { Some(value) => Some(Ingredient::parse(value)?), None => None };
        let pattern_consumed = match v.get("pattern_consumed") { None => false, Some(value) => value.as_bool()? };
        Some(Self { material: Ingredient::parse(v.get("material")?)?, pattern, pattern_consumed,
            result: v.get("result")?.as_str()?.to_owned() })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Output { Item(String), Tag(String) }
/// Bucket subset of source table-filling recipes. Other containers require
/// their own fluid-capability semantics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BucketFillingRecipe;
impl BucketFillingRecipe {
    pub(super) fn matches(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> bool {
        let Ok(value) = serde_json::from_slice::<Value>(bytes) else { return false; };
        value.get("type").and_then(Value::as_str) == Some("tconstruct:table_filling")
            && value.get("container").and_then(Value::as_str) == Some("minecraft:bucket")
            && value.get("fluid_amount").and_then(Value::as_u64) == Some(1000)
            && enabled(&value, conditions)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BasinFillingRecipe { pub container: String, pub fluid_amount: u32 }
impl BasinFillingRecipe {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let value: Value = serde_json::from_slice(bytes).ok()?;
        if value.get("type")?.as_str()? != "tconstruct:basin_filling" || !enabled(&value, conditions) { return None; }
        let fluid_amount = u32::try_from(value.get("fluid_amount")?.as_u64()?).ok()?;
        if fluid_amount == 0 { return None; }
        Some(Self { container: value.get("container")?.as_str()?.to_owned(), fluid_amount })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DuplicationRecipe { pub basin: bool, pub cast: Ingredient, pub fluid: FluidSpec, pub cooling_time: u32 }
impl DuplicationRecipe {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let value: Value = serde_json::from_slice(bytes).ok()?;
        let basin = match value.get("type")?.as_str()? { "tconstruct:table_duplication" => false, "tconstruct:basin_duplication" => true, _ => return None };
        if !enabled(&value, conditions) { return None; }
        let cooling_time = u32::try_from(value.get("cooling_time")?.as_u64()?).ok()?;
        if cooling_time == 0 { return None; }
        Some(Self { basin, cast: Ingredient::parse(value.get("cast")?)?, fluid: FluidSpec::parse(value.get("fluid")?)?, cooling_time })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CastingRecipe {
    pub basin: bool, pub retextured: bool, pub cast: Option<Ingredient>, pub consumes_cast: bool,
    pub switch_slots: bool, pub fluid: FluidSpec, pub output: Output, pub cooling_time: u32,
}
pub(super) struct PartSwapCastingRecipe { pub basin: bool, pub tools: Ingredient, pub item_cost: u32, pub index: usize }
impl PartSwapCastingRecipe {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let value: Value = serde_json::from_slice(bytes).ok()?;
        let basin = match value.get("type")?.as_str()? {
            "tconstruct:table_casting_part_swapping" => false,
            "tconstruct:basin_casting_part_swapping" => true,
            _ => return None,
        };
        if !enabled(&value, conditions) || value.get("allowed_materials").is_some() { return None; }
        let item_cost = u32::try_from(value.get("item_cost")?.as_u64()?).ok()?;
        if item_cost == 0 { return None; }
        Some(Self { basin, tools: Ingredient::parse(value.get("tools")?)?, item_cost,
            index: usize::try_from(value.get("index")?.as_u64()?).ok()? })
    }
}

/// The source material casting recipe and the independent fluid-to-material
/// recipe are joined only after the current world's tags are resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MaterialCastingRecipe {
    pub basin: bool, pub cast: Option<Ingredient>, pub item_cost: u32, pub result: String,
    pub consumes_cast: bool, pub switch_slots: bool, pub allowed_materials: Option<Value>,
}
impl MaterialCastingRecipe {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let v: Value = serde_json::from_slice(bytes).ok()?;
        let basin = match v.get("type")?.as_str()? { "tconstruct:table_casting_material" => false, "tconstruct:basin_casting_material" => true, _ => return None };
        if !enabled(&v, conditions) { return None; }
        let item_cost = u32::try_from(v.get("item_cost")?.as_u64()?).ok()?;
        if item_cost == 0 { return None; }
        let cast = match v.get("cast") { Some(value) => Some(Ingredient::parse(value)?), None => None };
        Some(Self { basin, cast, item_cost,
            result: v.get("result")?.as_str()?.to_owned(), allowed_materials: v.get("allowed_materials").cloned(),
            consumes_cast: optional_bool(&v, "cast_consumed")?,
            switch_slots: optional_bool(&v, "switch_slots")? })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MaterialFluidRecipe { pub fluid: FluidSpec, pub material: String, pub temperature: u32 }
impl MaterialFluidRecipe {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let v: Value = serde_json::from_slice(bytes).ok()?;
        if v.get("type")?.as_str()? != "tconstruct:material_fluid" || !enabled(&v, conditions) || v.get("input").is_some() { return None; }
        Some(Self { fluid: FluidSpec::parse(v.get("fluid")?)?, material: v.get("output")?.as_str()?.to_owned(),
            temperature: u32::try_from(v.get("temperature")?.as_u64()?).ok()? })
    }
}
/// Composite casting consumes a material-bearing copy of its result item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CompositeCastingRecipe {
    pub basin: bool, pub item_cost: u32, pub result: String,
    pub allowed_materials: Option<Value>, pub casting_stat_conflict: Option<String>,
}
impl CompositeCastingRecipe {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let v: Value = serde_json::from_slice(bytes).ok()?;
        let basin = match v.get("type")?.as_str()? {
            "tconstruct:table_casting_composite" => false,
            "tconstruct:basin_casting_composite" => true,
            _ => return None,
        };
        if !enabled(&v, conditions) { return None; }
        let item_cost = u32::try_from(v.get("item_cost")?.as_u64()?).ok()?;
        if item_cost == 0 { return None; }
        Some(Self { basin, item_cost, result: v.get("result")?.as_str()?.to_owned(),
            allowed_materials: v.get("allowed_materials").cloned(),
            casting_stat_conflict: match v.get("casting_stat_conflict") {
                Some(value) => Some(value.as_str()?.to_owned()), None => None,
            } })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CompositeFluidRecipe { pub fluid: FluidSpec, pub input: String, pub material: String, pub temperature: u32 }
impl CompositeFluidRecipe {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let v: Value = serde_json::from_slice(bytes).ok()?;
        if v.get("type")?.as_str()? != "tconstruct:material_fluid" || !enabled(&v, conditions) { return None; }
        Some(Self { fluid: FluidSpec::parse(v.get("fluid")?)?, input: v.get("input")?.as_str()?.to_owned(),
            material: v.get("output")?.as_str()?.to_owned(), temperature: u32::try_from(v.get("temperature")?.as_u64()?).ok()? })
    }
}

impl CastingRecipe {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let v: Value = serde_json::from_slice(bytes).ok()?;
        let (basin, retextured) = match v.get("type")?.as_str()? {
            "tconstruct:casting_table" => (false, false),
            "tconstruct:casting_basin" => (true, false),
            "tconstruct:retextured_casting_basin" => (true, true),
            _ => return None,
        };
        if !enabled(&v, conditions) { return None; }
        let cast = match v.get("cast") { Some(value) => Some(Ingredient::parse(value)?), None => None };
        let result = v.get("result")?;
        let output = if let Some(item) = result.as_str() { Output::Item(item.to_owned()) }
            else { let obj = result.as_object()?; if obj.len() != 1 { return None; }
                match (obj.get("item"), obj.get("tag")) {
                    (Some(item), None) => Output::Item(item.as_str()?.to_owned()),
                    (None, Some(tag)) => Output::Tag(tag.as_str()?.to_owned()),
                    _ => return None,
                }
            };
        let cooling_time = u32::try_from(v.get("cooling_time")?.as_u64()?).ok()?;
        if cooling_time == 0 { return None; }
        Some(Self { basin, retextured, cast, consumes_cast: optional_bool(&v, "cast_consumed")?,
            switch_slots: optional_bool(&v, "switch_slots")?,
            fluid: FluidSpec::parse(v.get("fluid")?)?, output, cooling_time })
    }
}
fn optional_bool(value: &Value, key: &str) -> Option<bool> {
    match value.get(key) { None => Some(false), Some(value) => value.as_bool() }
}
fn enabled(value: &Value, conditions: &impl Fn(&[Value]) -> bool) -> bool {
    match value.get("conditions") { None => true, Some(Value::Array(items)) => conditions(items), _ => false }
}

#[cfg(test)]
mod tests {
    use super::*;
    const COPPER: &[u8] = br#"{"type":"tconstruct:melting","ingredient":{"tag":"forge:ingots/copper"},"result":{"amount":90,"tag":"forge:molten_copper"},"temperature":500,"time":50}"#;
    const CAST: &[u8] = br#"{"type":"tconstruct:casting_table","cast":{"tag":"tconstruct:casts/multi_use/ingot"},"cooling_time":50,"fluid":{"amount":90,"tag":"forge:molten_copper"},"result":{"tag":"forge:ingots/copper"}}"#;
    #[test]
    fn original_copper_melting_and_casting_keep_units_and_tag_matching() {
        let m = MeltingRecipe::parse(COPPER, &|_| false).unwrap();
        assert_eq!((m.temperature, m.time, m.output.amount), (500, 50, 90));
        assert!(m.input.matches("minecraft:copper_ingot", &|tag, item| tag == "forge:ingots/copper" && item == "minecraft:copper_ingot"));
        let c = CastingRecipe::parse(CAST, &|_| false).unwrap();
        assert_eq!((c.basin, c.consumes_cast, c.cooling_time, c.fluid.amount), (false, false, 50, 90));
        assert_eq!(c.output, Output::Tag("forge:ingots/copper".into()));
    }
    #[test]
    fn original_ore_rates_and_difference_ingredients_apply() {
        let ore = br#"{"type":"tconstruct:ore_melting","ingredient":{"type":"forge:difference","base":{"tag":"forge:ores/copper"},"subtracted":{"tag":"tconstruct:non_singular_ore_rates"}},"result":{"amount":180,"tag":"forge:molten_copper"},"temperature":500,"time":125,"rate":"metal","byproducts":[{"tag":"forge:molten_gold","amount":60,"rate":"metal"}]}"#;
        let recipe = MeltingRecipe::parse(ore, &|_| true).unwrap();
        assert_eq!(recipe.output.amount, 240);
        assert!(recipe.input.matches("minecraft:copper_ore", &|tag, _| tag == "forge:ores/copper"));
        assert!(!recipe.input.matches("minecraft:copper_ore", &|_, _| true));
    }
    #[test]
    fn melter_omits_foundry_byproducts_but_rejects_unknown_conditions_and_nbt() {
        let m = br#"{"type":"tconstruct:melting","ingredient":{"item":"minecraft:iron_ingot"},"result":{"amount":90,"fluid":"tconstruct:molten_iron"},"temperature":800,"time":60,"byproducts":[{"amount":10,"fluid":"tconstruct:molten_gold"}]}"#;
        assert_eq!(MeltingRecipe::parse(m, &|_| true).unwrap().output.amount, 90);
        let c = br#"{"type":"tconstruct:casting_basin","conditions":[{"type":"forge:mod_loaded","modid":"unknown"}],"cooling_time":40,"fluid":{"amount":90,"tag":"forge:molten_copper"},"result":{"item":"minecraft:stone","nbt":{}}}"#;
        assert!(CastingRecipe::parse(c, &|_| false).is_none());
        assert!(CastingRecipe::parse(c, &|_| true).is_none());
    }
}

#[cfg(test)]
mod damaged_tests {
    use super::*;
    #[test]
    fn scales_primary_and_byproduct_by_source_units() {
        let raw = br#"{"type":"tconstruct:damagable_melting","ingredient":{"item":"minecraft:diamond_pickaxe"},"result":{"amount":300,"fluid":"tconstruct:molten_diamond","unit_size":25},"temperature":1450,"time":137,"byproducts":[{"fluid":"tconstruct:molten_gold","amount":90,"unit_size":10}]}"#;
        let half = damaged_melting(raw, 50, 100, &|_| true).unwrap();
        assert_eq!(half.melting.output.amount, 150);
        assert_eq!(half.byproducts[0].amount, 40);
        let almost_broken = damaged_melting(raw, 99, 100, &|_| true).unwrap();
        assert_eq!((almost_broken.melting.output.amount, almost_broken.byproducts[0].amount), (25, 10));
        assert_eq!(damaged_melting(raw, 0, 0, &|_| true).unwrap().melting.output.amount, 300);
    }
}

#[cfg(test)]
mod material_melting_tests {
    use super::*;
    #[test]
    fn material_and_cost_control_outputs() {
        let raw = br#"{"type":"tconstruct:material_melting","input":"tconstruct:copper","result":{"fluid":"tconstruct:molten_copper","amount":90},"temperature":500,"byproducts":[{"fluid":"tconstruct:molten_gold","amount":10}]}"#;
        let found = material_melting(raw, "tconstruct:pick_head", "tconstruct:copper#default", 2, &|_| true).unwrap();
        assert_eq!(found.melting.output.amount, 180);
        assert_eq!(found.byproducts[0].amount, 20);
        assert!(found.melting.time > 0);
        assert!(material_melting(raw, "tconstruct:pick_head", "tconstruct:iron", 2, &|_| true).is_none());
        assert!(material_melting(raw, "tconstruct:pick_head", "tconstruct:copper", 0, &|_| true).is_none());
    }
}
