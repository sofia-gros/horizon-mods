use super::Game;

pub(super) fn is_smeltery(game: &Game<'_>) -> bool {
    if game.remote.is_some() {
        return game.remote_menu_title().map(|title| title.plain()).as_deref() == Some("TConstruct Smeltery");
    }
    game.container.is_some_and(|pos| &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name == "tconstruct:smeltery_controller")
}

pub(super) fn position(game: &Game<'_>) -> Option<(i32, i32, i32)> {
    if game.remote.is_some() { super::remote::tconstruct_source_machine_remote::position(game) } else { game.container }
}

pub(super) fn page(game: &Game<'_>) -> (usize, usize) {
    let Some(pos) = position(game) else { return (0, 0); };
    if game.remote.is_some() {
        game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages().get(&pos).copied().unwrap_or_default()
    } else {
        let count = super::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
            .and_then(|tag| super::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)).map_or(0, |state| state.slots.len());
        let offset = game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages().get(&pos).map_or(0, |page| page.1);
        (count, offset)
    }
}
