// Private native boundary. Public event contracts live in the shared API crate.
use horizon_client::pluto_inventory::advancements::events::{Event, EventSink, Submission};

struct GameEvents<'a, 'g>(&'a mut super::Game<'g>);
impl EventSink for GameEvents<'_, '_> {
    fn submit(&mut self, event: Event<'_>) -> Submission {
        use horizon_client::advancements as adv;
        let game = &mut *self.0;
        if game.remote.is_some() {
            return Submission::RemoteClient;
        }
        if game.no_player_here() {
            return Submission::NoActor;
        }
        // Load first so player_entity_for can see the criterion's requested
        // player facts/NBT. This uses the actor currently selected by the host.
        game.load_advancements();
        let player = game.player_entity_for(event.trigger);
        game.trigger(event.trigger, |conditions, world| {
            let context = adv::Entities {
                this: Some(&player),
                origin: player.position,
                tool: None,
                block_pos: None,
            };
            (event.matches)(conditions, event.payload)
                && adv::conditions(&conditions["player"], &context, world)
        });
        Submission::Submitted
    }
}

/// Supply a synchronous sink for the host's current actor. This neither changes
/// actor selection nor emits an event by itself. Call within an existing guest
/// acting scope when appropriate, never after that scope has been restored.
pub(crate) fn with_game<R>(
    game: &mut super::Game<'_>,
    run: impl FnOnce(&mut dyn EventSink) -> R,
) -> R {
    run(&mut GameEvents(game))
}
