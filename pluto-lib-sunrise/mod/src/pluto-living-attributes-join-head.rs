fn pluto_living_attributes_join(game:&mut Game<'_>,dimension:&str,subject:mod_api::EntityRef,kind:&str,canceled:&mut bool)->core::ops::ControlFlow<()> {
    if crate::game::pluto_living_attributes_join::before_join(game,dimension,subject.id(),kind,canceled) {
        core::ops::ControlFlow::Continue(())
    } else {core::ops::ControlFlow::Break(())}
}
