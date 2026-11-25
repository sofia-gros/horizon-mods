fn pluto_item_inside_dispatch(game: &mut crate::game::Game<'_>, item: &mut sunrise_client::entities::ItemEntity, escape_speed: Option<f32>, entity_boxes: &[horizon_core::geometry::Aabb], client: bool, splash: &mut dyn FnMut(sunrise_client::entities::ItemSplash)) -> std::ops::ControlFlow<(bool, bool)> {
    std::ops::ControlFlow::Break(crate::game::potions::pluto_contact_sound::item_tick(game, item, escape_speed, entity_boxes, client, splash))
}
