fn after_stop(value: Result<()>) -> Result<()> {
    crate::game::pluto_server_stopped::finish(value, true)
}
