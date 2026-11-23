// After the host's own split dispatch: PlutoLib split preparation tunes the
// same count/cancel the native hook carries.
fn pluto_mob_split_head(
    value:(),
    game:&mut Game<'_>,
    parent:u32,
    kind:&str,
    size:i32,
    child_size:i32,
    count:&mut u32,
    cancel:&mut bool,
) {
    let _=value;
    crate::game::pluto_mob_split_game::prepare(game,parent,kind,size,child_size,count,cancel);
}
