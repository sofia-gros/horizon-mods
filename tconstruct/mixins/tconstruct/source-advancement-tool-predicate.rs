fn tconstruct_advancement_tool_predicate(v: &serde_json::Value, it: &ItemView, w: &dyn World) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    use horizon_world::nbt::Tag;
    if v.get("type").and_then(serde_json::Value::as_str) != Some("tconstruct:tool_stack") {
        return Continue(());
    }
    // Do not let the native matcher ignore an unsupported source predicate
    // and accidentally grant an advancement for an arbitrary inventory item.
    if v.get("predicate").and_then(serde_json::Value::as_str) != Some("tconstruct:has_upgrades")
        || it.count == 0 || !w.in_tag("item", "tconstruct:modifiable", &it.id) {
        return Break(false);
    }
    let Some(Tag::Compound(upgrades)) = it.custom_data.as_ref().and_then(|data| data.get("tic_upgrades")) else {
        return Break(false);
    };
    if upgrades.iter().any(|(id, amount)| {
        let Ok(id) = id.to_string() else { return true; };
        let valid_id = id.split_once(':').is_some_and(|(namespace, path)| {
            !namespace.is_empty() && !path.is_empty()
                && namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
                && path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
        });
        !valid_id || !matches!(amount, Tag::Int(level) if *level >= 0)
    }) { return Break(false); }
    // Native saved upgrades use positive integer amounts (including partial
    // upgrades); material traits and total modifier levels are separate data.
    Break(upgrades.iter().any(|(_, amount)| matches!(amount, Tag::Int(level) if *level > 0)))
}
