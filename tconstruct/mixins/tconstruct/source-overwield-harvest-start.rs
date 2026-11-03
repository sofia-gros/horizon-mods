// Run after native cancellation handlers and before actual block removal.
// Area recursion spends only on its first allowed call.
fn tconstruct_overwield_harvest_start(game: &mut crate::game::Game<'_>, canceled: bool) {
    super::tconstruct_native_hook_bodies::tconstruct_overwield_harvest_start(game, canceled);
}
