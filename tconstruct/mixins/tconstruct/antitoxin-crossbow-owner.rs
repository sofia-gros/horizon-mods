fn tconstruct_antitoxin_crossbow_owner(
    mut value: horizon_assets::item_model::OwnerState,
    game: &crate::game::Game<'_>, stack: Option<&horizon_client::items::ItemStack>,
) -> horizon_assets::item_model::OwnerState {
    if value.using_this_item {
        if let Some(stack) = stack.filter(|stack| stack.item.as_ref() == "tconstruct:crossbow") {
            if let Some(bonus) = crate::game::tconstruct_antitoxin_world::draw_bonus(game, stack) {
                value.crossbow_charge_duration = crate::game::tconstruct_ranged_draw::draw_ticks_with_bonus(stack, bonus).max(1) as i32;
            }
        }
    }
    value
}
