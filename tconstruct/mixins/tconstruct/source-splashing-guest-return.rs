// Complete the deferred operation while this packet still owns LanHost.
fn tconstruct_splashing_guest_return(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, swung: Option<bool>) {
    game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_scope_mut().remove(&conn);
    game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_packet_mut().remove(&conn);
    let Some((stack, slot, mobs, players, creative, level, player_first)) = game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_pending_mut().remove(&conn) else { return; };
    if swung != Some(true) { return; }
    // All checks happen before the first target is mutated. This is still one
    // packet turn: no guest action can interleave with these guest swaps.
    let Some(actor) = host.hub.guests.iter().find(|guest| guest.conn == conn && guest.placed && guest.motion.is_some()) else { return; };
    if players.iter().any(|id| !host.hub.guests.iter().any(|guest| guest.conn == *id && guest.placed && guest.motion.is_some() && guest.dimension == actor.dimension)) { return; }
    if mobs.iter().any(|id| !game.mobs.iter().any(|mob| mob.id == *id && mob.alive() && crate::game::tconstruct_source_spilling::eligible_mob(mob))) { return; }
    let intact = game.as_guest(host, conn, |g| g.stack_in(slot).is_some_and(|held| held.count == stack.count && held.damage == stack.damage && held.same_item_same_components(&stack))).unwrap_or(false);
    if !intact { return; }
    let mut working = stack.clone();
    let mut applied = 0u32;
    enum Target { Mob(u32), Player(u32) }
    let mut ordered = Vec::with_capacity(mobs.len() + players.len());
    let mut players = players.into_iter();
    if player_first { if let Some(primary) = players.next() { ordered.push(Target::Player(primary)); } }
    ordered.extend(mobs.into_iter().map(Target::Mob));
    ordered.extend(players.map(Target::Player));
    for target in ordered {
        match target {
            Target::Mob(id) => {
                let Some(index) = game.mobs.iter().position(|mob| mob.id == id && mob.alive()) else { continue; };
                let result = game.as_guest(host, conn, |g| crate::game::tconstruct_source_spilling::apply(g, index, working.clone(), "tconstruct:splashing", false)).flatten();
                if let Some(drained) = result { working = drained; applied += 1; }
            }
            Target::Player(target) => {
                let mut result = Some(working.clone());
                let changed = game.as_guest(host, target, |g| crate::game::tconstruct_source_slurping::finish_for(g, &working, &mut result, "tconstruct:splashing", None, true)).unwrap_or(false);
                if changed { if let Some(drained) = result { working = drained; applied += 1; } }
            }
        }
    }
    if applied > 0 && !creative {
        // The actor's item has been parked in LanHost throughout target swaps.
        let _ = game.as_guest(host, conn, |g| {
            let same = g.stack_in(slot).is_some_and(|held| held.count == stack.count && held.damage == stack.damage && held.same_item_same_components(&stack));
            if !same { return; }
            *g.stack_in_mut(slot) = Some(working);
            let Some(current) = g.stack_in(slot).cloned() else { return; };
            let wear = applied.saturating_mul(level as u32);
            let (loss, shield) = crate::game::tconstruct_source_overslime_host::damage_with_cause(g, &current, wear, Some("tconstruct:splashing"));
            if let Some(held) = g.stack_in_mut(slot).as_mut() {
                crate::game::tconstruct_source_overslime::spend(held, shield);
                crate::game::tconstruct_tx::fixed_damage(held, loss);
            }
        });
    }
}
