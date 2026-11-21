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

/// Per-stack enchantability: the item's id and the stack's custom data as a JSON object
/// (an empty object for a stack that carries none). None delegates to the stack's own
/// `minecraft:enchantable` component and the item default behind it.
pub type EnchantabilityDispatcher = fn(&str, &serde_json::Value) -> Option<i32>;
static ENCHANTABILITY: OnceLock<EnchantabilityDispatcher> = OnceLock::new();
pub fn set_enchantability_dispatcher(dispatcher: EnchantabilityDispatcher) -> Result<(), &'static str> {
    ENCHANTABILITY.set(dispatcher).map_err(|_| "item enchantability dispatcher already installed")
}
pub fn enchantability(item: &str, custom_data: &serde_json::Value) -> Option<i32> {
    ENCHANTABILITY.get().and_then(|dispatch| dispatch(item, custom_data))
}

/// Whether a stack may perform a tool action: the item's id, the stack's custom data as a
/// JSON object (an empty object for a stack that carries none) and the action's name, as
/// NeoForge's `ToolActions` name it (such as `shears_harvest`). None delegates to the
/// game's own answer for the stack.
pub type ToolActionDispatcher = fn(&str, &serde_json::Value, &str) -> Option<bool>;
static TOOL_ACTION: OnceLock<ToolActionDispatcher> = OnceLock::new();
pub fn set_tool_action_dispatcher(dispatcher: ToolActionDispatcher) -> Result<(), &'static str> {
    TOOL_ACTION.set(dispatcher).map_err(|_| "item tool action dispatcher already installed")
}
pub fn tool_action(item: &str, custom_data: &serde_json::Value, action: &str) -> Option<bool> {
    TOOL_ACTION.get().and_then(|dispatch| dispatch(item, custom_data, action))
}
