// Called only after a validated controller menu has been opened. The native
// advancement pipeline retains actor-specific progress, rewards and packets.
pub(super) fn opened(game: &mut super::Game<'_>, block: &str) {
    use horizon_client::advancements as adv;
    // Other source block-entity types are deliberately not projected yet.
    let container_type = match block {
        "tconstruct:smeltery_controller" => "tconstruct:smeltery",
        "tconstruct:foundry_controller" => "tconstruct:foundry",
        _ => return,
    };
    if game.remote.is_some() { return; }
    let player = game.player_entity();
    game.trigger("tconstruct:block_container_opened", move |conditions, world| {
        let context = adv::Entities {
            this: Some(&player), origin: player.position, tool: None, block_pos: None,
        };
        conditions["type"].as_str() == Some(container_type)
            && adv::conditions(&conditions["player"], &context, world)
    });
}
