fn tconstruct_chest_remote_data(game: &mut crate::game::Game<'_>, id: usize, value: i16) -> std::ops::ControlFlow<()> {
    if crate::game::tconstruct_chest_ui::kind(game).is_none() || !matches!(id,30101|30102) { return std::ops::ControlFlow::Continue(()); }
    let Some(pos)=crate::game::remote::tconstruct_source_machine_remote::position(game) else { return std::ops::ControlFlow::Break(()); };
    let Ok(value)=usize::try_from(value) else { return std::ops::ControlFlow::Break(()); };
    if value>256 { return std::ops::ControlFlow::Break(()); }
    let page=game.world.chunks.__horizon_mixins_tconstruct_chest_ui_pages_mut().entry(pos).or_default();
    if id==30101 { page.0=value; } else { page.1=value; }
    std::ops::ControlFlow::Break(())
}
