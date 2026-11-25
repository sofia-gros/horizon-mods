// tick_creature_entities creates this round immediately after clearing the
// previous charged_exploders wave and before any new entity tick callbacks.
fn pluto_head_wave_reset(value:crate::game::native_ticks::TickRound,game:&Game<'_>)->crate::game::native_ticks::TickRound {
    let spent=game.mobs.iter().filter(|mob|!mob.removed && mob.alive() && mob.species()==sunrise_client::mobs::Species::Creeper
        && !mob.ai.nav.__horizon_mixins_pluto_lib_pluto_entity_spawn_state().head_drop_available()).map(|mob|mob.id);
    if let Err(error)=game.combat.__horizon_mixins_pluto_lib_pluto_living_drop_state().begin_wave(spent) {
        eprintln!("PlutoLib: explosion head state unavailable: {error}");
    }
    value
}
