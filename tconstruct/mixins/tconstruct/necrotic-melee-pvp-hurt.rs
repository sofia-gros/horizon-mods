fn tconstruct_necrotic_melee_pvp_hurt(value: bool, game: &mut crate::game::Game<'_>, damage_type: &str) -> bool {
    if damage_type != "minecraft:player_attack" || game.combat.player_attacker.is_none() { return value; }
    let Some(health) = game.player.as_ref().map(|player| player.health.health()) else { return value; };
    let actor_uuid = game.acting_player_uuid();
    if let Some(Some(entry)) = game.world.chunks.__horizon_mixins_tconstruct_necrotic_melee_pvp_mut().last_mut() {
        if entry.2 == actor_uuid && entry.8 > 0 {
            entry.8 -= 1;
            if entry.8 == 0 && value && entry.7.is_none() && health.is_finite() { entry.7 = Some(health); }
        }
    }
    value
}
