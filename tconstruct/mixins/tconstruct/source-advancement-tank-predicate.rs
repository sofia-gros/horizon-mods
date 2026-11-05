// Explicit native adaptation of the source goals' legacy tank SNBT subset.
// A custom type prevents the vanilla matcher from ignoring the fluid fields.
fn tconstruct_advancement_tank_predicate(v: &serde_json::Value, it: &ItemView) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    use horizon_world::nbt::Tag;
    fn matches(v: &serde_json::Value, it: &ItemView) -> Option<bool> {
        let fields = v.as_object()?;
        if fields.keys().any(|key| !matches!(key.as_str(), "type" | "items" | "count" | "tank")) {
            return None;
        }
        // The ten source criteria each select exactly one supported container.
        let items = v.get("items")?.as_array()?;
        if items.len() != 1 || items[0].as_str()? != it.id || it.count == 0 { return Some(false); }
        let capacity = match it.id.as_str() {
            "tconstruct:seared_fuel_tank" | "tconstruct:seared_fuel_gauge"
            | "tconstruct:scorched_fuel_tank" | "tconstruct:scorched_fuel_gauge" => 4000,
            "tconstruct:seared_ingot_tank" | "tconstruct:seared_ingot_gauge"
            | "tconstruct:scorched_ingot_tank" | "tconstruct:scorched_ingot_gauge" => 4320,
            "tconstruct:seared_lantern" | "tconstruct:scorched_lantern" => 50,
            _ => return None,
        };
        if let Some(count) = v.get("count") {
            let range = count.as_object()?;
            if range.keys().any(|key| !matches!(key.as_str(), "min" | "max")) { return None; }
            let bound = |key: &str, default: u64| -> Option<u64> {
                match range.get(key) {
                    None => Some(default),
                    Some(value) => value.as_u64().filter(|value| *value <= u32::MAX as u64),
                }
            };
            let min = bound("min", 0)?;
            let max = bound("max", u32::MAX as u64)?;
            if min > max || !(min..=max).contains(&u64::from(it.count)) { return Some(false); }
        }
        let wanted = v.get("tank")?.as_object()?;
        if wanted.len() != 2 { return None; }
        let amount = wanted.get("amount")?.as_u64()?;
        if amount == 0 || amount > capacity { return None; }
        let fluid = wanted.get("fluid")?.as_str()?;
        // These goals select these two source fluids, not arbitrary FluidHandlers.
        if !matches!(fluid, "tconstruct:blazing_blood" | "tconstruct:molten_manyullyn") { return None; }
        let custom = it.custom_data.as_ref()?;
        if !matches!(custom, Tag::Compound(_)) || custom.get("tank").is_some() { return None; }
        let tank = custom.get("tconstruct:tank")?;
        if !matches!(tank, Tag::Compound(_)) { return None; }
        // Never resolve mixed Forge/native owned fields by silently choosing one.
        if ["FluidName", "Amount", "Tag"].iter().any(|key| tank.get(key).is_some()) { return None; }
        let Tag::Int(actual_amount) = tank.get("amount")? else { return None; };
        if u64::try_from(*actual_amount).ok()? != amount { return Some(false); }
        let Tag::String(actual_fluid) = tank.get("fluid")? else { return None; };
        let actual_fluid = actual_fluid.to_string().ok()?;
        if !matches!(tank.get("fluid_tag"), None | Some(Tag::Compound(_))) { return None; }
        // Source NBT is a subset: unrelated native extensions and compound fluid
        // metadata do not invalidate a matching identity/amount. No data is changed.
        Some(actual_fluid == fluid)
    }
    if v.get("type").and_then(serde_json::Value::as_str) != Some("tconstruct:portable_tank") {
        return Continue(());
    }
    Break(matches(v, it).unwrap_or(false))
}
