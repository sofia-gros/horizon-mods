fn pluto_remote(info: &sunrise_items::items::ItemInfo) -> core::ops::ControlFlow<bool> {
    if pluto_lib::pluto_item_api::contains(&info.id) { std::ops::ControlFlow::Break(true) } else { std::ops::ControlFlow::Continue(()) }
}
