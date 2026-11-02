// Player-victim attacks take the existing PvP path rather than weapon_hit.
// Apply the source material-tool wear once only after that hit actually lands.
fn tconstruct_pvp_melee_wear(value:bool, game:&mut crate::game::Game<'_>, host:&mut crate::game::lan::LanHost, strike:&Strike) -> bool {
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
