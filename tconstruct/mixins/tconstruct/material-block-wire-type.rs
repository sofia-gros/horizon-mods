fn material_block_wire_type(name: &str) -> std::ops::ControlFlow<Option<&'static str>> {
    let kind = match name {
        "tconstruct:fake_storage_block" => "tconstruct:material_block",
        "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil" | "tconstruct:modifier_worktable" => "tconstruct:anvil",
        "tconstruct:seared_melter" => "tconstruct:melter",
        "tconstruct:seared_heater" => "tconstruct:heater",
        "tconstruct:seared_faucet" | "tconstruct:scorched_faucet" => "tconstruct:faucet",
        "tconstruct:seared_table" | "tconstruct:scorched_table" => "tconstruct:table",
        "tconstruct:seared_basin" => "tconstruct:basin",
        "tconstruct:seared_lantern" | "tconstruct:scorched_lantern" => "tconstruct:lantern",
        "tconstruct:seared_casting_tank" => "tconstruct:casting_tank",
        _ => return std::ops::ControlFlow::Continue(()),
    };
    std::ops::ControlFlow::Break(Some(kind))
}
