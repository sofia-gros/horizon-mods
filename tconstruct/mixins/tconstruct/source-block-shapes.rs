use horizon_world::nbt::Tag;

/// Reuse the matching vanilla collision shape without copying honey behavior.
pub(super) fn apply_congealed_shape(table: &mut Tag, honey: usize, targets: &[usize]) -> Result<(), &'static str> {
    let Tag::IntArray(indices) = table.get("cached_shapes").ok_or("missing cached shapes")? else { return Err("invalid cached shapes"); };
    let index = *indices.get(honey).ok_or("missing honey state")?;
    let Tag::List(shapes) = table.get("shapes").ok_or("missing collision shapes")? else { return Err("invalid collision shapes"); };
    let shape = shapes.get(usize::try_from(index).map_err(|_| "invalid honey shape")?).ok_or("missing honey shape")?;
    let bits = |values: &[f64]| Tag::LongArray(values.iter().map(|value| value.to_bits() as i64).collect());
    if shape.get("x") != Some(&bits(&[0.0625, 0.9375])) || shape.get("y") != Some(&bits(&[0.0, 0.9375])) || shape.get("z") != Some(&bits(&[0.0625, 0.9375])) {
        return Err("honey collision shape differs from source congealed slime");
    }
    if targets.iter().any(|target| *target >= indices.len()) { return Err("missing congealed state"); }
    let Some(Tag::IntArray(indices)) = table.get_mut("cached_shapes") else { return Err("invalid cached shapes"); };
    for target in targets { indices[*target] = index; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shape(x: [f64; 2]) -> Tag {
        let coords = |values: [f64; 2]| Tag::LongArray(values.iter().map(|v| v.to_bits() as i64).collect());
        Tag::compound([("x", coords(x)), ("y", coords([0.0, 0.9375])), ("z", coords([0.0625, 0.9375]))])
    }
    #[test]
    fn cheese_lighting_is_idempotent_and_preserves_other_fields() {
        let mut row = serde_json::json!({"dampening":0,"skylight_down":true,"emission":7});
        apply_cheese_lighting(&mut row).unwrap();
        assert_eq!(row, serde_json::json!({"dampening":1,"skylight_down":false,"emission":7}));
        let before = row.clone();
        apply_cheese_lighting(&mut row).unwrap();
        assert_eq!(row, before);
        let mut malformed = serde_json::json!({"dampening":0,"skylight_down":"bad"});
        let before = malformed.clone();
        assert!(apply_cheese_lighting(&mut malformed).is_err());
        assert_eq!(malformed, before);
    }
    #[test]
    fn cheese_behavior_changes_only_transparent_subclass() {
        let mut table = serde_json::json!({"blocks":[{"name":"tconstruct:cheese_block","class":["TransparentBlock","HalfTransparentBlock","Block"]},{"name":"minecraft:glass","class":["TransparentBlock","HalfTransparentBlock","Block"]}]});
        apply_cheese_behavior(&mut table).unwrap();
        assert_eq!(table["blocks"][0]["class"], serde_json::json!(["HalfTransparentBlock","Block"]));
        assert_eq!(table["blocks"][1]["class"], serde_json::json!(["TransparentBlock","HalfTransparentBlock","Block"]));
        let before = table.clone();
        apply_cheese_behavior(&mut table).unwrap();
        assert_eq!(table, before);
        let mut malformed = serde_json::json!({"blocks":[{"name":"tconstruct:cheese_block","class":["TransparentBlock"]}]});
        let before = malformed.clone();
        assert!(apply_cheese_behavior(&mut malformed).is_err());
        assert_eq!(malformed, before);
    }
    #[test]
    fn cheese_changes_only_factors_and_rejects_invalid_definitions() {
        let row = |speed: f32, jump: f32| Tag::compound([("speedFactor", Tag::Float(speed.to_bits())), ("jumpFactor", Tag::Float(jump.to_bits()))]);
        let mut table = Tag::compound([("blocks", Tag::compound([("tconstruct:cheese_block", row(1.0, 1.0)), ("minecraft:glass", row(1.0, 1.0))]))]);
        apply_cheese_movement(&mut table).unwrap();
        let blocks = table.get("blocks").unwrap();
        let cheese = blocks.get("tconstruct:cheese_block").unwrap();
        assert_eq!(cheese.get("speedFactor"), Some(&Tag::Float(0.4_f32.to_bits())));
        assert_eq!(cheese.get("jumpFactor"), Some(&Tag::Float(0.5_f32.to_bits())));
        assert_eq!(blocks.get("minecraft:glass"), Some(&row(1.0, 1.0)));
        let mut malformed = Tag::compound([("blocks", Tag::compound([("tconstruct:cheese_block", Tag::compound([("speedFactor", Tag::Float(1.0_f32.to_bits()))]))]))]);
        let before = malformed.clone();
        assert!(apply_cheese_movement(&mut malformed).is_err());
        assert_eq!(malformed, before);
    }
    #[test]
    fn sets_only_target_shapes_and_rejects_unmatched_geometry() {
        let mut table = Tag::compound([("cached_shapes", Tag::IntArray(vec![0, 1, 0])), ("shapes", Tag::List(vec![shape([0.0, 1.0]), shape([0.0625, 0.9375])]))]);
        apply_congealed_shape(&mut table, 1, &[2]).unwrap();
        assert_eq!(table.get("cached_shapes"), Some(&Tag::IntArray(vec![0, 1, 1])));
        let old = table.clone();
        assert!(apply_congealed_shape(&mut table, 0, &[2]).is_err());
        assert_eq!(table, old);
    }
}

/// Soul glass uses an empty collision shape and slows movement inside the block.
pub(super) fn apply_empty_shape(table: &mut Tag, air: usize, targets: &[usize]) -> Result<(), &'static str> {
    let Tag::IntArray(indices) = table.get("cached_shapes").ok_or("missing cached shapes")? else { return Err("invalid cached shapes"); };
    let index = *indices.get(air).ok_or("missing air state")?;
    let Tag::List(shapes) = table.get("shapes").ok_or("missing collision shapes")? else { return Err("invalid collision shapes"); };
    let shape = shapes.get(usize::try_from(index).map_err(|_| "invalid air shape")?).ok_or("missing air shape")?;
    if shape.get("occupied") != Some(&Tag::ByteArray(vec![])) { return Err("air collision shape is not empty"); }
    if targets.iter().any(|target| *target >= indices.len()) { return Err("missing soul glass state"); }
    let Some(Tag::IntArray(indices)) = table.get_mut("cached_shapes") else { return Err("invalid cached shapes"); };
    for target in targets { indices[*target] = index; }
    Ok(())
}

/// Copy the full-cube collision used by a vanilla solid block.
pub(super) fn apply_full_shape(table: &mut Tag, solid: usize, targets: &[usize]) -> Result<(), &'static str> {
    let Tag::IntArray(indices) = table.get("cached_shapes").ok_or("missing cached shapes")? else { return Err("invalid cached shapes"); };
    let index = *indices.get(solid).ok_or("missing solid state")?;
    let Tag::List(shapes) = table.get("shapes").ok_or("missing collision shapes")? else { return Err("invalid collision shapes"); };
    let shape = shapes.get(usize::try_from(index).map_err(|_| "invalid solid shape")?).ok_or("missing solid shape")?;
    let unit = Tag::LongArray(vec![0.0f64.to_bits() as i64, 1.0f64.to_bits() as i64]);
    if ["x","y","z"].iter().any(|axis| shape.get(axis) != Some(&unit)) { return Err("solid block shape is not a unit cube"); }
    if !matches!(shape.get("occupied"), Some(Tag::ByteArray(value)) if !value.is_empty()) { return Err("solid block shape is empty"); }
    if targets.iter().any(|target| *target >= indices.len()) { return Err("missing cannon state"); }
    let Some(Tag::IntArray(indices)) = table.get_mut("cached_shapes") else { return Err("invalid cached shapes"); };
    for target in targets { indices[*target] = index; }
    Ok(())
}

pub(super) fn apply_soul_speed(table: &mut Tag, targets: &[(&str, f32)]) -> Result<(), &'static str> {
    let blocks = table.get_mut("blocks").ok_or("missing movement definitions")?;
    for (name, factor) in targets {
        let entry = blocks.get_mut(name).ok_or("missing soul glass movement definition")?;
        let Some(Tag::Float(bits)) = entry.get_mut("speedFactor") else { return Err("invalid speed factor"); };
        *bits = factor.to_bits();
    }
    Ok(())
}

/// Apply source Cheese Block factors without changing any other movement entry.
pub(super) fn apply_cheese_movement(table: &mut Tag) -> Result<(), &'static str> {
    let entry = table.get_mut("blocks").and_then(|blocks| blocks.get_mut("tconstruct:cheese_block")).ok_or("missing cheese movement definition")?;
    if !matches!(entry.get("speedFactor"), Some(Tag::Float(_))) || !matches!(entry.get("jumpFactor"), Some(Tag::Float(_))) { return Err("invalid cheese movement definition"); }
    let Some(Tag::Float(speed)) = entry.get_mut("speedFactor") else { unreachable!() };
    *speed = 0.4_f32.to_bits();
    let Some(Tag::Float(jump)) = entry.get_mut("jumpFactor") else { unreachable!() };
    *jump = 0.5_f32.to_bits();
    Ok(())
}

/// Restore light attenuation inherited from source HalfTransparentBlock.
pub(super) fn apply_cheese_lighting(row: &mut serde_json::Value) -> Result<(), &'static str> {
    if !row.get("dampening").is_some_and(serde_json::Value::is_u64) || !row.get("skylight_down").is_some_and(serde_json::Value::is_boolean) { return Err("invalid cheese light facts"); }
    row["dampening"] = serde_json::json!(1);
    row["skylight_down"] = serde_json::json!(false);
    Ok(())
}

/// Glass's TransparentBlock subclass has brighter shading than the source
/// HalfTransparentBlock. Remove only that class from the cloned behavior.
pub(super) fn apply_cheese_behavior(behaviors: &mut serde_json::Value) -> Result<(), &'static str> {
    let blocks = behaviors.get_mut("blocks").and_then(serde_json::Value::as_array_mut).ok_or("missing block behaviors")?;
    let row = blocks.iter_mut().find(|row| row.get("name").and_then(serde_json::Value::as_str) == Some("tconstruct:cheese_block")).ok_or("missing cheese behavior")?;
    let classes = row.get_mut("class").and_then(serde_json::Value::as_array_mut).ok_or("invalid cheese class")?;
    if !classes.iter().any(|c| c.as_str() == Some("HalfTransparentBlock")) || !classes.iter().any(|c| c.as_str() == Some("Block")) { return Err("unexpected cheese class"); }
    classes.retain(|c| c.as_str() != Some("TransparentBlock"));
    Ok(())
}

/// Exact source-connected pane IDs, used only when preparing render metadata.
pub(super) fn is_source_connected_pane(name: &str) -> bool {
    let Some(name) = name.strip_prefix("tconstruct:") else { return false; };
    if matches!(name, "clear_glass_pane" | "soul_glass_pane" | "seared_glass_pane" | "seared_soul_glass_pane" | "scorched_glass_pane" | "scorched_soul_glass_pane") { return true; }
    let Some(color) = name.strip_suffix("_clear_stained_glass_pane") else { return false; };
    matches!(color, "white" | "orange" | "magenta" | "light_blue" | "yellow" | "lime" | "pink" | "gray" | "light_gray" | "cyan" | "purple" | "blue" | "brown" | "green" | "red" | "black")
}
