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

/// Per-stack maximum stack size: the item's id and the stack's custom data as a JSON object
/// (an empty object for a stack that carries none). None delegates to the item's own maximum.
pub type MaxStackDispatcher = fn(&str, &serde_json::Value) -> Option<u32>;
static MAX_STACK: OnceLock<MaxStackDispatcher> = OnceLock::new();
pub fn set_max_stack_dispatcher(dispatcher: MaxStackDispatcher) -> Result<(), &'static str> {
    MAX_STACK.set(dispatcher).map_err(|_| "item max stack dispatcher already installed")
}
pub fn max_stack(item: &str, custom_data: &serde_json::Value) -> Option<u32> {
    MAX_STACK.get().and_then(|dispatch| dispatch(item, custom_data))
}
