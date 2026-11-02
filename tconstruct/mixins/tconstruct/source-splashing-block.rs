fn tconstruct_splashing_block(game: &mut crate::game::Game<'_>, stack: &horizon_items::items::ItemStack, pos: [i32; 3], face: [i32; 3]) -> std::ops::ControlFlow<horizon_client::Result<bool>> {
    match crate::game::tconstruct_source_splashing::block(game, stack, pos, face) {
        Some(result) => std::ops::ControlFlow::Break(result),
        None => std::ops::ControlFlow::Continue(()),
    }
}
