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
    fn sets_only_target_shapes_and_rejects_unmatched_geometry() {
        let mut table = Tag::compound([("cached_shapes", Tag::IntArray(vec![0, 1, 0])), ("shapes", Tag::List(vec![shape([0.0, 1.0]), shape([0.0625, 0.9375])]))]);
        apply_congealed_shape(&mut table, 1, &[2]).unwrap();
        assert_eq!(table.get("cached_shapes"), Some(&Tag::IntArray(vec![0, 1, 1])));
        let old = table.clone();
        assert!(apply_congealed_shape(&mut table, 0, &[2]).is_err());
        assert_eq!(table, old);
    }
}
