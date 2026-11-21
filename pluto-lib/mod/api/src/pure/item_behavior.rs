//! General item behavior registration, separate from use-over-time behavior.
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use super::base::Identifier;
use super::player_api::StackData;

/// Minecraft/NeoForge item ender-mask query. Missing host data is explicit.
#[derive(Clone, Debug)]
pub struct EnderMaskContext {
    pub item: String,
    /// The worn helmet as of the wearer's last tick, when it carries custom data; the native AI snapshot itself has only its ID.
    pub stack: Option<StackData>,
    pub wearer_uuid: Option<u128>,
    pub observer_entity: Option<u32>,
}

/// A per-stack maximum stack size query (Minecraft: `ItemStack.getMaxStackSize`, NeoForge's
/// stack-sensitive override): the most items one stack of this item may hold, for this stack.
/// A filled container stacks lower than an empty one (the original's tank items: 16 filled,
/// the item's own 64 empty), which the content-declared item default cannot express.
#[derive(Clone, Debug)]
pub struct MaxStackSizeContext {
    pub item: String,
    /// The stack's custom data, empty for a stack that carries none. The query needs nothing
    /// else about the stack: the count is what is being decided, and damage does not bear on it.
    pub custom_data: super::base::NbtCompound,
}

/// A per-stack enchantability query (Minecraft: `Item.getEnchantmentValue`, NeoForge's stack-sensitive
/// `getEnchantmentValue(ItemStack)`): how readily the stack takes enchantments at an enchanting table, for
/// this stack (a tool whose materials decide its enchantability reads them from its custom data). 0 means
/// the stack cannot be enchanted at a table at all. The game asks through the item behaviour transport:
/// `Enchantments::enchantable_of` — the one lookup the table's offers, costs and enchantment selection
/// all read — consults a registered answer before the stack's `minecraft:enchantable` component and the
/// item default behind it, so a registered answer moves the offers exactly as the stack's own value does
/// (0 closes the table to the stack; a higher value widens the selection's level bonus).
#[derive(Clone, Debug)]
pub struct EnchantabilityContext {
    pub item: String,
    /// The stack's custom data, empty for a stack that carries none.
    pub custom_data: super::base::NbtCompound,
}

/// A tool-action query (NeoForge: `ItemStack.canPerformAction(ToolAction)`, the `ToolActions` names such as
/// `shears_dig`, `shears_harvest`, `shears_carve`, `shears_disarm` and `fishing_rod_cast`): whether this
/// stack may do `action`. The game asks through the item behaviour transport where it names an item
/// instead of asking the stack, at the two function-boundary shear gates: shearing a snow golem or
/// bogged (`Game::interact_shear_mob`) and shearing a saddle or equipment off a mob
/// (`Game::shear_saddle`) accept a stack that answers `shears_harvest` and run their original bodies
/// for it. The remaining vanilla name-checks are not function boundaries a head injection can
/// rewrite, so they still name the vanilla items: the sheep, mooshroom and wolf shear arms and the
/// LAN shear hook's gate are match guards inside `Game::interact_mob`, the dispenser's shears
/// behaviour is chosen by an item-id match in its dispense dispatch, a fishing cast runs a custom
/// hook entity in the original rather than the vanilla bobber a gate answer would admit, and block
/// interactions (axe stripping and kin) resolve from the game's exported behaviour tables rather
/// than a stack query. The flows the actions unlock ride the events that do exist (a shearing rides
/// [`ServerEntityEvents::SHEAR`](super::entity_events_v1::ServerEntityEvents::SHEAR) and the entity
/// interaction callback) once the game lets the stack start them.
#[derive(Clone, Debug)]
pub struct ToolActionContext {
    pub item: String,
    /// The action's name, as NeoForge's `ToolActions` name it (lower case, such as `shears_dig`).
    pub action: String,
    /// The stack's custom data, empty for a stack that carries none.
    pub custom_data: super::base::NbtCompound,
}

type Mask = Arc<dyn Fn(&EnderMaskContext) -> Option<bool> + Send + Sync>;
type MaxStackSize = Arc<dyn Fn(&MaxStackSizeContext) -> Option<u32> + Send + Sync>;
type Enchantability = Arc<dyn Fn(&EnchantabilityContext) -> Option<i32> + Send + Sync>;
type ToolAction = Arc<dyn Fn(&ToolActionContext) -> Option<bool> + Send + Sync>;
/// Item extension behavior. None from a callback preserves the native answer.
#[derive(Clone)]
pub struct ItemBehavior {
    item: Identifier,
    mask: Option<Mask>,
    max_stack_size: Option<MaxStackSize>,
    enchantability: Option<Enchantability>,
    tool_action: Option<ToolAction>,
}
impl ItemBehavior {
    pub fn new(item: Identifier) -> Self { Self { item, mask: None, max_stack_size: None, enchantability: None, tool_action: None } }
    /// Explicit false disables even a native mask; None delegates to native.
    pub fn is_ender_mask(mut self, callback: impl Fn(&EnderMaskContext) -> Option<bool> + Send + Sync + 'static) -> Self {
        self.mask = Some(Arc::new(callback)); self
    }
    /// The most items one stack may hold, for the stack in the context; None delegates to the
    /// item's content-declared maximum. An answer of 0 is treated as no answer.
    pub fn max_stack_size(mut self, callback: impl Fn(&MaxStackSizeContext) -> Option<u32> + Send + Sync + 'static) -> Self {
        self.max_stack_size = Some(Arc::new(callback)); self
    }
    /// How readily a stack of this item takes enchantments (see [`EnchantabilityContext`]); None delegates
    /// to the item's own value. A negative answer is treated as no answer.
    pub fn enchantability(mut self, callback: impl Fn(&EnchantabilityContext) -> Option<i32> + Send + Sync + 'static) -> Self {
        self.enchantability = Some(Arc::new(callback)); self
    }
    /// Whether a stack of this item may perform the tool action in the context (see
    /// [`ToolActionContext`]); None delegates to the item's own actions.
    pub fn can_perform_action(mut self, callback: impl Fn(&ToolActionContext) -> Option<bool> + Send + Sync + 'static) -> Self {
        self.tool_action = Some(Arc::new(callback)); self
    }
}
fn registry() -> &'static Mutex<HashMap<String, ItemBehavior>> {
    static ITEMS: OnceLock<Mutex<HashMap<String, ItemBehavior>>> = OnceLock::new();
    ITEMS.get_or_init(Default::default)
}
/// Register Minecraft item extension behavior without a mod-owned injection.
pub struct ItemBehaviors;
impl ItemBehaviors {
    /// Duplicate registrations fail without replacing the original behavior.
    pub fn register(behavior: ItemBehavior) -> Result<(), String> {
        let mut items = registry().lock().map_err(|_| "item behavior registry poisoned")?;
        let id = behavior.item.to_string();
        if items.contains_key(&id) { return Err(format!("item behavior already registered: {id}")); }
        items.insert(id, behavior); Ok(())
    }
}
pub(crate) fn ender_mask(context: &EnderMaskContext) -> Option<bool> {
    let callback = registry().lock().ok()?.get(&context.item)?.mask.clone()?;
    callback(context)
}

pub(crate) fn max_stack_size(context: &MaxStackSizeContext) -> Option<u32> {
    let callback = registry().lock().ok()?.get(&context.item)?.max_stack_size.clone()?;
    callback(context).filter(|limit| *limit > 0)
}

pub(crate) fn enchantability(context: &EnchantabilityContext) -> Option<i32> {
    let callback = registry().lock().ok()?.get(&context.item)?.enchantability.clone()?;
    callback(context).filter(|value| *value >= 0)
}

pub(crate) fn can_perform_action(context: &ToolActionContext) -> Option<bool> {
    let callback = registry().lock().ok()?.get(&context.item)?.tool_action.clone()?;
    callback(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mask_answers_preserve_false_delegation_duplicates_and_reentrant_registration() {
        let id = Identifier::parse("mask_test:helmet").unwrap();
        ItemBehaviors::register(ItemBehavior::new(id.clone()).is_ender_mask(|q| {
            // This must not deadlock: callbacks run after unlocking the registry.
            let _ = ItemBehaviors::register(ItemBehavior::new(Identifier::parse("mask_test:nested").unwrap()));
            match q.observer_entity { Some(1) => Some(true), Some(2) => Some(false), _ => None }
        })).unwrap();
        assert!(ItemBehaviors::register(ItemBehavior::new(id)).is_err());
        let mut context = EnderMaskContext { item: "mask_test:helmet".into(), stack: None,
            wearer_uuid: Some(7), observer_entity: Some(1) };
        assert_eq!(ender_mask(&context), Some(true));
        context.observer_entity = Some(2); assert_eq!(ender_mask(&context), Some(false));
        context.observer_entity = None; assert_eq!(ender_mask(&context), None);
        context.item = "minecraft:carved_pumpkin".into(); assert_eq!(ender_mask(&context), None);
    }

    #[test]
    fn max_stack_size_answers_by_stack_data_and_delegates_otherwise() {
        use crate::pure::base::{Nbt, NbtCompound};
        ItemBehaviors::register(ItemBehavior::new(Identifier::parse("stack_test:tank").unwrap())
            .max_stack_size(|context| context.custom_data.contains_key("tank").then_some(16))).unwrap();
        let mut filled = MaxStackSizeContext { item: "stack_test:tank".into(), custom_data: NbtCompound::new() };
        assert_eq!(max_stack_size(&filled), None, "an empty tank delegates to the item default");
        filled.custom_data.insert("tank".into(), Nbt::Compound(NbtCompound::new()));
        assert_eq!(max_stack_size(&filled), Some(16), "a filled tank caps at 16");
        filled.item = "stack_test:other".into();
        assert_eq!(max_stack_size(&filled), None, "unregistered items delegate");
        // a zero answer is no answer (a stack must hold at least one)
        ItemBehaviors::register(ItemBehavior::new(Identifier::parse("stack_test:zero").unwrap())
            .max_stack_size(|_| Some(0))).unwrap();
        assert_eq!(max_stack_size(&MaxStackSizeContext { item: "stack_test:zero".into(), custom_data: NbtCompound::new() }), None);
    }

    #[test]
    fn enchantability_answers_by_stack_data_and_delegates_otherwise() {
        use crate::pure::base::{Nbt, NbtCompound};
        ItemBehaviors::register(ItemBehavior::new(Identifier::parse("enchant_test:tool").unwrap())
            .enchantability(|context| if context.custom_data.contains_key("unenchantable") { Some(0) } else { Some(14) })).unwrap();
        let mut context = EnchantabilityContext { item: "enchant_test:tool".into(), custom_data: NbtCompound::new() };
        assert_eq!(enchantability(&context), Some(14));
        context.custom_data.insert("unenchantable".into(), Nbt::Byte(1));
        assert_eq!(enchantability(&context), Some(0), "zero is a real answer: the stack takes no enchantments");
        context.item = "enchant_test:other".into();
        assert_eq!(enchantability(&context), None, "unregistered items delegate");
        ItemBehaviors::register(ItemBehavior::new(Identifier::parse("enchant_test:negative").unwrap())
            .enchantability(|_| Some(-3))).unwrap();
        assert_eq!(enchantability(&EnchantabilityContext { item: "enchant_test:negative".into(), custom_data: NbtCompound::new() }), None, "a negative answer is no answer");
    }

    #[test]
    fn tool_action_answers_by_action_name_and_delegates_otherwise() {
        use crate::pure::base::NbtCompound;
        ItemBehaviors::register(ItemBehavior::new(Identifier::parse("action_test:kama").unwrap())
            .can_perform_action(|context| context.action.starts_with("shears_").then_some(true))).unwrap();
        let context = |action: &str| ToolActionContext { item: "action_test:kama".into(), action: action.into(), custom_data: NbtCompound::new() };
        assert_eq!(can_perform_action(&context("shears_dig")), Some(true));
        assert_eq!(can_perform_action(&context("shears_harvest")), Some(true));
        assert_eq!(can_perform_action(&context("fishing_rod_cast")), None, "an unanswered action delegates");
        let mut other = context("shears_dig");
        other.item = "action_test:other".into();
        assert_eq!(can_perform_action(&other), None, "unregistered items delegate");
    }
}
