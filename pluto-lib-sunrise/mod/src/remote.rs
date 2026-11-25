fn pluto_remote(info: &sunrise_client::items::ItemInfo) -> std::ops::ControlFlow<bool> {
    if pluto_item_api::contains(&info.id) { std::ops::ControlFlow::Break(true) } else { std::ops::ControlFlow::Continue(()) }
}
