use super::{Game, lan::LanHost};
use horizon_client::lan::guest::Guest;

pub(super) fn live(game: &mut Game<'_>, host: &mut LanHost, gi: usize) {
    let Some((_, slots, _)) = game.world.chunks.__horizon_mixins_tconstruct_smeltery_guest_containers_mut().remove(&host.hub.guests[gi].conn) else { return; };
    let stacks = slots.into_iter().flatten().collect::<Vec<_>>();
    if !stacks.is_empty() {
        game.lan_place_back(host, gi, stacks);
        host.hub.send_inventory(gi);
    }
}

pub(super) fn left(game: &mut Game<'_>, guest: &mut Guest) {
    let Some((_, slots, _)) = game.world.chunks.__horizon_mixins_tconstruct_smeltery_guest_containers_mut().remove(&guest.conn) else { return; };
    for stack in slots.into_iter().flatten() {
        let max = game.res.items().get(&stack.item).map_or(64, |item| item.max_stack).max(1);
        let remaining = super::tconstruct_tx::source_give_inventory(&mut guest.inventory, 0, stack, max, max > 1);
        if let Some(stack) = remaining { game.spawn_item(stack, guest.position, [0.0, 0.1, 0.0], 40); }
    }
}
