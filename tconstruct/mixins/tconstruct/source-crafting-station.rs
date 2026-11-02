// Local crafting station sessions use the host crafting menu, while the
// nine persistent inputs remain owned by the block entity.
use super::*;

pub(super) fn position(game: &Game<'_>) -> Option<(i32, i32, i32)> {
    if game.remote.is_some() || game.lan_acting.is_some() || game.craft_size != 3 { return None; }
    let [x, y, z] = game.craft_table?;
    game.world.chunks.__horizon_mixins_tconstruct_crafting_sessions().contains(&(x, y, z)).then_some((x, y, z))
}

pub(super) fn open(game: &mut Game<'_>, pos: [i32; 3]) -> bool {
    if game.remote.is_some() { return false; }
    game.ensure_datapacks();
    if game.lan_acting.is_some() {
        // as_guest hands this request to the host's LAN crafting opener.
        game.craft_size = 3;
        game.craft_table = Some(pos);
        return true;
    }
    let key = (pos[0], pos[1], pos[2]);
    let entity = game.world.block_entities.get_or_insert_with(key, || BlockEntity::Chest(vec![None; 9]));
    let BlockEntity::Chest(items) = entity else { return false; };
    if items.len() != 9 { return false; }
    game.craft_grid.clone_from(items);
    game.world.chunks.__horizon_mixins_tconstruct_crafting_sessions_mut().insert(key);
    game.craft_size = 3;
    game.craft_table = Some(pos);
    game.screen = Screen::Inventory;
    game.capture(false);
    game.right = false;
    true
}

pub(super) fn sync(game: &mut Game<'_>) {
    let Some(pos) = position(game) else { return; };
    if &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name != "tconstruct:crafting_station" { return; }
    let Some(BlockEntity::Chest(items)) = game.world.block_entities.get_mut(&pos) else { return; };
    if items.len() != 9 || game.craft_grid.len() != 9 { return; }
    *items = game.craft_grid.clone();
    game.world.modified.insert((pos.0 >> 4, pos.2 >> 4));
}

pub(super) fn refresh(game: &mut Game<'_>) {
    let Some(pos) = position(game) else { return; };
    if &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name != "tconstruct:crafting_station" { return; }
    let Some(BlockEntity::Chest(items)) = game.world.block_entities.get(&pos) else { return; };
    if items.len() == 9 { game.craft_grid.clone_from(items); }
}

pub(super) fn close(game: &mut Game<'_>) {
    let Some(pos) = position(game) else { return; };
    sync(game);
    // The host's vanilla close returns its temporary grid to the player.
    // This grid is a view of block-owned slots and must stay in the block.
    game.craft_grid.fill(None);
    game.world.chunks.__horizon_mixins_tconstruct_crafting_sessions_mut().remove(&pos);
}
