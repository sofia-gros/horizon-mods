// A selected converted frog already completed its inherited finalization and
// equipment before join. The pinned MobEvent caller must not finalize it twice.
fn pluto_entity_spawn_converted(game:&Game<'_>,id:u32)->std::ops::ControlFlow<()> {
    let Some(mob)=game.mobs.iter().find(|mob|mob.id==id) else { return std::ops::ControlFlow::Continue(()) };
    match mob.ai.nav.__horizon_mixins_pluto_lib_pluto_entity_spawn_state().is_consumed() {
        Ok(true)=>std::ops::ControlFlow::Break(()),
        Ok(false)=>std::ops::ControlFlow::Continue(()),
        Err(error)=>{ log::error!("PlutoLib converted spawn state: {error}"); std::ops::ControlFlow::Break(()) },
    }
}
