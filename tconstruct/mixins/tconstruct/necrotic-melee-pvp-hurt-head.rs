fn tconstruct_necrotic_melee_pvp_hurt_head(game: &mut crate::game::Game<'_>, damage_type: &str) {
    if damage_type != "minecraft:player_attack" || game.combat.player_attacker.is_none() { return; }
    let actor_uuid = game.acting_player_uuid();
    if let Some(Some(entry)) = game.world.chunks.__horizon_mixins_tconstruct_necrotic_melee_pvp_mut().last_mut() {
        if entry.2 == actor_uuid { entry.8 = entry.8.saturating_add(1); }
    }
}
