use super::{Game, LanHost};

/// Reuse the normal server container-open path for an authorized tab switch.
pub(crate) fn open(game: &mut Game<'_>, host: &mut LanHost, conn: u32, pos: (i32, i32, i32)) -> bool {
    game.lan_open_container(host, conn, pos)
}
