fn probe_fail(stack: &horizon_client::items::ItemStack) -> std::ops::ControlFlow<Result<horizon_client::first_person::UseResult>> {
    if stack.item.as_ref() == "example:probe" {
        std::ops::ControlFlow::Break(Ok(horizon_client::first_person::UseResult::Fail))
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
