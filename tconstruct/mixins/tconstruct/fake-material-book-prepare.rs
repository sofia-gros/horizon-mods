fn tconstruct_fake_material_book_prepare(mut value: Result<PreparedWorld>) -> Result<PreparedWorld> {
    if let Ok(prepared) = &mut value {
        // The loaded book is still unfiltered, and this ChunkMap moves with
        // this exact PreparedWorld. Failed preparation stores nothing.
        *prepared.world.chunks.__horizon_mixins_tconstruct_fake_material_book_pending_mut() =
            horizon_client::recipe_book::tconstruct_source_fake_material_book_load::capture(
                &prepared.level.recipe_book);
    }
    value
}
