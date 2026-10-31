// Spill inputs removed by a shrinking Smeltery at the controller, not at the
// player. The state update is committed before this is called.
use crate::game::Game;
use horizon_client::items::ItemStack;
use horizon_core::position::BlockPos;

pub(in crate::game) fn spill(game: &mut Game<'_>, pos: (i32, i32, i32), items: Vec<ItemStack>) {
    if items.is_empty() { return; }
    game.with_level(|_, level, _| {
        let pos = BlockPos::new(pos.0, pos.1, pos.2);
        for stack in items { level.drop_item_stack(pos, Some(stack)); }
    });
}
