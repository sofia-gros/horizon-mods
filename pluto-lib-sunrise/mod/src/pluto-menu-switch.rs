// The flush of the menu switches a mod's menu button queued during its press
// (pluto-menu-switch-api): applied here, where the game is at hand, at the end of the press
// that asked for them. A switch is the game's own open for the block it names — the same
// `open_mod_menu` a block use runs, then the open container moves to it — for the player
// whose menu this press was on: the game's own player, whose open container the game tracks.
// A guest's press runs through the same dispatch, but a guest's menu lives in the LAN host's
// guest state, which this flush has no hold of: the guest flush (pluto-menu-switch-guest-flush,
// on the LAN station button's return) applies a guest's switch through the host's guest
// container open, mirroring `lan_open_container`'s tail for a mod menu.
use crate::game::Game;
use crate::game::lan::LanHost;
use sunrise_client_lib::lan::menu::Slots;

impl Game<'_> {
    pub(crate) fn apply_queued_menu_switch(&mut self, from: (i32, i32, i32)) {
        // The local player's switches only: during a guest's press the acting player is the
        // guest, whose switches the guest flush applies — taking them here would drop them,
        // because the local container is not the guest's menu.
        let me = sunrise_client_lib::lan::playerdata::ints_uuid(self.acting_player_uuid());
        if me != sunrise_client_lib::lan::playerdata::ints_uuid(self.level.ensure_player_uuid()) {
            return;
        }
        for switch in crate::game::pluto_menu_switch::take_queued_for(me) {
            if switch.from != [from.0, from.1, from.2] {
                continue;
            }
            let to = (switch.to[0], switch.to[1], switch.to[2]);
            if self.container == Some(from) && self.open_mod_menu(to) {
                self.container = Some(to);
            }
        }
    }

    /// A LAN guest's queued switches, applied through the host's guest container open
    /// (`lan_open_container`'s tail for a mod menu at `to`): the veto and start checks run
    /// before the open menu closes, so a refused switch leaves the old menu open.
    pub(crate) fn apply_queued_menu_switch_for_guest(&mut self, host: &mut LanHost, conn: u32) {
        let Some(guest_uuid) = host.hub.guest(conn).map(|g| g.uuid) else { return; };
        for switch in crate::game::pluto_menu_switch::take_queued_for(guest_uuid) {
            let from = (switch.from[0], switch.from[1], switch.from[2]);
            let to = (switch.to[0], switch.to[1], switch.to[2]);
            // The guest's menu must be the one the switch closes.
            if !host.hub.guest(conn).is_some_and(|g| g.menu.as_ref().is_some_and(|m| m.pos == Some(from))) {
                continue;
            }
            let Some(menu) = self.mod_menu_at(to) else { continue; };
            // `RandomizableContainer.unpackLootTable`: a guest opening a block container rolls
            // the table it still holds (no-ops when there is no roll), as the host's own open
            // does.
            let opened = self.res.facts().get(self.world.block(to.0, to.1, to.2)).name.to_string();
            self.unpack_opened_loot(to, &opened);
            let name = &self.res.facts().get(self.world.block(to.0, to.1, to.2)).name;
            let block = name.to_string();
            // The menu provider's getDisplayName: a container's custom name or its default name
            // (a mod block's container is named after its block).
            let key = match sunrise_client_render::containers::default_name_key(name) {
                Some(key) => key.to_owned(),
                None => format!("block.{}", name.replace(':', ".")),
            };
            let title = sunrise_client_render::containers::display_name(self.block_name_plain(to).as_deref(), &key);
            let Some(items) = host.hub.guest(conn).and_then(|g| Slots::Block(to).read(&self.world, g)) else {
                continue;
            };
            // The block entity's createMenu: a shulker box opens a ShulkerBoxMenu (not a 9x3
            // ChestMenu), whose slots refuse shulker boxes.
            let menu = sunrise_client_render::containers::block_menu_type(name).unwrap_or(menu);
            // BaseContainerBlockEntity.createMenu refuses a locked container: no menu opens. A
            // Tinkers table never carries a Lock tag, so this cannot trigger for one; the
            // switch is simply left unapplied, with no locked message or sound.
            let (spectator, hand) = host.hub.guest(conn).map_or((false, None), |g| {
                (g.spectator(), g.inventory.get(g.selected).cloned().flatten())
            });
            let Some((uuid, guest_name)) = host.hub.guest(conn).map(|g| (g.uuid, g.name.to_owned())) else {
                continue;
            };
            let mut locked = self.container_locked_for(to, spectator, hand.as_ref());
            self.native_block_lock_check(uuid, &guest_name, [to.0, to.1, to.2], &mut locked);
            if locked {
                continue;
            }
            // A mod menu type only for a client that has it: as `ServerPlayer.openMenu` with
            // no menu, nothing opens.
            let brand = host.hub.guest(conn).map_or("", |g| g.brand.as_str());
            if !sunrise_client_render::containers::menu_reaches(brand, menu) {
                continue;
            }
            // The mods may stop the menu; the click is spent (the old menu stays open).
            if !self.guest_may_open(host, conn, menu, Some([to.0, to.1, to.2])) {
                continue;
            }
            let mut cancel = false;
            self.native_inventory_block_start(uuid, &guest_name, [to.0, to.1, to.2], &mut cancel);
            if cancel {
                continue;
            }
            // ServerPlayer.openMenu closes the menu that was open.
            self.guest_close_all(host, conn);
            let hidden = self.native_hidden_slots(to, items.len());
            // A mod menu type's block tells the client where it is, with what its mod writes
            // (`openMenu` with an extra-data writer); a stock menu sends nothing more.
            let mut data = None;
            self.native_menu_extra_data(uuid, menu, &block, [to.0, to.1, to.2], &mut data);
            let mut data = data.unwrap_or_default();
            if data.len() > sunrise_protocol::menu_extra::MAX_DATA {
                log::warn!(
                    "Horizon: {block}'s menu extra data ({} bytes) is past {} bytes; sent without it",
                    data.len(),
                    sunrise_protocol::menu_extra::MAX_DATA
                );
                data.clear();
            }
            host.hub.open_container(conn, menu, title, to, Slots::Block(to), items, Some(data));
            if !hidden.is_empty() {
                host.hub.hide_menu_slots(conn, hidden);
            }
        }
    }
}
