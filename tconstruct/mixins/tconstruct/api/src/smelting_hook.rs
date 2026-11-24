//! Connects the smelting modules' cooking to the hits and shots that power it: a melee hit cooks
//! by the damage it deals (SmeltingModule.afterMeleeHit, read off the damage event's dealt
//! estimate as lifesteal reads it), a launcher's hit cooks the launcher by the damage its arrow
//! or trident deals (onLauncherHitEntity), damage taken while wearing the tool as armor cooks the
//! armor by the amount that arrives (onAttacked, at the hurt stage, the original's LivingHurtEvent,
//! before armor and enchantment protection take their share), shearing cooks the shears by one
//! (afterShearEntity), and a sling's launch cooks by the force doubled over its multiplier
//! (afterSlingLaunch, through the release's carried edits in `cook_release`). A shot cooks by the
//! launcher's projectile damage stat (onProjectileLaunch, whose arrow power is that stat for a
//! tool's own arrows). The block harvest cooking rides block_break_hook (finishHarvest) and the
//! plant harvest cooking harvest_hook (afterHarvest). The cooking engine is tool_stack::cook_smelting.
//!
//! Connected through the mob-stack-write surface: a monster's held tool (MonsterMeleeHit RedirectAfter) cooks by the damage it deals,
//! in monster_melee's damage dispatch, through `EntityAction::EditStackData` on the attacker (MainHand for a held weapon, Chest for
//! the unarmed chest piece). PlutoLib applies the edit on the mob's own tick, reading the slot as it is when the action lands and
//! writing it back through the living handle's equipment write (API 382, the `/item replace entity` path) — no new injection was
//! needed, the reach the surface stands on already existed. The computation is tool_stack::smelting_edits on the monster's held stack.
//!
//! The shear cooking is heard before the shears act, so a shearing a later callback cancels still
//! leaves the cook — the same deviation the other shear consumers accept. A player's own shearing
//! is never heard on a LAN guest (the host reports only the owner's shearing there).
use crate::{break_speed_hook::with_held_tool, tool_stack, tool_stats::Stat};
use pluto_lib::{ActionTarget, CarriedEdit, CarriedSlot, DamageModification, EntityAction, Identifier, ItemSlot, ProjectileEvents, ReleaseUsing, ServerEntityEvents, ServerLivingEntityEvents, StackSlot};

const PLAYER_ATTACK: &str = "minecraft:player_attack";
const ARROW: &str = "minecraft:arrow";
const TRIDENT: &str = "minecraft:trident";
/// The tag a worn tool must carry for its cooking to answer damage taken (SmeltingModule.onAttacked
/// only cooks tools with the armor tag).
const ARMOR_TAG: &str = "modifiable/armor";

/// Registers the callbacks. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
        // A melee hit cooks the attacker's held tool by the damage it dealt.
        if hit.damage.damage_type == PLAYER_ATTACK {
            if let Some(held) = hit.attacker_stack.as_ref() {
                cook_held(hit, StackSlot::MainHand, held, dealt(hit));
            }
        }
        // A launcher's hit cooks the launcher by the damage its arrow or trident dealt: the hand
        // the shot telling names (the telling reads the weapon the ammunition event named and
        // matches it against the hands the hit's opening reported). Without a telling — a
        // trident, whose shot names no weapon, or a weapon swapped away after the shot — the
        // main hand stands in, as before. Fireballs and other projectiles carry other damage
        // types, so their launchers are not cooked here.
        if matches!(hit.damage.damage_type.as_str(), ARROW | TRIDENT) && hit.attacker_is_player == Some(true) {
            if let Some((slot, held)) = launcher_of(hit) {
                cook_held(hit, slot, held, dealt(hit));
            }
        }
    });
    ServerLivingEntityEvents::MODIFY_HURT.register(|hit| {
        // Damage taken while wearing the tool as armor cooks each worn armor piece by the amount
        // that arrives, before armor and enchantment protection take their share.
        let amount = hit.amount.get();
        if amount <= 0.0 { return; }
        for (name, piece) in &hit.armor {
            let Some(slot) = pluto_lib::armor_stack_slot(name) else { continue; };
            let Ok(edits) = with_held_tool(piece, |env, stack| {
                if !is_armor(env) { return Vec::new(); }
                tool_stack::smelting_edits(&stack.custom_data, env, amount)
            }) else { continue; };
            if edits.is_empty() { continue; }
            hit.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Victim, slot, item: piece.item.clone(), edits });
        }
    });
    ServerEntityEvents::SHEAR.register(|attempt| {
        // Shearing a mob with a tool cooks it by one (afterShearEntity).
        let Some(tool) = attempt.tool.as_ref() else { return; };
        let Ok(edits) = with_held_tool(tool, |env, stack| tool_stack::smelting_edits(&stack.custom_data, env, 1.0)) else { return; };
        if edits.is_empty() { return; }
        let slot = if attempt.off_hand { StackSlot::OffHand } else { StackSlot::MainHand };
        attempt.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot, item: tool.item.clone(), edits });
    });
    ProjectileEvents::SHOOT_BOW.register(|shot| {
        for (hand, slot) in [(&shot.main_hand, StackSlot::MainHand), (&shot.off_hand, StackSlot::OffHand)] {
            let Some(stack) = hand else { continue };
            let Ok(edits) = with_held_tool(stack, |env, tool| tool_stack::smelting_edits(&tool.custom_data, env, tool_stack::stat(&tool.custom_data, Stat::ProjectileDamage))) else { continue };
            if edits.is_empty() { continue; }
            shot.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot, item: stack.item.clone(), edits });
            return;
        }
    });
}

/// The damage the hit dealt to the target, for the cooking that measures by it: the amount, capped
/// by the target's health when the hit reported it (a killing blow cooks by the health it had, not
/// the overkill).
fn dealt(hit: &DamageModification) -> f32 {
    let amount = hit.amount.get();
    hit.target_state.as_ref().and_then(|state| state.health).map_or(amount, |health| amount.min(health))
}

/// Whether the held stack's item is in the armor tag, for the damage-taken cooking.
fn is_armor(env: &tool_stack::Environment) -> bool {
    let (Some(item), Some(tags)) = (env.item, env.item_tags) else { return false; };
    Identifier::new("tconstruct", ARMOR_TAG).ok().and_then(|tag| tags.get_tag(&tag)).is_some_and(|members| members.contains(item))
}

/// The launcher of a hit and the hand that fired it: the telling's hand and stack when the hit
/// names them, else the main hand's stack when the hit reports one (a trident's shot names no
/// weapon, and a weapon swapped away after the shot names no hand). The telling's stack is the
/// launcher as the hit landed; the fallback's is the main hand as the hit landed.
fn launcher_of(hit: &DamageModification) -> Option<(StackSlot, &pluto_lib::StackData)> {
    if let (Some(hand), Some(stack)) = (hit.attacker_launcher_hand, hit.attacker_launcher_stack.as_ref()) {
        return Some((hand, stack));
    }
    hit.attacker_main_hand.as_ref().map(|stack| (StackSlot::MainHand, stack))
}

/// Cook the attacker's stack `held` in `slot` by `amount`, as an edit of that slot.
fn cook_held(hit: &DamageModification, slot: StackSlot, held: &pluto_lib::StackData, amount: f32) {
    if amount <= 0.0 { return; }
    let Ok(edits) = with_held_tool(held, |env, stack| tool_stack::smelting_edits(&stack.custom_data, env, amount)) else { return; };
    if edits.is_empty() { return; }
    hit.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot, item: held.item.clone(), edits });
}

/// SmeltingModule.afterSlingLaunch: cook the released tool by `amount` (the sling's force doubled
/// over its multiplier), writing the cooked inventory through the release's carried edits. The cook
/// chains off the latest edit already queued for the hand — the wear `spend` queued — so the wear
/// survives; without one the release's own stack stands in. Several modules sharing one release
/// cook the same queued edit in turn, so each sees the last one's cooking.
pub(crate) fn cook_release(release: &ReleaseUsing, amount: f32) {
    if !amount.is_finite() || amount <= 0.0 { return; }
    let same = |slot: &ItemSlot| slot.item.to_string() == release.stack.item && slot.custom_data == release.stack.custom_data;
    let hand = release.main_hand.as_ref().filter(|slot| same(slot)).map(|slot| (CarriedSlot::Equipment(StackSlot::MainHand), slot.clone()))
        .or_else(|| release.off_hand.as_ref().filter(|slot| same(slot)).map(|slot| (CarriedSlot::Equipment(StackSlot::OffHand), slot.clone())));
    let Some((slot, held)) = hand else { return; };
    let _ = with_held_tool(&release.stack, |env, _tool| {
        let mut edits = release.edits.borrow_mut();
        match edits.iter_mut().rev().find(|edit| edit.slot == slot) {
            Some(edit) => {
                let mut data = edit.stack.custom_data.clone();
                tool_stack::cook_smelting(&mut data, env, amount);
                edit.stack.custom_data = data;
            }
            None => {
                let mut stack = held;
                tool_stack::cook_smelting(&mut stack.custom_data, env, amount);
                edits.push(CarriedEdit { slot, stack });
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::LivingDamage;
    use std::cell::Cell;

    fn hit(amount: f32, health: Option<f32>) -> DamageModification {
        let damage = LivingDamage { entity: 4, kind: "minecraft:zombie".into(), damage_type: "minecraft:player_attack".into(), attacker: 1, direct: 1, amount };
        DamageModification {
            damage, amount: Cell::new(amount), attacker_stack: None, armor: Vec::new(),
            attacker_state: None, target_state: health.map(|health| pluto_lib::EntityState { health: Some(health), ..Default::default() }),
            attacker_kind: String::new(), attacker_is_player: None, attacker_main_hand: None, attacker_launcher_hand: None,
            attacker_launcher_stack: None, attacker_chest: None,
            attacker_attack_damage: None, target_health: None, target_attack_damage: None,
            attacker_position: None, target_position: None, actions: Default::default(),
        }
    }

    #[test]
    fn dealt_is_the_amount_capped_by_the_targets_health() {
        assert_eq!(dealt(&hit(5.0, Some(3.0))), 3.0);
        assert_eq!(dealt(&hit(5.0, Some(8.0))), 5.0);
        assert_eq!(dealt(&hit(5.0, None)), 5.0);
    }

    #[test]
    fn cook_release_ignores_a_non_positive_or_unbounded_amount() {
        // No hand matches the release's stack, so nothing is queued whatever the amount; the
        // guard is exercised by the amounts the sling hook never passes (a zero multiplier).
        let release = ReleaseUsing {
            stack: pluto_lib::StackData { item: "tconstruct:slime_sling".into(), count: 1, ..Default::default() },
            player: pluto_lib::ServerPlayer { uuid: 1, name: "test".into() },
            dimension: "minecraft:overworld".into(), position: [0.0, 64.0, 0.0], eye: [0.0, 65.0, 0.0],
            look: [0.0, 0.0, 1.0], creative: false, ticks_used: 20,
            main_hand: None, off_hand: None, used: Cell::new(false), edits: Default::default(),
        };
        cook_release(&release, 0.0);
        cook_release(&release, f32::INFINITY);
        assert!(release.edits.borrow().is_empty());
    }

    #[test]
    fn launcher_of_names_the_tellings_hand_and_falls_back_to_the_main_hand() {
        let bow = pluto_lib::StackData { item: "tconstruct:longbow".into(), count: 1, ..Default::default() };
        let sword = pluto_lib::StackData { item: "tconstruct:sword".into(), count: 1, ..Default::default() };
        // The telling names the off hand: the off-hand bow is the launcher, not the main hand's sword.
        let mut telling = hit(4.0, None);
        telling.attacker_launcher_hand = Some(StackSlot::OffHand);
        telling.attacker_launcher_stack = Some(bow.clone());
        telling.attacker_main_hand = Some(sword.clone());
        assert_eq!(launcher_of(&telling), Some((StackSlot::OffHand, &bow)));
        // No telling, a main hand reported: the main hand stands in (a trident's hit).
        let mut fallback = hit(4.0, None);
        fallback.attacker_main_hand = Some(sword.clone());
        assert_eq!(launcher_of(&fallback), Some((StackSlot::MainHand, &sword)));
        // Neither: no launcher is named.
        assert_eq!(launcher_of(&hit(4.0, None)), None);
    }

    #[test]
    fn armor_stack_slot_names_the_four_pieces() {
        assert_eq!(pluto_lib::armor_stack_slot("feet"), Some(StackSlot::Feet));
        assert_eq!(pluto_lib::armor_stack_slot("legs"), Some(StackSlot::Legs));
        assert_eq!(pluto_lib::armor_stack_slot("chest"), Some(StackSlot::Chest));
        assert_eq!(pluto_lib::armor_stack_slot("head"), Some(StackSlot::Head));
        assert_eq!(pluto_lib::armor_stack_slot("mainhand"), None);
    }
}
