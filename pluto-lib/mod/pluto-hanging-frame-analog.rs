// HEAD on GameLevel::item_frame_analog_output: a fancy frame's comparator
// output is its variant's (a 16-step variant reads its rotation, 0-16);
// a frame the registry does not know runs the host's vanilla formula.
fn pluto_hanging_frame_analog(
    level: &GameLevel<'_>,
    pos: BlockPos,
    facing: Direction,
) -> std::ops::ControlFlow<Option<Option<i32>>> {
    let mut frames = level
        .item_frames
        .iter()
        .filter(|f| f.pos == [pos.x, pos.y, pos.z] && f.facing == facing as u8);
    let Some(frame) = frames.next().filter(|_| frames.next().is_none()) else {
        return std::ops::ControlFlow::Continue(());
    };
    // A rotation past the vanilla eight steps only exists on a 16-step
    // variant (the loader and the interact path cap the rest at 7), and
    // its output is the rotation itself.
    if frame.rotation > 7 {
        return std::ops::ControlFlow::Break(Some(Some(
            frame.item.as_ref().map_or(0, |_| i32::from(frame.rotation)),
        )));
    }
    let Some(facts) = crate::game::pluto_hanging_game::known_frame(frame.id) else {
        return std::ops::ControlFlow::Continue(());
    };
    std::ops::ControlFlow::Break(Some(Some(
        horizon_client::entry::pluto_hanging_access::frame_analog(
            &facts,
            frame.item.is_some(),
            i32::from(frame.rotation),
        ),
    )))
}
