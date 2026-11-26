use std::sync::OnceLock;

use sunrise_items::items::ItemStack;

pub type Resolver = fn(&str, &[i32], &[Option<ItemStack>]) -> Option<Vec<Option<[f32; 2]>>>;

static RESOLVER: OnceLock<Resolver> = OnceLock::new();

pub fn set_resolver(resolver: Resolver) {
    let _ = RESOLVER.set(resolver);
}

pub fn resolver() -> Option<Resolver> {
    RESOLVER.get().copied()
}

use super::{Game, OpenNativeScreen};
use sunrise_items::items::ItemStack;

impl OpenNativeScreen {
    pub(super) fn refresh_dynamic_slots(&mut self, stacks: &[Option<ItemStack>]) {
        let Some(resolve) = resolver() else {
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
