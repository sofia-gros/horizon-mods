use super::*;

/// The source result is virtual. Send it without generic chest prediction,
/// retaining the current authoritative menu state and carried-stack hash.
pub(crate) fn send(game: &mut Game<'_>) {
    let Some(remote) = game.remote.as_ref() else { return; };
    let Some((window, _)) = remote.menu.as_ref() else { return; };
    let (container, state) = (*window, remote.state_id);
    let carried = game.to_network(&game.carried).map(|stack| Hashed::of(&stack));
    game.send(S::ContainerClick(play::ContainerClick {
        container, state, slot: 22, button: 0, input: 1, changed: Vec::new(), carried,
    }));
}
