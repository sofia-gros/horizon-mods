fn pluto_structure_command_head(game:&mut Game<'_>,id:&str,context:&Context)->std::ops::ControlFlow<R<i32>> {
    pluto_structure_command::run(game,id,context)
}
