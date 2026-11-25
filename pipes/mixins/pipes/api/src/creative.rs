//! The pipes creative tab: one tab listing every pipe this mod ships, so the pipes can be taken
//! from the creative inventory like any other block. Registered through PlutoLib's
//! `ItemGroupRegistry`, the same public surface Tinkers' Construct registers its tabs through;
//! nothing here reaches into the host.

use crate::common::{EXTRACTION_PIPE, FLUID_EXTRACTION_PIPE, FLUID_PIPE, ITEM_PIPE, NAMESPACE};
use pluto_lib::{Identifier, ItemGroup, ItemGroupRegistry, ItemStack};

fn item(name: &str) -> ItemStack {
    ItemStack::new(Identifier::new(NAMESPACE, name).expect("valid item id"), 1)
}

/// The one pipes tab: the item pipes first, then the fluid pipes, transport before extraction.
pub fn group() -> ItemGroup {
    ItemGroup::new(Identifier::new(NAMESPACE, "pipes").expect("valid group id"), "Pipes", item(ITEM_PIPE))
        .entries([item(ITEM_PIPE), item(EXTRACTION_PIPE), item(FLUID_PIPE), item(FLUID_EXTRACTION_PIPE)])
}

pub fn register() {
    ItemGroupRegistry::register(group());
}
