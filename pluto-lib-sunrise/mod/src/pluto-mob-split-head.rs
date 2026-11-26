// Before the host's own split dispatch: PlutoLib split preparation tunes the
// same count/cancel the native hook carries. A smaller tuned count drops the
// constructed tail; a larger one cannot be honoured at this pin (new children
// need host-constructed proposals), so the constructed count stands as the cap.
fn pluto_mob_split_head(game: &mut Game<'_>, mut call: mod_api::__private::MobSplitCall<'_>) {
    let mut count = call.children.len() as u32;
    crate::game::pluto_mob_split_game::prepare(game, call.parent, call.kind, call.size, call.child_size, &mut count, call.cancel);
    while (call.children.len() as u32) > count {
        let last = call.children.len() - 1;
        call.children.remove(last);
    }
}
