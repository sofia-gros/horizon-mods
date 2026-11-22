//! UpdateHealthModule, from PlutoLib's player tick: a worn piece that arrives in a slot (equipped, or swapped
//! for another tool) refreshes its wearer's health by its bonus, and one that leaves takes the bonus back —
//! healing for a positive change (the game cuts a heal to the maximum, as the original caps the raised health)
//! and damage of the `tconstruct:update_health` type for a negative one. SlimeKnights (MIT; see LICENSE).
//! The tick reports worn armor only, so modules naming a hand slot stay unbound at decode.
use crate::{break_speed_hook::with_held_tool, tool_stack};
use pluto_lib::{ActionTarget, EntityAction, ServerLivingEntityEvents, StackData};
use std::{collections::BTreeMap, sync::Mutex};

/// The armor each player (by entity id) wore at their last tick.
static WORN: Mutex<BTreeMap<i32, Vec<(String, StackData)>>> = Mutex::new(BTreeMap::new());

/// EquipmentChangeModifierHook.didChange over the stacks as the tick reports them: the item or the modifier list differs.
fn changed(before: &StackData, after: &StackData) -> bool {
    before.item != after.item || before.custom_data.get(tool_stack::TAG_MODIFIERS) != after.custom_data.get(tool_stack::TAG_MODIFIERS)
}

/// The pieces of `after` whose slot `before` did not hold, or held as another tool: they were equipped.
pub(crate) fn arrived<'a>(before: &[(String, StackData)], after: &'a [(String, StackData)]) -> Vec<(&'a str, &'a StackData)> {
    after.iter().filter(|(slot, piece)| before.iter().find(|(other, _)| other == slot).is_none_or(|(_, was)| changed(was, piece))).map(|(slot, piece)| (slot.as_str(), piece)).collect()
}

/// The pieces of `before` whose slot `after` no longer holds, or holds as another tool: they were unequipped.
pub(crate) fn departed<'a>(before: &'a [(String, StackData)], after: &[(String, StackData)]) -> Vec<(&'a str, &'a StackData)> {
    before.iter().filter(|(slot, piece)| after.iter().find(|(other, _)| other == slot).is_none_or(|(_, now)| changed(piece, now))).map(|(slot, piece)| (slot.as_str(), piece)).collect()
}

/// The action for one piece's health delta: a heal for a gain, update-health damage for a loss, none for zero.
fn action(delta: f32) -> Option<EntityAction> {
    if delta > 0.0 { Some(EntityAction::Heal { target: ActionTarget::Attacker, amount: delta }) }
    else if delta < 0.0 { Some(EntityAction::Hurt { target: ActionTarget::Attacker, amount: -delta, damage_type: "tconstruct:update_health".to_owned() }) }
    else { None }
}

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
        let Ok(mut worn) = WORN.lock() else { return; };
        let before = if tick.armor.is_empty() { worn.remove(&tick.entity) } else { worn.insert(tick.entity, tick.armor.clone()) };
        let Some(before) = before else { return; };
        let mut actions = Vec::new();
        for (slot, piece) in departed(&before, &tick.armor) {
            if let Ok(delta) = with_held_tool(piece, |env, stack| tool_stack::update_health_delta(&stack.custom_data, env, slot, false)) {
                actions.extend(action(delta));
            }
        }
        for (slot, piece) in arrived(&before, &tick.armor) {
            if let Ok(delta) = with_held_tool(piece, |env, stack| tool_stack::update_health_delta(&stack.custom_data, env, slot, true)) {
                actions.extend(action(delta));
            }
        }
        tick.actions.borrow_mut().extend(actions);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{Nbt, NbtCompound};

    fn piece(item: &str, modifiers: i32) -> StackData {
        let mut custom_data = NbtCompound::new();
        custom_data.insert(tool_stack::TAG_MODIFIERS.into(), Nbt::Int(modifiers));
        StackData { item: item.into(), count: 1, damage: 0, max_damage: None, custom_data, custom_model_data: None, enchantments: Default::default() }
    }

    #[test]
    fn a_change_is_seen_per_slot_as_an_arrival_and_a_departure() {
        let before = vec![("head".to_owned(), piece("tconstruct:travelers_helmet", 1)), ("chest".to_owned(), piece("tconstruct:travelers_chestplate", 1))];
        assert!(arrived(&before, &before).is_empty() && departed(&before, &before).is_empty(), "nothing changed");
        assert_eq!(departed(&before, &before[..1]).iter().map(|(slot, _)| *slot).collect::<Vec<_>>(), ["chest"], "the chest piece left");
        assert_eq!(arrived(&[], &before).len(), 2, "both pieces arrived");
        let swapped = vec![before[0].clone(), ("chest".to_owned(), piece("tconstruct:plate_chestplate", 1))];
        assert_eq!((arrived(&before, &swapped).len(), departed(&before, &swapped).len()), (1, 1), "a swap is one of each");
        let mut worn = before[1].1.clone();
        worn.damage = 9;
        assert!(arrived(&before, &[before[0].clone(), ("chest".to_owned(), worn)]).is_empty(), "wear alone is no change");
    }

    #[test]
    fn a_gain_heals_and_a_loss_hurts_as_update_health() {
        assert_eq!(action(4.0), Some(EntityAction::Heal { target: ActionTarget::Attacker, amount: 4.0 }));
        assert_eq!(action(-4.0), Some(EntityAction::Hurt { target: ActionTarget::Attacker, amount: 4.0, damage_type: "tconstruct:update_health".to_owned() }));
        assert_eq!(action(0.0), None);
    }
}
