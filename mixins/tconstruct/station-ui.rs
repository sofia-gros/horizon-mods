use super::*;
pub(super) fn kind(game: &Game<'_>) -> Option<bool> {
    if game.remote.is_some() {
        return match game.remote_menu_title().map(|t| t.plain()).as_deref() {
            Some("TConstruct Part Builder") => Some(false),
            Some("TConstruct Tinker Station") => Some(true),
            _ => None,
        };
    }
    let key = game.container?;
    match &*game.res.facts().get(game.world.block(key.0, key.1, key.2)).name {
        "tconstruct:part_builder" => Some(false),
        "tconstruct:tinker_station" => Some(true),
        _ => None,
    }
}

/// Select the first adjacent container that fits the existing menu protocol.
pub(super) fn side_inventory(game: &Game<'_>, key: (i32, i32, i32)) -> Option<(i32, i32, i32)> {
    [(0, -1), (1, 0), (0, 1), (-1, 0)].into_iter().find_map(|(dx, dz)| {
        let pos = (key.0 + dx, key.1, key.2 + dz);
        let name = &game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name;
        if matches!(&**name, "tconstruct:part_builder" | "tconstruct:tinker_station") || game.chest_partner(pos).is_some() { return None; }
        match game.world.block_entities.get(&pos) {
            Some(BlockEntity::Chest(items)) if matches!(items.len(), 5 | 9 | 27) => Some(pos),
            _ => None,
        }
    })
}
