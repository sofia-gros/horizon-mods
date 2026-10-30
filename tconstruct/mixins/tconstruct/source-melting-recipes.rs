// Parse the source melting and casting recipe forms used by the melter.
// Registry tags and recipe conditions are resolved by the caller against the
// loaded game data, rather than guessed from a legacy item list.
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Ingredient { Item(String), Tag(String), Any(Vec<Ingredient>), Difference(Box<Ingredient>, Box<Ingredient>) }

impl Ingredient {
    fn parse(value: &Value) -> Option<Self> {
        if let Some(values) = value.as_array() {
            let choices: Vec<_> = values.iter().map(Self::parse).collect::<Option<_>>()?;
            return (!choices.is_empty()).then_some(Self::Any(choices));
        }
        let obj = value.as_object()?;
        if obj.get("type").and_then(Value::as_str) == Some("forge:difference") {
            if obj.keys().any(|key| key != "type" && key != "base" && key != "subtracted") { return None; }
            return Some(Self::Difference(Box::new(Self::parse(obj.get("base")?)?), Box::new(Self::parse(obj.get("subtracted")?)?)));
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Output { Item(String), Tag(String) }
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CastingRecipe {
    pub basin: bool, pub cast: Option<Ingredient>, pub consumes_cast: bool,
    pub switch_slots: bool, pub fluid: FluidSpec, pub output: Output, pub cooling_time: u32,
}
impl CastingRecipe {
    pub(super) fn parse(bytes: &[u8], conditions: &impl Fn(&[Value]) -> bool) -> Option<Self> {
        let v: Value = serde_json::from_slice(bytes).ok()?;
        let basin = match v.get("type")?.as_str()? { "tconstruct:casting_table" => false, "tconstruct:casting_basin" => true, _ => return None };
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
        Some(Self { basin, cast, consumes_cast: v.get("cast_consumed").and_then(Value::as_bool).unwrap_or(false),
            switch_slots: v.get("switch_slots").and_then(Value::as_bool).unwrap_or(false),
            fluid: FluidSpec::parse(v.get("fluid")?)?, output, cooling_time })
    }
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
