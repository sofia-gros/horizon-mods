impl super::Game<'_> {
    pub(crate) fn tconstruct_invalidate_travelers_book(&mut self) {
        self.forget_book();
        self.recipe_ui.client_known = None;
        self.recipe_ui.view = Default::default();
        self.recipe_ui.ghost.clear();
        self.recipe_ui.overlay = None;
        self.recipe_ui.last_placed = None;
        self.recipe_ui.last_clicked = None;
    }
}
