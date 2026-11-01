use super::*;

/// The Anvil also names a result when modifying an existing tool.
pub(super) fn available(game: &Game<'_>) -> bool {
    if super::tconstruct_ui::kind(game) != Some(true) { return false; }
    let Some(entity) = game.container_kind() else { return false; };
    let operation = super::tconstruct_source_station_world::has_operation_tool(entity.slots());
    let capacity = super::tconstruct_ui::station_capacity(game);
    if operation && capacity != 5 { return false; }
    let output = if game.remote.is_some() {
        entity.slots().get(if operation { 25 } else { 24 }).and_then(Option::as_ref).cloned()
    } else {
        if operation { super::tconstruct_source_station_world::operation_plan_with_capacity(&game.world, entity.slots(), capacity).map(|plan| plan.output) }
        else { super::tconstruct_source_station_world::building_plan_with_capacity(&game.world, entity.slots(), capacity).map(|(_, output)| output) }
    };
    output.as_ref().is_some_and(|stack| operation || super::tconstruct_ui::station_accepts_output(game, stack))
}

pub(super) fn new_box(ox: f32, oy: f32) -> horizon_client::ui::EditBox {
    let mut edit = horizon_client::ui::EditBox::new(ox + 80.0, oy + 7.0, 82.0, 9.0);
    edit.max_length = 50;
    edit.bordered = false;
    edit.focused = false;
    edit
}

pub(super) fn focused(game: &Game<'_>) -> bool {
    available(game) && game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_station_name_edit().get(&pos)).is_some_and(|edit| edit.focused)
}

pub(super) fn send(game: &Game<'_>, name: String) {
    if game.remote.is_some() {
        game.send(horizon_protocol::packets::play::Serverbound::RenameItem(horizon_protocol::packets::play::RenameItem { name }));
    }
}
