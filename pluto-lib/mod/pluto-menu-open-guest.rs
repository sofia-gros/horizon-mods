// The guest half of the menu opens a mod's block use queued (pluto-menu-open, in game.rs):
// a LAN guest's menu lives in the LAN host's guest state, so the guests' opens are
// applied here, at the end of a guest's turn, through the host's own guest container
// open — only the opens a guest's click left ready (nothing else claimed it), and only
// for a guest with no menu open already (a station's, through the sim, stands). This
// module sits in lan_actions.rs so it can call that open, which is private to this file.
use super::Game;

impl Game<'_> {
    pub(crate) fn apply_ready_menu_opens_for_guests(&mut self, host: &mut crate::game::lan::LanHost) {
        for open in horizon_client::game::pluto_menu_open::take_all_ready() {
            let Some(uuid) = open.player else { continue };
            let Some(conn) = host.hub.guests.iter().find(|guest| guest.uuid == uuid && guest.menu.is_none()).map(|guest| guest.conn) else { continue };
            self.lan_open_container(host, conn, (open.pos[0], open.pos[1], open.pos[2]));
        }
    }
}
