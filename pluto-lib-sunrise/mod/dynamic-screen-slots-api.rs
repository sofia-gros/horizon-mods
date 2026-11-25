use std::sync::OnceLock;

use crate::items::ItemStack;

pub type Resolver = fn(&str, &[i32], &[Option<ItemStack>]) -> Option<Vec<Option<[f32; 2]>>>;

static RESOLVER: OnceLock<Resolver> = OnceLock::new();

pub fn set_resolver(resolver: Resolver) {
    let _ = RESOLVER.set(resolver);
}

pub fn resolver() -> Option<Resolver> {
    RESOLVER.get().copied()
}
