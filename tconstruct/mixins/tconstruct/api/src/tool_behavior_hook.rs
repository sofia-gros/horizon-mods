//! The item behaviour answers of every modifiable item: whether a stack may perform a tool action
//! (ToolActionModifierHook over its modifiers — the query the game's shear gates ask) and how readily it
//! takes enchantments at a table. SlimeKnights (MIT; see LICENSE).
//!
//! The enchantability answer is the original's own, not an omission: ModifiableItem.isEnchantable and
//! ModifiableArmorItem.isEnchantable return false with no getEnchantmentValue override, and
//! ModifiableLauncherItem.getEnchantmentValue returns 0 — so every modifiable item answers 0 and the
//! table stays closed to it, exactly as in the original, where a tool's enchantments come from its
//! modifiers instead. (The tier and material values, TinkerTier.getEnchantmentValue and kin, feed the
//! material stats; no item class registers them as the item's own answer.)
use crate::{break_speed_hook::with_held_tool, tool_definitions, tool_stack};
use pluto_lib::{ItemBehavior, ItemBehaviors, StackData};

/// The helmets the mob disguise hook registers its own behaviour for: one behaviour per item, and theirs
/// carries the ender mask answer. A helmet's table answer is the same 0 by default — no content declares
/// an enchantable component for any Tinkers item — and no shipped shears or fishing modifier targets a
/// helmet, so nothing is lost by leaving those three to the disguise hook.
const DISGUISE_HELMETS: [&str; 3] = ["tconstruct:travelers_helmet", "tconstruct:plate_helmet", "tconstruct:slime_helmet"];

/// Registers the behaviours. Called once from the product's hooks.
pub(crate) fn register() {
    for item in tool_definitions::registered() {
        if DISGUISE_HELMETS.contains(&item.to_string().as_str()) { continue; }
        let behavior = ItemBehavior::new(item.clone())
            .enchantability(|_| Some(0))
            .can_perform_action(|query| {
                let stack = StackData {
                    item: query.item.clone(), count: 1, damage: 0, max_damage: None,
                    custom_data: query.custom_data.clone(), custom_model_data: None, enchantments: Default::default(),
                };
                // A stack that is not a built, unbroken Tinkers tool delegates: the item has no actions of
                // its own for the game to find, which is the same answer the original gives for it.
                match with_held_tool(&stack, |env, tool| tool_stack::can_perform_action(&tool.custom_data, env, &query.action)) {
                    Ok(answer) => Some(answer),
                    Err(_) => None,
                }
            });
        if let Err(error) = ItemBehaviors::register(behavior) {
            eprintln!("Tinkers: tool behaviour for {item} not registered: {error}");
        }
    }
}
