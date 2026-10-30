fn tconstruct_spill(name: &str) -> std::ops::ControlFlow<PreRemove> {
    if matches!(name, "tconstruct:part_builder" | "tconstruct:tinker_station" |
        "tconstruct:seared_melter" | "tconstruct:seared_heater" | "tconstruct:seared_faucet" |
        "tconstruct:seared_table" | "tconstruct:seared_basin") {
        return std::ops::ControlFlow::Break(PreRemove::Contents { slots: 27, furnace: false });
    }
    std::ops::ControlFlow::Continue(())
}
