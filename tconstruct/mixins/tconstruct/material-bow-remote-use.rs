fn tconstruct_remote_bow_has_use(
    info: &horizon_client::items::ItemInfo,
) -> std::ops::ControlFlow<bool> {
    if &*info.id == "tconstruct:longbow" {
        std::ops::ControlFlow::Break(true)
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
