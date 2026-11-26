fn pluto_living_drops_head(game:&mut Game<'_>,m:&mut Mob,player_uuid:Option<u128>,by_player:bool,
    killer:&sunrise_world::loot::LootEntity,consumed:bool,pos:[f64;3])->core::ops::ControlFlow<()> {
    match crate::game::pluto_living_drops::active() {
        Ok(false)=>return core::ops::ControlFlow::Continue(()),
        Err(error)=>{eprintln!("PlutoLib: death drop registry unavailable; retaining owned counter policy: {error}");},
        Ok(true)=>{},
    }
    game.pluto_drop_death_loot(m,player_uuid,by_player,killer.clone(),consumed,pos);
    core::ops::ControlFlow::Break(())
}
