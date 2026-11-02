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
