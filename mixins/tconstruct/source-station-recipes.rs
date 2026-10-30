//! Source tool-building recipe data used by the station's shared transaction.
use horizon_items::items::ItemStack;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BuildingRecipe {
    pub result: String,
    pub result_count: u32,
    pub layout: Option<String>,
    pub parts_override: Option<Vec<String>>,
    pub extra_requirements: Vec<Value>,
    pub extra_materials: Vec<String>,
}

impl BuildingRecipe {
    pub(super) fn parse(bytes: &[u8]) -> Option<Self> {
        let value: Value = serde_json::from_slice(bytes).ok()?;
        if value.get("type")?.as_str()? != "tconstruct:tool_building" { return None; }
        let strings = |v: &Value| v.as_array()?.iter().map(|item| Some(item.as_str()?.to_owned())).collect::<Option<Vec<_>>>();
        let count = match value.get("result_count") {
            Some(v) => u32::try_from(v.as_u64()?).ok().filter(|n| *n > 0)?,
            None => 1,
        };
        Some(Self {
            result: value.get("result")?.as_str()?.to_owned(),
            result_count: count,
            layout: value.get("slot_layout").and_then(Value::as_str).map(str::to_owned),
            parts_override: match value.get("parts_override") { Some(Value::Null) | None => None, Some(v) => Some(strings(v)?) },
            extra_requirements: match value.get("extra_requirements") { None => Vec::new(), Some(v) => v.as_array()?.clone() },
            extra_materials: match value.get("extra_materials") { None => Vec::new(), Some(v) => strings(v)? },
        })
    }

    /// Parts must occupy their ordered input slots; other recipe conditions and
    /// material validation are evaluated separately by the caller.
    pub(super) fn matches_parts(&self, expected: &[String], inputs: &[Option<ItemStack>]) -> bool {
        if expected.is_empty() || expected.len() > inputs.len() { return false; }
        expected.iter().zip(inputs).all(|(part, stack)| stack.as_ref().is_some_and(|s| s.item.as_ref() == part && s.count > 0))
            && inputs[expected.len()..].iter().all(Option::is_none)
            && self.extra_requirements.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordered_parts_follow_recipe_and_reject_extra_inputs() {
        let recipe = BuildingRecipe::parse(br#"{"type":"tconstruct:tool_building","result":"tconstruct:pickaxe"}"#).unwrap();
        let parts = vec!["tconstruct:pick_head".to_owned(), "tconstruct:tool_handle".to_owned()];
        let mut slots = vec![Some(ItemStack::new(&parts[0], 1)), Some(ItemStack::new(&parts[1], 1)), None];
        assert!(recipe.matches_parts(&parts, &slots));
        slots.swap(0, 1);
        assert!(!recipe.matches_parts(&parts, &slots));
        slots.swap(0, 1);
        slots[2] = Some(ItemStack::new("minecraft:stone", 1));
        assert!(!recipe.matches_parts(&parts, &slots));
    }
}
