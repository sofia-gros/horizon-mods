fn tconstruct_spill(name: &str) -> std::ops::ControlFlow<PreRemove> {
    match name {
        "tconstruct:crafting_station" => return std::ops::ControlFlow::Break(PreRemove::Contents { slots: 9, furnace: false }),
        "tconstruct:part_chest" => return std::ops::ControlFlow::Break(PreRemove::Contents { slots: 256, furnace: false }),
        "tconstruct:tinkers_chest" => return std::ops::ControlFlow::Break(PreRemove::Contents { slots: 64, furnace: false }),
        _ => {},
    }
    if matches!(name, "tconstruct:part_builder" | "tconstruct:tinker_station" | "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil" | "tconstruct:modifier_worktable" |
        "tconstruct:seared_melter" | "tconstruct:seared_heater" | "tconstruct:seared_faucet" | "tconstruct:scorched_faucet" |
        "tconstruct:seared_table" | "tconstruct:scorched_table" | "tconstruct:seared_basin" | "tconstruct:smeltery_controller" | "tconstruct:foundry_controller") {
        return std::ops::ControlFlow::Break(PreRemove::Contents { slots: 27, furnace: false });
    }
    std::ops::ControlFlow::Continue(())
}
