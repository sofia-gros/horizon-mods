// After split_dead_cube spawned the children: PlutoLib inheritance dresses
// them before the world ticks on.
fn pluto_mob_split_children(value:(),game:&mut Game<'_>,m:&Mob) {
    let _=value;
    crate::game::pluto_mob_split_game::inherit(game,m);
}
