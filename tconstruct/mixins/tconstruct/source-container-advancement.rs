// Called only after a validated controller menu has been opened. The native
// advancement pipeline retains actor-specific progress, rewards and packets.
pub(super) fn opened(game: &mut super::Game<'_>, block: &str) {
    use horizon_client::advancements as adv;
    // Other source block-entity types are deliberately not projected yet.
    if block != "tconstruct:smeltery_controller" || game.remote.is_some() { return; }
    let player = game.player_entity();
    game.trigger("tconstruct:block_container_opened", move |conditions, world| {
        let context = adv::Entities {
            this: Some(&player), origin: player.position, tool: None, block_pos: None,
        };
        conditions["type"].as_str() == Some("tconstruct:smeltery")
            && adv::conditions(&conditions["player"], &context, world)
    });
}
