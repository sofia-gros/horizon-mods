fn material_block_wire_type(name: &str) -> std::ops::ControlFlow<Option<&'static str>> {
    let kind = match name {
        "tconstruct:fake_storage_block" => "tconstruct:material_block",
        "tconstruct:seared_melter" => "tconstruct:melter",
        "tconstruct:seared_heater" => "tconstruct:heater",
        "tconstruct:seared_faucet" => "tconstruct:faucet",
        "tconstruct:seared_table" => "tconstruct:table",
        "tconstruct:seared_basin" => "tconstruct:basin",
        _ => return std::ops::ControlFlow::Continue(()),
    };
    std::ops::ControlFlow::Break(Some(kind))
}
