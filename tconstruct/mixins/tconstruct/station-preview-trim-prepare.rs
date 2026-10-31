// Add the selected Station preview trim to Horizon's existing trim queue.
fn station_preview_trim_prepare(game: &crate::game::Game<'_>, wanted: &mut Vec<TrimTexture>) {
    if crate::game::tconstruct_ui::kind(game) != Some(true) { return; }
    let Some(stack) = crate::game::armor_stands::tconstruct_armor_preview::selected(game) else { return; };
    if let Some(trim) = crate::game::armor_stands::tconstruct_armor_preview::source_trim(game, &stack) {
        if !wanted.contains(&trim) { wanted.push(trim); }
    }
}
