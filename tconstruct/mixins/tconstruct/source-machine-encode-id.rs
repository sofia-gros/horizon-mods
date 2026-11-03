fn tconstruct_machine_encode_id(entity: &crate::containers::BlockEntity, block: &str) -> std::ops::ControlFlow<String> {
    if !matches!(entity, crate::containers::BlockEntity::Chest(_)) { return std::ops::ControlFlow::Continue(()); }
    let id = match block {
        "tconstruct:crafting_station" => "tconstruct:crafting_station",
        "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil" | "tconstruct:modifier_worktable" => "tconstruct:anvil",
        "tconstruct:part_chest" => "tconstruct:part_chest",
        "tconstruct:tinkers_chest" => "tconstruct:modifier_chest",
        "tconstruct:seared_melter" => "tconstruct:melter",
        "tconstruct:seared_heater" => "tconstruct:heater",
        "tconstruct:seared_faucet" | "tconstruct:scorched_faucet" => "tconstruct:faucet",
        "tconstruct:seared_table" => "tconstruct:table",
        "tconstruct:seared_basin" => "tconstruct:basin",
        "tconstruct:smeltery_controller" | "tconstruct:foundry_controller" => "tconstruct:smeltery",
        "tconstruct:seared_fuel_tank" | "tconstruct:seared_fuel_gauge" | "tconstruct:scorched_fuel_tank" | "tconstruct:scorched_fuel_gauge" => "tconstruct:fuel_tank",
        "tconstruct:seared_ingot_tank" | "tconstruct:seared_ingot_gauge" | "tconstruct:scorched_ingot_tank" | "tconstruct:scorched_ingot_gauge" => "tconstruct:ingot_tank",
        _ => return std::ops::ControlFlow::Continue(()),
    };
    std::ops::ControlFlow::Break(id.to_owned())
}
