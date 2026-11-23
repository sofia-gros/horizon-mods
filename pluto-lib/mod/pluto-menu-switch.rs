// The flush of the menu switches a mod's menu button queued during its press
// (pluto-menu-switch-api): applied here, where the game is at hand, at the end of the press
// that asked for them. A switch is the game's own open for the block it names — the same
// `open_mod_menu` a block use runs, then the open container moves to it — for the player
// whose menu this press was on: the game's own player, whose open container the game tracks.
// A guest's press runs through the same dispatch, but a guest's menu lives in the LAN host's
// guest state, which this flush has no hold of; a switch queued by a guest's press whose
// position the game's own player does not have open is dropped with the rest of the queue.
use super::Game;

impl Game<'_> {
    pub(crate) fn apply_queued_menu_switch(&mut self, from: (i32, i32, i32)) {
        for switch in horizon_client::game::pluto_menu_switch::take_queued() {
            if switch.from != [from.0, from.1, from.2] {
                continue;
            }
            let to = (switch.to[0], switch.to[1], switch.to[2]);
            if self.container == Some(from) && self.open_mod_menu(to) {
                self.container = Some(to);
            }
        }
    }
}
