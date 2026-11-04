// Apply source shapes to the shared picking and collision tables.
pub(super) fn apply(facts: &serde_json::Value, gameplay: &mut serde_json::Value, out: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        // The registration templates are only behavioral stand-ins. Replace
        // their selection and collision geometry with the original machine shapes.
        let mut machine_shapes = std::collections::BTreeMap::new();
        if let (Some(states), Some(rows)) = (facts["states"].as_array(), gameplay["states"].as_array_mut()) {
            for (id, state) in states.iter().enumerate() {
                let Some(name) = state.get(0).and_then(serde_json::Value::as_str) else { continue; };
                let facing = state.pointer("/4/properties/facing").and_then(serde_json::Value::as_str).unwrap_or("north");
                let key = (name.to_owned(), facing.to_owned());
                if !machine_shapes.contains_key(&key) {
                    if let Some(shape) = super::tconstruct_machine_shapes::shape(name, facing) { machine_shapes.insert(key.clone(), shape); }
                }
                let Some(shape) = machine_shapes.get(&key) else { continue; };
                let row = rows.get_mut(id).ok_or("missing machine shape row")?;
                row["outline"] = serde_json::to_value(&shape.boxes)?;
                row["outline_edges"] = serde_json::to_value(&shape.edges)?;
                row["full_collision"] = serde_json::Value::Bool(false);
                // These source blocks use noOcclusion and have a non-full
                // shape. Do not inherit the full-cube template's light block.
                if matches!(name, "tconstruct:tinker_station" | "tconstruct:part_builder" | "tconstruct:seared_basin" | "tconstruct:scorched_basin") {
                    row["dampening"] = serde_json::json!(0);
                }
            }
        }
        for name in ["cached-collision.nbt", "worldgen/blocks/cached-collision.nbt"] {
            let path = out.join(name);
            let (mut named, _) = horizon_world::nbt::read_named(&std::fs::read(&path)?, horizon_world::nbt::Limits::tables())?;
            let mut indices = std::collections::BTreeMap::new();
            {
                let Some(horizon_world::nbt::Tag::List(shapes)) = named.tag.get_mut("shapes") else { return Err("missing collision shapes".into()); };
                for (key, shape) in &machine_shapes {
                    let index = i32::try_from(shapes.len())?;
                    let coords = horizon_world::nbt::Tag::LongArray((0..=16).map(|n| (n as f64 / 16.0).to_bits() as i64).collect());
                    let mut min = [16i32; 3]; let mut max = [0i32; 3];
                    for x in 0..16 { for y in 0..16 { for z in 0..16 {
                        if shape.grid[(x*16+y)*16+z] != 0 {
                            for (axis, n) in [x,y,z].into_iter().enumerate() { min[axis] = min[axis].min(n as i32); max[axis] = max[axis].max(n as i32 + 1); }
                        }
                    }}}
                    shapes.push(horizon_world::nbt::Tag::compound([
                        ("x", coords.clone()), ("y", coords.clone()), ("z", coords),
                        ("occupied", horizon_world::nbt::Tag::ByteArray(shape.grid.clone())),
                        ("cube", horizon_world::nbt::Tag::Byte(1)), ("cube_axes", horizon_world::nbt::Tag::Byte(7)),
                        ("canonical", horizon_world::nbt::Tag::Byte(0)),
                        ("discrete_min", horizon_world::nbt::Tag::IntArray(min.to_vec())),
                        ("discrete_max", horizon_world::nbt::Tag::IntArray(max.to_vec())),
                    ]));
                    indices.insert(key.clone(), index);
                }
            }
            let Some(horizon_world::nbt::Tag::IntArray(mapping)) = named.tag.get_mut("cached_shapes") else { return Err("missing collision mapping".into()); };
            for (id, state) in facts["states"].as_array().ok_or("missing states")?.iter().enumerate() {
                let Some(name) = state.get(0).and_then(serde_json::Value::as_str) else { continue; };
                let facing = state.pointer("/4/properties/facing").and_then(serde_json::Value::as_str).unwrap_or("north");
                if let Some(index) = indices.get(&(name.to_owned(), facing.to_owned())) {
                    *mapping.get_mut(id).ok_or("missing machine collision state")? = *index;
                }
            }
            std::fs::write(path, horizon_world::nbt::write_named(&named, horizon_world::nbt::Limits::tables())?)?;
        }
    Ok(())
}
