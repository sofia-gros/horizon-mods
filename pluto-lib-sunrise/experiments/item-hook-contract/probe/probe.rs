fn probe_fail(stack: &sunrise_client::items::ItemStack) -> std::ops::ControlFlow<Result<sunrise_client::first_person::UseResult>> {
    if stack.item.as_ref() == "example:probe" {
        std::ops::ControlFlow::Break(Ok(sunrise_client::first_person::UseResult::Fail))
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
