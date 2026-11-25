// The game half of the menu opens a mod's block use queued (pluto-menu-open, in game.rs):
// settled here, where the game is at hand, in the host's right-click bridge right after
// the dispatch that queued them.
use super::Game;

impl Game<'_> {
    /// Settles the menu opens the acting player's click queued, in the right-click
    /// bridge right after the dispatch: an open asked for the clicked block is applied
    /// through the game's own menu open (the same open the sim's menu effect reaches),
    /// and the click is claimed, as a block use that acted claims it, so nothing else
    /// in the click runs after the menu. The acting player's queued opens are taken
    /// either way, so none outlives its click. On a guest's turn the guest's menu
    /// lives in the LAN host's guest state, which this point has no hold of: the open
    /// is only marked ready for the guest flush at the end of the guest's turn
    /// (pluto-menu-open-guest), and the click is claimed the same way.
    pub(crate) fn settle_queued_menu_open(&mut self, pos: [i32; 3], cancel: &mut bool) {
        let me = horizon_client::lan::playerdata::ints_uuid(self.acting_player_uuid());
        if me != horizon_client::lan::playerdata::ints_uuid(self.level.ensure_player_uuid()) {
            if *cancel {
                horizon_client::game::pluto_menu_open::drop_unready_for(me);
                return;
            }
            let asked = horizon_client::game::pluto_menu_open::has_queued_for(me, pos);
            if asked {
                horizon_client::game::pluto_menu_open::mark_ready_for(me, pos);
                *cancel = true;
            }
            horizon_client::game::pluto_menu_open::drop_unready_for(me);
            return;
        }
        let queued = horizon_client::game::pluto_menu_open::take_queued_for(me);
        if *cancel || self.spectator() {
            return;
        }
        for open in queued {
            if open.pos != pos {
                continue;
            }
            let at = (pos[0], pos[1], pos[2]);
            if self.container == Some(at) {
                *cancel = true;
                continue;
            }
            self.open_menu(pos);
            if self.container == Some(at) {
                *cancel = true;
            }
        }
    }
}
