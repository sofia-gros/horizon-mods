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
