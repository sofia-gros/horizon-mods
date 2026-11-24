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
        let before = self
            .lan
            .as_ref()?
            .hub
            .guests
            .iter()
            .find(|g| g.conn == conn)
            .and_then(|g| g.menu.as_ref())
            .map(|m| m.window);
        let size = items.len();
        let mut host = self.lan.take()?;
        host.hub.open_container(
            conn,
            menu_kind,
            component_nbt(&title.component()),
            (0, horizon_client::lan::menu::VEHICLE_KEY_Y, 0),
            Slots::Native,
            items.clone(),
            Some(extra),
        );
        host.hub.set_native_menu(conn, items);
        host.hub.set_native_store(conn, NativeStore::Input(0..size));
        let after = host
            .hub
            .guests
            .iter()
            .find(|g| g.conn == conn)
            .and_then(|g| g.menu.as_ref())
            .map(|m| m.window);
        self.lan = Some(host);
        // `open_container` returns nothing and leaves the menu alone for an unknown menu type, so
        // an unchanged window is the only sign it did not open (a fresh window id is assigned each
        // open, and the old menu is closed first).
        if after.is_none() || after == before {
            return None;
        }
        after
    }
}
