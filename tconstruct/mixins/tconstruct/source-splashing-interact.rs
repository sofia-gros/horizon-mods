fn tconstruct_splashing_interact(game: &mut crate::game::Game<'_>, entity: i32, stack: Option<&horizon_client::items::ItemStack>, claimed: &mut bool) {
    crate::game::tconstruct_source_splashing::interact(game, entity, stack, claimed);
}
