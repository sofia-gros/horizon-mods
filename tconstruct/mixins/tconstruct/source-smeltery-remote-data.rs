fn smeltery_remote_data(game: &mut crate::game::Game<'_>, id: usize, value: i16) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if !crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) || !matches!(id, 30002 | 30003 | 30004) { return Continue(()); }
    let Some(pos) = crate::game::tconstruct_source_smeltery_ui::position(game) else { return Break(()); };
    let Ok(value) = usize::try_from(value) else { return Break(()); };
    if id == 30004 {
        if value <= 2 { game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers_mut().entry(pos).or_insert(([None, None], 0)).1 = value as u8; }
        return Break(());
    }
    let page = game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_pages_mut().entry(pos).or_default();
    if id == 30002 { page.0 = value; } else { page.1 = value; }
    Break(())
}
