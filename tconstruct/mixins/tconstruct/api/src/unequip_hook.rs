//! Armor taken off, from PlutoLib's player tick: the pieces a player wore at their last tick are compared with the ones they wear now, and a
//! piece that is gone or has become another tool (a different item or modifier list, EquipmentChangeModifierHook.didUnequip) was unequipped.
//! Its clear-effect-on-unequip modules take their effect off the wearer. SlimeKnights (MIT; see LICENSE). The tick also comes once after the
//! last piece is taken off, so removing the final piece is seen too. Held tools are not reported by the tick; the effects on unequip that
//! shorten an effect or hurt the wearer need an effect-duration or hurt action the tick does not offer, and stay unbound.
use crate::{break_speed_hook::with_held_tool, tool_stack};
use pluto_lib::{Identifier, ServerLivingEntityEvents, StackData};
use std::{collections::BTreeMap, sync::Mutex};

/// The armor each player (by entity id) wore at their last tick.
static WORN: Mutex<BTreeMap<i32, Vec<(String, StackData)>>> = Mutex::new(BTreeMap::new());

/// EquipmentChangeModifierHook.didChange over the stacks as the tick reports them: the item or the modifier list differs.
fn changed(before: &StackData, after: &StackData) -> bool {
    before.item != after.item || before.custom_data.get(tool_stack::TAG_MODIFIERS) != after.custom_data.get(tool_stack::TAG_MODIFIERS)
}

/// The pieces of `before` that `after` no longer holds in the same slot.
pub(crate) fn departed<'a>(before: &'a [(String, StackData)], after: &[(String, StackData)]) -> Vec<&'a StackData> {
    before.iter().filter(|(slot, piece)| after.iter().find(|(other, _)| other == slot).is_none_or(|(_, now)| changed(piece, now))).map(|(_, piece)| piece).collect()
}

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
        let Ok(mut worn) = WORN.lock() else { return; };
        let before = if tick.armor.is_empty() { worn.remove(&tick.entity) } else { worn.insert(tick.entity, tick.armor.clone()) };
        let Some(before) = before else { return; };
        let mut cleared: Vec<Identifier> = Vec::new();
        for piece in departed(&before, &tick.armor) {
            if let Ok(effects) = with_held_tool(piece, |env, stack| tool_stack::effects_cleared_on_unequip(&stack.custom_data, env)) { cleared.extend(effects); }
        }
        for effect in cleared {
            let id = effect.to_string();
            if tick.effects.iter().any(|(active, _, _)| *active == id) { tick.remove_effects.borrow_mut().push(id); }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{Nbt, NbtCompound};

    fn piece(item: &str, modifiers: i32) -> StackData {
        let mut custom_data = NbtCompound::new();
        custom_data.insert(tool_stack::TAG_MODIFIERS.into(), Nbt::Int(modifiers));
        StackData { item: item.into(), count: 1, damage: 0, max_damage: None, custom_data }
    }

    #[test]
    fn a_piece_is_unequipped_when_its_slot_empties_or_holds_another_tool() {
        let before = vec![("head".to_owned(), piece("tconstruct:travelers_helmet", 1)), ("chest".to_owned(), piece("tconstruct:travelers_chestplate", 1))];
        assert!(departed(&before, &before).is_empty(), "nothing changed");
        assert_eq!(departed(&before, &before[..1]).len(), 1, "the chest piece is gone");
        assert_eq!(departed(&before, &[]).len(), 2, "the last pieces coming off count");
        let other_item = vec![before[0].clone(), ("chest".to_owned(), piece("tconstruct:plate_chestplate", 1))];
        assert_eq!(departed(&before, &other_item).len(), 1, "another item in the slot");
        let reforged = vec![before[0].clone(), ("chest".to_owned(), piece("tconstruct:travelers_chestplate", 2))];
        assert_eq!(departed(&before, &reforged).len(), 1, "a different modifier list is another tool");
        let mut worn = before[1].1.clone();
        worn.damage = 9;
        assert!(departed(&before[1..], &[("chest".to_owned(), worn)]).is_empty(), "wear alone is no change");
    }
}
