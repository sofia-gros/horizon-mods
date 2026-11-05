fn tconstruct_dreambound_jump_input(
    motion: &mut crate::motion::MotionWorld<'_, '_, '_>,
    jumping: bool,
) -> std::ops::ControlFlow<Result<()>> {
    match crate::motion::tconstruct_dreambound_jump_motion::input(motion, jumping) {
        Ok(()) => std::ops::ControlFlow::Continue(()),
        Err(error) => std::ops::ControlFlow::Break(Err(error)),
    }
}
