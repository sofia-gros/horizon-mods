fn pluto_use_b2(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    allow_block_actions: bool,
) -> std::ops::ControlFlow<Result<horizon_client::first_person::UseResult>> {
    use horizon_client::first_person::{Hand as HostHand, UseResult};
    use pluto_api::item::{Actor, BlockTarget, Hand, UseContext, UseOutcome};
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote.is_some() || !pluto_api::item::contains(&stack.item) {
        return Continue(());
    }
    let actor = game
        .native_acting_player()
        .map(|p| Actor {
            id: p.uuid,
            dimension: p.dimension,
            position: p.position,
            permission: p.permission,
        })
        .or_else(|| {
            let position = game
                .player
                .as_ref()
                .map(|p| [p.position.x, p.position.y, p.position.z])?;
            let id = horizon_client::lan::playerdata::ints_uuid(game.acting_player_uuid());
            Some(Actor {
                id,
                dimension: game.dim.id().to_owned(),
                position,
                permission: game.ui.registries.permission,
            })
        });
    let context = UseContext {
        actor,
        hand: match game.hand {
            HostHand::Main => Hand::Main,
            HostHand::Off => Hand::Off,
        },
        item_id: stack.item.to_string(),
        count: stack.count,
    };
    let mut data = match crate::game::pluto_item_adapter_b2::read(stack) {
        Ok(data) => data,
        Err(()) => return Break(Ok(UseResult::Fail)),
    };
    let target = if allow_block_actions {
        match game.pick() {
            Some(hit) => Some(BlockTarget {
                position: hit.pos,
                face: hit.face,
            }),
            None => return Break(Ok(UseResult::Fail)),
        }
    } else {
        None
    };
    let mut placement = None;
    let mut outcome = if let Some(target) = target {
        let result = pluto_api::item::dispatch_block(&stack.item, &context, target, &mut data);
        placement = result.placement;
        result.outcome
    } else {
        UseOutcome::Pass
    };
    // The host sends separate UseItemOn / UseItem packets for a LAN guest.
    // The local host executes both parts in this call after a block Pass.
    if outcome == UseOutcome::Pass
        && !matches!(game.guest_use, Some(crate::game::GuestUse::OnBlock(_)))
    {
        outcome = pluto_api::item::dispatch_event(&stack.item, &context, None, &mut data);
    }
    match outcome {
        UseOutcome::Pass => Continue(()),
        UseOutcome::Cancel => Break(Ok(UseResult::Fail)),
        UseOutcome::Handled => {
            let Some(current) = game.held_mut().as_mut().filter(|current| {
                current.count == stack.count && current.same_item_same_components(stack)
            }) else {
                return Break(Ok(UseResult::Fail));
            };
            let mut updated = current.clone();
            if crate::game::pluto_item_adapter_b2::write(&mut updated, &data).is_err() {
                return Break(Ok(UseResult::Fail));
            }
            if let Some(request) = placement {
                use pluto_api::item::{notify_placement, PlacementOutcome};
                let Some(target) = target.filter(|_| context.actor.is_some()) else {
                    notify_placement(&stack.item, PlacementOutcome::Rejected);
                    return Break(Ok(UseResult::Fail));
                };
                let Some((at, state)) =
                    crate::game::pluto_item_placement::prepare(game, target, &request)
                else {
                    notify_placement(&stack.item, PlacementOutcome::Rejected);
                    return Break(Ok(UseResult::Fail));
                };
                if !crate::game::pluto_item_placement::apply(game, at, state) {
                    notify_placement(&stack.item, PlacementOutcome::Rejected);
                    return Break(Ok(UseResult::Fail));
                }
                // The block write has happened. Never return Pass or invite a retry.
                // A raw write committed world effects. Commit the prepared item
                // before later events can fail or replace the destination state.
                let (item_committed, events_ok, final_state_matches) =
                    crate::game::pluto_placement_commit::finish(
                        game,
                        |game| {
                            if let Some(current) = game.held_mut().as_mut().filter(|current| {
                                current.count == stack.count
                                    && current.same_item_same_components(stack)
                            }) {
                                *current = updated;
                                true
                            } else {
                                eprintln!(
                                    "PlutoLib: held item changed during committed block placement"
                                );
                                false
                            }
                        },
                        |game| match game.process_level_events() {
                            Ok(()) => true,
                            Err(error) => {
                                eprintln!("PlutoLib: events failed after committed block placement: {error}");
                                false
                            }
                        },
                        |game| game.world.block(at[0], at[1], at[2]) == state,
                    );
                let result = PlacementOutcome::Applied {
                    final_state_matches,
                    item_committed,
                    events_ok,
                };
                notify_placement(&stack.item, result);
                return Break(Ok(UseResult::Success { swing: true }));
            }
            *current = updated;
            Break(Ok(UseResult::Success { swing: true }))
        }
    }
}
