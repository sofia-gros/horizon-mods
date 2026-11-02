// Finalize a TConstruct Tool component through the shared transport contract.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

pub fn finalize_tool(
    stack: &mut ItemStack,
    resolve_block: impl Fn(&str) -> Option<i32>,
) -> Result<(), crate::pluto_tool_codec::ToolError> {
    use crate::pluto_tool_codec::ToolError;
    let Some(tool) = stack.data().and_then(|data| data.other("minecraft:tool")) else {
        // A saved mining mirror without the real component cannot establish
        // that the mirror is a complete, authoritative replacement.
        if stack.data().and_then(|data| data.other("minecraft:custom_data"))
            .and_then(|custom| custom.get("tic_mining_tool")).is_some() {
            return Err(ToolError::InvalidOrUnsupported);
        }
        return Ok(());
    };
    let canonical = crate::pluto_tool_codec::prepare_tool(&horizon_items::item_data::canon_tag(tool), resolve_block)?;
    let (prepared, used) = horizon_world::nbt::parse_snbt(&canonical).map_err(|_| ToolError::InvalidSnbt)?;
    if !canonical[used..].trim().is_empty() { return Err(ToolError::InvalidSnbt); }
    let mirror = stack.data().and_then(|data| data.other("minecraft:custom_data"))
        .and_then(|custom| custom.get("tic_mining_tool"));
    if tool == &prepared && mirror.is_none_or(|value| value == &prepared) { return Ok(()); }
    // A changed cached wire patch may contain components unknown to this port.
    // Preserve the input until the shared transport has a safe mutation path.
    if stack.wire_patch().is_some() { return Err(ToolError::InvalidOrUnsupported); }
    let custom = stack.data().and_then(|data| data.other("minecraft:custom_data")).cloned().map(|mut custom| {
        if let Tag::Compound(ref mut fields) = custom {
            if fields.contains_key(&"tic_mining_tool".into()) {
                fields.insert("tic_mining_tool".into(), prepared.clone());
            }
        }
        custom
    });
    stack.data_mut().set_other("minecraft:tool", Some(prepared));
    if let Some(custom) = custom { stack.data_mut().set_other("minecraft:custom_data", Some(custom)); }
    Ok(())
}

// Check the original snapshot before repair or rebuild changes damage/components.
// Only patches whose contents are known to survive the selected host+adapter
// are allowed to be regenerated; unsupported additions and all removals fail closed.
pub fn patch_safe_for_tool_mutation(stack: &ItemStack) -> bool {
    use horizon_protocol::item::{component, Component, WirePatch};
    use std::collections::HashSet;
    let Some(data) = stack.data() else { return true; };
    let Some(raw) = data.wire.patch() else { return true; };
    if stack.wire_patch().is_none() { return false; }
    let Ok((added, removed)) = WirePatch::decode(raw) else { return false; };
    if !removed.is_empty() { return false; }
    let mut seen = HashSet::new();
    added.iter().all(|(id, value)| {
        if !seen.insert(*id) { return false; }
        match (*id, value) {
            (component::CUSTOM_DATA, Component::Nbt(tag)) => data.other("minecraft:custom_data") == Some(tag),
            (component::MAX_DAMAGE, Component::Int(value)) => matches!(data.other("minecraft:max_damage"), Some(Tag::Int(max)) if max == value),
            (component::DAMAGE, Component::Int(value)) => u32::try_from(*value).ok() == Some(stack.damage),
            (component::UNBREAKABLE, Component::Unit) => data.other("minecraft:unbreakable").is_some(),
            (component::CUSTOM_NAME, Component::Text(tag)) => data.other("minecraft:custom_name") == Some(tag),
            (component::ITEM_NAME, Component::Text(tag)) => data.other("minecraft:item_name") == Some(tag),
            (component::ITEM_MODEL, Component::Ident(value)) => matches!(data.other("minecraft:item_model"), Some(Tag::String(model)) if model.to_string_lossy() == value.as_str()),
            (component::LORE, Component::Lore(lines)) => matches!(data.other("minecraft:lore"), Some(Tag::List(actual)) if actual == lines),
            // The shared Tool decoder must have restored the component; its
            // finalizer will separately validate and canonicalize it.
            (28, Component::Raw(_)) => data.other("minecraft:tool").is_some(),
            _ => false,
        }
    })
}
