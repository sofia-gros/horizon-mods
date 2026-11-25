fn pluto_living_attributes_join(game:&mut Game<'_>,dimension:&str,entity:u32,kind:&str,canceled:&mut bool)->std::ops::ControlFlow<()> {
    if crate::game::pluto_living_attributes_join::before_join(game,dimension,entity,kind,canceled) {
        std::ops::ControlFlow::Continue(())
    } else {std::ops::ControlFlow::Break(())}
}
