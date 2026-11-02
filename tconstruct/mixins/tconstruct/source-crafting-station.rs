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
    let side = super::tconstruct_ui::side_inventory(game, key, true);
    if let Some(side) = side { game.world.chunks.__horizon_mixins_tconstruct_builder_side_mut().insert(key, side); }
    else { game.world.chunks.__horizon_mixins_tconstruct_builder_side_mut().remove(&key); }
    game.world.chunks.__horizon_mixins_tconstruct_crafting_sessions_mut().insert(key);
    game.craft_size = 3;
    game.craft_table = Some(pos);
    game.screen = Screen::Inventory;
    game.capture(false);
    game.right = false;
    true
}

pub(super) fn side(game: &Game<'_>) -> Option<((i32, i32, i32), usize, usize, super::tconstruct_chest_rules::ChestKind)> {
    use super::tconstruct_chest_rules::ChestKind;
    if let Some((count,offset,kind)) = remote_side(game) { return Some(((0,0,0),count,offset,kind)); }
    let table = position(game)?;
    let pos = *game.world.chunks.__horizon_mixins_tconstruct_builder_side().get(&table)?;
    let count = game.world.block_entities.get(&pos)?.slots().len();
    let kind = match &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name {
        "tconstruct:part_chest" => ChestKind::Part,
        "tconstruct:tinkers_chest" => ChestKind::Tinkers,
        _ if matches!(count,5|9|27) && game.chest_partner(pos).is_none() => ChestKind::Plain(count),
        _ => return None,
    };
    if count != kind.slots() { return None; }
    let offset = game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll().get(&table).map_or(0, |state| state.0);
    Some((pos, count, offset, kind))
}

pub(super) fn remote_side(game: &Game<'_>) -> Option<(usize,usize,super::tconstruct_chest_rules::ChestKind)> {
    use super::tconstruct_chest_rules::ChestKind;
    if game.remote_menu_title().map(|title|title.plain()).as_deref() != Some("TConstruct Crafting Station") { return None; }
    let (window,count,offset,_) = game.world.chunks.__horizon_mixins_tconstruct_crafting_side_client().as_ref()?;
    if *window != game.remote_menu_id() { return None; }
    let kind = match *count {256=>ChestKind::Part,64=>ChestKind::Tinkers,5|9|27=>ChestKind::Plain(*count),_=>return None};
    Some((*count,*offset,kind))
}

pub(super) fn set_scroll(game: &mut Game<'_>,offset: usize) {
    if remote_side(game).is_some() {
        game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick {container:game.remote_menu_id(),button:43000+offset as i32}));
    } else if let Some(pos)=position(game) {
        game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll_mut().entry(pos).or_default().0=offset;
    }
}

pub(super) fn scroll_key(game: &Game<'_>) -> Option<(i32,i32,i32)> {
    if remote_side(game).is_some() { Some((i32::MIN,game.remote_menu_id(),0)) } else { position(game) }
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
    // The host's vanilla close returns its temporary grid to the player.
    // Each completed transaction already updated the block. Do not overwrite
    // it here: a LAN guest may have edited it since this view last refreshed.
    game.craft_grid.fill(None);
    game.world.chunks.__horizon_mixins_tconstruct_crafting_sessions_mut().remove(&pos);
    game.world.chunks.__horizon_mixins_tconstruct_builder_side_mut().remove(&pos);
    game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll_mut().remove(&pos);
}
