fn material_block_wire_type(name: &str) -> std::ops::ControlFlow<Option<&'static str>> {
    if name == "tconstruct:fake_storage_block" {
        std::ops::ControlFlow::Break(Some("tconstruct:material_block"))
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
