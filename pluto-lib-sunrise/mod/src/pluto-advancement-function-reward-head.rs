// Advancement function rewards a mod resolved in code: the game's
// function store carries only data pack files, so a function registered
// with PlutoLib is granted here, at the head of the game's own
// reward-function runner, for the player the reward belongs to. A
// function the store does carry keeps the game's path, and an
// unregistered function falls through to the game's silent miss.
fn pluto_advancement_function_reward(
    game: &mut Game<'_>,
    id: &str,
) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let grants = super::pluto_advancement_function_source::resolve(id);
    if grants.is_empty() || game.remote.is_some() {
        return Continue(());
    }
    game.ensure_datapacks();
    if game.fns.store.functions.contains_key(id) {
        return Continue(());
    }
    let tree = game.adv.tree.clone();
    for target in &grants {
        if let Some(&index) = tree.by_id.get(target) {
            game.grant_advancement(index, None);
        }
    }
    Break(())
}
