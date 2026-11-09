use super::{Game, OpenNativeScreen};
use horizon_client::items::ItemStack;
use horizon_client::game::pluto_dynamic_screen_slots;

impl OpenNativeScreen {
    pub(super) fn refresh_dynamic_slots(&mut self, stacks: &[Option<ItemStack>]) {
        let Some(resolve) = pluto_dynamic_screen_slots::resolver() else {
            return;
        };
        let items = &stacks[..stacks.len().min(self.screen.slots.len())];
        let Some(slots) = resolve(&self.block, &self.data, items) else {
            return;
        };
        if slots.len() == self.screen.slots.len() {
            self.screen.slots = slots;
        }
    }
}

impl Game<'_> {
    pub(crate) fn refresh_dynamic_screen_slots(&mut self) {
        let Some(stacks) = self.native_menu_stacks() else {
            return;
        };
        if let Some(screen) = self.mod_screen.as_mut() {
            screen.refresh_dynamic_slots(&stacks);
        }
    }
}
