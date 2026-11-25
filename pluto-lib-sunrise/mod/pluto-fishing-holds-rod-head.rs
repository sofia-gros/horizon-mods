// A hook stays alive while its owner holds a rod: the game's own check names the vanilla rod, and a
// registered mod rod (PlutoLib's fishing registry, asked through the access provider) counts too.
fn pluto_fishing_holds_rod_head(game: &sunrise_client::game::Game<'_>) -> std::ops::ControlFlow<bool> {
    let Some(provider) = crate::entry::pluto_fishing_access::provider() else {
        return std::ops::ControlFlow::Continue(());
    };
    let held = game.held().is_some_and(|stack| (provider.is_rod)(&stack.item));
    let offhand = game.offhand.as_ref().is_some_and(|stack| (provider.is_rod)(&stack.item));
    if held || offhand {
        std::ops::ControlFlow::Break(true)
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
