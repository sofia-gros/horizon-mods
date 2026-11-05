fn tconstruct_advancement_tool_predicate(v: &serde_json::Value, it: &ItemView, w: &dyn World) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    use horizon_world::nbt::Tag;
    fn valid_id(id: &str) -> bool {
        id.split_once(':').is_some_and(|(namespace, path)| {
            !namespace.is_empty() && !path.is_empty()
                && namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
                && path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
        })
    }
    fn material_id(id: &str) -> Option<(&str, Option<&str>)> {
        let (base, variant) = id.split_once('#').map_or((id, None), |(base, variant)| (base, Some(variant)));
        if !valid_id(base) || variant.is_some_and(|variant| variant.is_empty()
            || !variant.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))) {
            return None;
        }
        Some((base, variant))
    }
    fn matches(predicate: &serde_json::Value, it: &ItemView, depth: u8) -> bool {
        if depth > 8 { return false; }
        match predicate.get("type").and_then(serde_json::Value::as_str) {
            Some("mantle:and") => {
                let Some(children) = predicate.get("predicates").and_then(serde_json::Value::as_array) else { return false; };
                return !children.is_empty() && children.len() <= 32
                    && children.iter().all(|child| matches(child, it, depth + 1));
            }
            Some("mantle:set") => {
                let Some(items) = predicate.get("items").and_then(serde_json::Value::as_array) else { return false; };
                return !items.is_empty() && items.len() <= 64
                    && items.iter().all(|item| item.as_str().is_some_and(valid_id))
                    && items.iter().any(|item| item.as_str() == Some(it.id.as_str()));
            }
            _ => {},
        }
        if predicate.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:has_material") {
            // Bounded compact material-ID form; tag/logical material predicates
            // are not interpreted as a weaker ID or wildcard match.
            let Some(wanted) = predicate.get("material").and_then(serde_json::Value::as_str).and_then(material_id) else {
                return false;
            };
            let index = match predicate.get("index") {
                None => -1,
                Some(value) => match value.as_i64() {
                    Some(index) if (-1..=i32::MAX as i64).contains(&index) => index,
                    _ => return false,
                },
            };
            let Some(Tag::List(materials)) = it.custom_data.as_ref().and_then(|data| data.get("tic_materials")) else {
                return false;
            };
            let mut matched = false;
            for (slot, value) in materials.iter().enumerate() {
                let Tag::String(id) = value else { return false; };
                let Ok(id) = id.to_string() else { return false; };
                let Some(actual) = material_id(&id) else { return false; };
                if index >= 0 && slot != index as usize { continue; }
                matched |= wanted.0 == actual.0 && match wanted.1 {
                    None => true,
                    Some("default") => actual.1.is_none(),
                    Some(variant) => actual.1 == Some(variant),
                };
            }
            return matched;
        }
        if predicate.as_str() != Some("tconstruct:has_upgrades") {
            return false;
        }
        let Some(Tag::Compound(upgrades)) = it.custom_data.as_ref().and_then(|data| data.get("tic_upgrades")) else {
            return false;
        };
        if upgrades.iter().any(|(id, amount)| {
            let Ok(id) = id.to_string() else { return true; };
            !valid_id(&id) || !matches!(amount, Tag::Int(level) if *level >= 0)
        }) { return false; }
        // Native saved upgrades use positive integer amounts (including partial
        // upgrades); material traits and total modifier levels are separate data.
        upgrades.iter().any(|(_, amount)| matches!(amount, Tag::Int(level) if *level > 0))
    }
    if v.get("type").and_then(serde_json::Value::as_str) != Some("tconstruct:tool_stack") {
        return Continue(());
    }
    // Keep the source wrapper's item gate around every composed predicate.
    // Unsupported forms must not fall through to the host's vanilla matcher.
    if it.count == 0 || !w.in_tag("item", "tconstruct:modifiable", &it.id) {
        return Break(false);
    }
    Break(matches(&v["predicate"], it, 0))
}
