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
    fn has_upgrade(it: &ItemView, wanted: Option<&str>) -> bool {
        let Some(upgrades) = it.custom_data.as_ref().and_then(|data| data.get("tic_upgrades")) else { return false; };
        let Tag::Compound(entries) = upgrades else { return false; };
        if entries.iter().any(|(id, amount)| {
            let Ok(id) = id.to_string() else { return true; };
            !valid_id(&id) || !matches!(amount, Tag::Int(level) if *level >= 0)
        }) { return false; }
        // Native saved amounts include progressive upgrades. Material traits
        // and computed total modifier levels are deliberately not consulted.
        match wanted {
            Some(id) => matches!(upgrades.get(id), Some(Tag::Int(level)) if *level > 0),
            None => entries.iter().any(|(_, amount)| matches!(amount, Tag::Int(level) if *level > 0)),
        }
    }
    fn matches(predicate: &serde_json::Value, it: &ItemView, w: &dyn World, depth: u8) -> bool {
        if depth > 8 { return false; }
        match predicate.get("type").and_then(serde_json::Value::as_str) {
            Some("mantle:and") => {
                let Some(children) = predicate.get("predicates").and_then(serde_json::Value::as_array) else { return false; };
                return !children.is_empty() && children.len() <= 32
                    && children.iter().all(|child| matches(child, it, w, depth + 1));
            }
            Some("mantle:set") => {
                let Some(items) = predicate.get("items").and_then(serde_json::Value::as_array) else { return false; };
                return !items.is_empty() && items.len() <= 64
                    && items.iter().all(|item| item.as_str().is_some_and(valid_id))
                    && items.iter().any(|item| item.as_str() == Some(it.id.as_str()));
            }
            Some("mantle:tag") => {
                // Only this declared acquisition tag uses the resolved-world
                // bridge; do not fall back to stale host tags for other IDs.
                return predicate.get("tag").and_then(serde_json::Value::as_str)
                    == Some("tconstruct:modifiable/harvest")
                    && w.in_tag("item", "tconstruct:modifiable/harvest", &it.id);
            }
            Some("tconstruct:stat_in_set") => {
                if predicate.get("stat").and_then(serde_json::Value::as_str) != Some("tconstruct:harvest_tier") {
                    return false;
                }
                const TIERS: &[&str] = &["minecraft:wood", "minecraft:gold", "minecraft:stone",
                    "minecraft:iron", "minecraft:diamond", "minecraft:netherite"];
                let Some(values) = predicate.get("values").and_then(serde_json::Value::as_array) else { return false; };
                if values.is_empty() || values.len() > 64
                    || !values.iter().all(|value| value.as_str().is_some_and(|id| TIERS.contains(&id))) {
                    return false;
                }
                let Some(Tag::String(tier)) = it.custom_data.as_ref().and_then(|data| data.get("tic_harvest_tier")) else { return false; };
                let Ok(tier) = tier.to_string() else { return false; };
                return TIERS.contains(&tier.as_str()) && values.iter().any(|value| value.as_str() == Some(tier.as_str()));
            }
            Some("tconstruct:has_modifier") => {
                // Only the source goals' installed-upgrade presence form is
                // supported. Never ignore an explicit level or all/trait check.
                if predicate.get("check").and_then(serde_json::Value::as_str) != Some("upgrades")
                    || predicate.get("level").is_some() { return false; }
                let modifier = &predicate["modifier"];
                if modifier.get("type").and_then(serde_json::Value::as_str) != Some("tconstruct:single") { return false; }
                let Some(id) = modifier.get("modifier").and_then(serde_json::Value::as_str).filter(|id| valid_id(id)) else { return false; };
                return has_upgrade(it, Some(id));
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
        predicate.as_str() == Some("tconstruct:has_upgrades") && has_upgrade(it, None)
    }
    if v.get("type").and_then(serde_json::Value::as_str) != Some("tconstruct:tool_stack") {
        return Continue(());
    }
    // Keep the source wrapper's item gate around every composed predicate.
    // Unsupported forms must not fall through to the host's vanilla matcher.
    if it.count == 0 || !w.in_tag("item", "tconstruct:modifiable", &it.id) {
        return Break(false);
    }
    Break(matches(&v["predicate"], it, w, 0))
}
