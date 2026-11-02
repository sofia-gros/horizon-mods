// Player-victim attacks take the existing PvP path rather than weapon_hit.
// Apply the source material-tool wear once only after that hit actually lands.
fn tconstruct_pvp_melee_wear(value:bool, game:&mut crate::game::Game<'_>, host:&mut crate::game::lan::LanHost, strike:&Strike) -> bool {
    let pending = game.world.chunks.__horizon_mixins_tconstruct_necrotic_melee_pvp_mut().pop().flatten();
    if let Some((identity, victim_conn, victim_uuid, health_before, attacker, attacker_uuid, before)) = pending {
        let target_health = host.hub.guest(victim_conn).filter(|g| horizon_client::lan::playerdata::uuid_ints(g.uuid) == victim_uuid)
            .and_then(|g| g.motion.as_deref().map(|p| p.health.health()));
        let data = game.res.mob_data();
        let allowed = data.entity_tags.contains_key("tconstruct:necrotic_blacklist") && !data.entity_has("tconstruct:necrotic_blacklist", "minecraft:player");
        if value && identity == strike as *const Strike as usize && attacker == strike.attacker && allowed {
            if let Some(dealt) = target_health.map(|after| health_before - after) {
                let heal = |g: &mut crate::game::Game<'_>| {
                    if g.acting_player_uuid() != attacker_uuid { return; }
                    let Some(stack) = g.held().cloned() else { return; };
                    let mut old = before.clone(); old.damage = stack.damage;
                    if old.count != stack.count || stack.count != 1 || !old.same_item_same_components(&stack) || stack.wire_patch().is_some() || crate::game::tconstruct_source_melee_shape::intact(&stack).is_none() { return; }
                    let Some(traits) = crate::game::tconstruct_source_material_traits::levels(&g.world, &stack) else { return; };
                    let level = traits.levels.get("tconstruct:necrotic").copied().unwrap_or(0);
                    if let Some(amount) = crate::game::tconstruct_source_necrotic::heal_amount(level, dealt, strike.scale, strike.crit, true) {
                        crate::game::tconstruct_source_necrotic::heal_player(g, amount);
                    }
                };
                match attacker {
                    Some(conn) => { let _ = game.as_guest(host, conn, heal); },
                    None => heal(game),
                }
            }
        }
    }
    if !value {return value;}
    fn wear(game:&mut crate::game::Game<'_>) {
        let Some(stack)=game.held().cloned() else{return;};
        if crate::game::tconstruct_source_melee_shape::builtin_attack_wear(stack.item.as_ref()).is_some()
            && crate::game::tconstruct_source_melee_shape::intact(&stack).is_some() {
            game.weapon_wear(&stack);
        }
    }
    match strike.attacker {
        Some(conn)=>{game.as_guest(host,conn,wear);},
        None=>wear(game),
    }
    value
}
