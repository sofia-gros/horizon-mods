// The hopper placement template points horizontal facings toward its support;
// the source faucet points away from it. Vertical placement already agrees.
fn tconstruct_faucet_placement_facing(game: &mut crate::game::Game<'_>, pos: [i32; 3], state: horizon_world::registry::BlockStateId, stack: &horizon_client::items::ItemStack) -> std::ops::ControlFlow<Result<horizon_world::registry::BlockStateId>> {
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote.is_some() || &*stack.item != "tconstruct:seared_faucet" { return Continue(()); }
    if &*game.res.facts().get(state).name != "tconstruct:seared_faucet" { return Continue(()); }
    let Some(block) = game.pack.assets.registry.state(state) else { return Continue(()); };
    let opposite = match block.properties.get("facing").map(String::as_str) {
        Some("north") => "south", Some("south") => "north", Some("east") => "west", Some("west") => "east", _ => return Continue(()),
    };
    let Some(new) = game.with_props(state, &[("facing", opposite)]) else { return Continue(()); };
    if let Err(error) = game.level_set(pos, new) { return Break(Err(error)); }
    let _ = crate::game::tconstruct_source_machine_runtime::ensure(game, (pos[0], pos[1], pos[2]), "tconstruct:seared_faucet");
    Break(Ok(new))
}
