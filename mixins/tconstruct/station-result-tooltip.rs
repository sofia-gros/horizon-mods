fn tconstruct_result_tooltip(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return Continue(()); };
    if game.carried.is_some() { return Continue(()); }
    let Some(slot) = game.slot_at(batch, game.cursor.0, game.cursor.1) else { return Continue(()); };
    if !(if station { (324..=325).contains(&slot) } else { slot == 322 }) { return Continue(()); }
    let Some(BlockEntity::Chest(slots)) = game.container_kind() else { return Break(()); };
    let result = if station {
        let mut copy = slots.clone();
        let mut output = None;
        let _ = crate::game::tconstruct_tx::take(&mut copy, &mut output, true, (slot - 300) as i16);
        output
    } else {
        game.container.and_then(|key| game.world.chunks.__horizon_mixins_tconstruct_part_selections().get(&key).copied()).and_then(|selected| crate::game::tconstruct_tx::part_preview(slots, selected))
    };
    if let Some(result) = result { game.draw_stack_tooltip(batch, &result, game.cursor.0, game.cursor.1); }
    Break(())
}
