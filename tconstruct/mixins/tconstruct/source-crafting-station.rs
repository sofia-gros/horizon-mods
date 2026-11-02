// Local crafting station sessions use the host crafting menu, while the
// nine persistent inputs remain owned by the block entity.
use super::*;

// Persistent crafting needs the currently disabled click synchronization hook.
pub(super) const PROVISIONAL_ENABLED: bool = false;

pub(super) fn position(game: &Game<'_>) -> Option<(i32, i32, i32)> {
    if game.remote.is_some() || game.lan_acting.is_some() || game.craft_size != 3 { return None; }
    let [x, y, z] = game.craft_table?;
    game.world.chunks.__horizon_mixins_tconstruct_crafting_sessions().contains(&(x, y, z)).then_some((x, y, z))
}

pub(super) fn open(game: &mut Game<'_>, pos: [i32; 3]) -> bool {
    if !PROVISIONAL_ENABLED {
        if game.lan_acting.is_none() { game.message = Some(("Crafting Station is unavailable in this development build".into(), std::time::Instant::now())); }
        return false;
    }
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
    let count = side_items(game,pos)?.len();
    let kind = match &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name {
        "tconstruct:part_chest" => ChestKind::Part,
        "tconstruct:tinkers_chest" => ChestKind::Tinkers,
        _ if matches!(count,5|9|27|54) => ChestKind::Plain(count),
        _ => return None,
    };
    if count != kind.slots() { return None; }
    let offset = game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll().get(&table).map_or(0, |state| state.0);
    Some((pos, count, offset, kind))
}

fn side_parts(game: &Game<'_>,pos:(i32,i32,i32)) -> Option<Vec<(i32,i32,i32)>> {
    let own=game.world.block_entities.get(&pos)?.slots().len();
    match game.chest_partner(pos) {
        Some((other,own_first)) if own==27 && game.world.block_entities.get(&other).is_some_and(|e|e.slots().len()==27) => Some(if own_first {vec![pos,other]} else {vec![other,pos]}),
        Some(_) => None,
        None => Some(vec![pos]),
    }
}

pub(super) fn side_items(game: &Game<'_>,pos:(i32,i32,i32)) -> Option<Vec<Option<ItemStack>>> {
    let mut result=Vec::new();
    for part in side_parts(game,pos)? { result.extend_from_slice(game.world.block_entities.get(&part)?.slots()); }
    Some(result)
}

pub(super) fn write_side(game: &mut Game<'_>,pos:(i32,i32,i32),items:Vec<Option<ItemStack>>) -> bool {
    let Some(parts)=side_parts(game,pos) else {return false;};
    let lengths:Vec<_>=parts.iter().map(|p|game.world.block_entities.get(p).map_or(0,|e|e.slots().len())).collect();
    if lengths.iter().sum::<usize>()!=items.len() || parts.iter().any(|p|!matches!(game.world.block_entities.get(p),Some(BlockEntity::Chest(_)))) {return false;}
    let mut start=0;
    for (part,count) in parts.into_iter().zip(lengths) {
        if let Some(BlockEntity::Chest(slots))=game.world.block_entities.get_mut(&part) {*slots=items[start..start+count].to_vec();}
        game.world.modified.insert((part.0>>4,part.2>>4));start+=count;
    }
    true
}

pub(super) fn remote_side(game: &Game<'_>) -> Option<(usize,usize,super::tconstruct_chest_rules::ChestKind)> {
    use super::tconstruct_chest_rules::ChestKind;
    if game.remote_menu_title().map(|title|title.plain()).as_deref() != Some("TConstruct Crafting Station") { return None; }
    let (window,count,offset,_) = game.world.chunks.__horizon_mixins_tconstruct_crafting_side_client().as_ref()?;
    if *window != game.remote_menu_id() { return None; }
    let kind = match *count {256=>ChestKind::Part,64=>ChestKind::Tinkers,5|9|27|54=>ChestKind::Plain(*count),_=>return None};
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
