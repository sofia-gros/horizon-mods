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

type Mask = Arc<dyn Fn(&EnderMaskContext) -> Option<bool> + Send + Sync>;
/// Item extension behavior. None from a callback preserves the native answer.
#[derive(Clone)]
pub struct ItemBehavior {
    item: Identifier,
    mask: Option<Mask>,
}
impl ItemBehavior {
    pub fn new(item: Identifier) -> Self { Self { item, mask: None } }
    /// Explicit false disables even a native mask; None delegates to native.
    pub fn is_ender_mask(mut self, callback: impl Fn(&EnderMaskContext) -> Option<bool> + Send + Sync + 'static) -> Self {
        self.mask = Some(Arc::new(callback)); self
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
}
