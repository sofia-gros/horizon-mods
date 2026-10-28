fn receive_part_selection(game: &mut crate::game::Game<'_>, window: i32, slot: i32, item: &Option<ItemStack>) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if slot != 23 || crate::game::tconstruct_ui::kind(game) != Some(false) { return Continue(()); }
    if game.remote.as_ref().and_then(|r| r.menu.as_ref()).is_none_or(|(id, _)| *id != window) { return Continue(()); }
    if let Some(key) = game.container {
        let index = match item.as_ref().map(|s| &*s.item) {
            Some("tconstruct:flint_pick_head") => Some(0),
            Some("tconstruct:wood_handle") => Some(1),
            Some("tconstruct:wood_binding") => Some(2),
            _ => None,
        };
        if let Some(index) = index { game.world.chunks.__horizon_mixins_tconstruct_part_selections_mut().insert(key, index); }
        else { game.world.chunks.__horizon_mixins_tconstruct_part_selections_mut().remove(&key); }
    }
    Break(())
}
