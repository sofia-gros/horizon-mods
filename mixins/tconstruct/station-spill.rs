fn tconstruct_spill(name: &str) -> std::ops::ControlFlow<PreRemove> {
    if name == "tconstruct:part_builder" || name == "tconstruct:tinker_station" {
        return std::ops::ControlFlow::Break(PreRemove::Contents { slots: 27, furnace: false });
    }
    std::ops::ControlFlow::Continue(())
}
