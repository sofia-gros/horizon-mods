// Opens a mod menu for a player's item (not a block) as plain values for `Host`
// (`Hub::open_container` is crate-private, so the generated `Host` cannot call it).
impl Game<'_> {
    /// Opens the mod menu type `menu_kind` for the player `uuid`, titled `title`, with `items` in
    /// its slots and `extra` as the menu-extra data the client screen reads. The menu's slots are
    /// the mod's input slots: the player moves items in them by vanilla's click rules, and what
    /// they hold when the menu closes comes back through `native_menu_input_closed`. The menu
    /// opens at a positionless stand-in (`VEHICLE_KEY_Y`, as a vehicle's menu does), so no block
    /// is named for it. The window id, or None for a player not here or an unknown menu type.
    /// API 10.
    pub(in crate::game) fn native_open_item_menu(
        &mut self,
        uuid: u128,
        menu_kind: &str,
        title: Text,
        items: Vec<Option<ItemStack>>,
        extra: Vec<u8>,
    ) -> Option<i32> {
        let conn = self.native_conn(uuid)?;
        // A comparable snapshot of a guest's open menu, for telling a fresh open from the menu
        // that was already there. Window ids cycle 1..=100 (the hub assigns
        // `next_window % 100 + 1`), so a fresh open can land on the very id the previous menu
        // had — about one open in a hundred — and the id alone cannot say whether anything
        // opened (issue #1159). `hidden` is left out of the snapshot: `set_native_store`
        // rewrites it for the menu this open installs.
        let mark = |m: &sunrise_client::lan::menu::Menu| {
            (
                m.window,
                m.kind,
                m.pos,
                m.slots,
                m.size,
                m.state,
                m.sent.clone(),
                m.sent_data.clone(),
                m.drag.clone(),
                m.craft.clone(),
                m.links.len(),
                m.mod_type.is_some(),
            )
        };
        let before = self
            .lan
            .as_ref()?
            .hub
            .guests
            .iter()
            .find(|g| g.conn == conn)
            .and_then(|g| g.menu.as_ref())
            .map(|m| mark(m));
        let size = items.len();
        let mut host = self.lan.take()?;
        host.hub.open_container(
            conn,
            menu_kind,
            component_nbt(&title.component()),
            (0, sunrise_client::lan::menu::VEHICLE_KEY_Y, 0),
            Slots::Native,
            items.clone(),
            Some(extra),
        );
        let opened = host
            .hub
            .guests
            .iter()
            .find(|g| g.conn == conn)
            .and_then(|g| g.menu.as_ref())
            .map(|m| (m.window, mark(m)));
        // `open_container` returns nothing and leaves the menu alone for an unknown menu type,
        // so the open counts only when the guest's menu afterwards differs from the one before
        // (or there was none): a fresh open closes the old menu and installs a new one whose
        // contents were just sent, which the snapshot tells apart even when the window id
        // wrapped onto the same number. Only a counted open has the native menu and store set,
        // so a failed open leaves a menu the guest already had untouched.
        match opened {
            Some((window, after)) if Some(&after) != before.as_ref() => {
                host.hub.set_native_menu(conn, items);
                host.hub.set_native_store(conn, NativeStore::Input(0..size));
                self.lan = Some(host);
                Some(window)
            }
            _ => {
                self.lan = Some(host);
                None
            }
        }
    }
}
