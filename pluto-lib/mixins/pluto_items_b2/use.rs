fn pluto_use_b2(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    allow_block_actions: bool,
) -> std::ops::ControlFlow<Result<horizon_client::first_person::UseResult>> {
    use horizon_client::first_person::{Hand as HostHand, UseResult};
    use pluto_item_api_b2::{Actor, BlockTarget, Hand, UseContext, UseOutcome};
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote.is_some() || !pluto_item_api_b2::contains(&stack.item) {
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
    let mut outcome = if let Some(target) = target {
        pluto_item_api_b2::dispatch_event(&stack.item, &context, Some(target), &mut data)
    } else {
        UseOutcome::Pass
    };
    // The host sends separate UseItemOn / UseItem packets for a LAN guest.
    // The local host executes both parts in this call after a block Pass.
    if outcome == UseOutcome::Pass
        && !matches!(game.guest_use, Some(crate::game::GuestUse::OnBlock(_)))
    {
        outcome = pluto_item_api_b2::dispatch_event(&stack.item, &context, None, &mut data);
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
            *current = updated;
            Break(Ok(UseResult::Success { swing: true }))
        }
    }
}
