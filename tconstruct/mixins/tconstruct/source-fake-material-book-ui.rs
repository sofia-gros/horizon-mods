impl super::Game<'_> {
    pub(crate) fn tconstruct_invalidate_fake_material_book(&mut self) {
        self.forget_book();
        self.recipe_ui.client_known = None;
        self.recipe_ui.view = Default::default();
        self.recipe_ui.ghost.clear();
        self.recipe_ui.overlay = None;
        self.recipe_ui.last_placed = None;
        self.recipe_ui.last_clicked = None;
    }

    pub(crate) fn tconstruct_preview_fake_material_recipe(
        &mut self, book_type: horizon_client::recipe_book::BookType, entry_index: usize,
    ) -> bool {
        if self.remote.is_some() || book_type != horizon_client::recipe_book::BookType::Crafting {
            return false;
        }
        let Some(entry) = self.recipe_ui.book.as_ref()
            .and_then(|book| book.entries.get(entry_index)).cloned() else { return false; };
        if !entry.id.starts_with("tconstruct:fake_metal_native/")
            || entry.stacks.is_empty() || entry.display_id < 0
            || !entry.fits(self.craft_size, self.craft_size) { return false; }
        // Preview only, including shift-click. Do not move or clear any slot.
        self.fill_ghost(book_type, &entry);
        true
    }
}
