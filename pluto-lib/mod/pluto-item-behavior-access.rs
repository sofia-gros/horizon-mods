// Shared transport: no host objects or executable-crate dependencies.
use std::sync::OnceLock;
pub type MaskDispatcher = fn(&str, u128, u32) -> Option<bool>;
static MASK: OnceLock<MaskDispatcher> = OnceLock::new();
pub fn set_mask_dispatcher(dispatcher: MaskDispatcher) -> Result<(), &'static str> {
    MASK.set(dispatcher).map_err(|_| "item mask dispatcher already installed")
}
pub fn mask(item: &str, wearer: u128, observer: u32) -> Option<bool> {
    MASK.get().and_then(|dispatch| dispatch(item, wearer, observer))
}
