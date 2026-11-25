//! Connects the damage-reducing modifiers to PlutoLib's item-wear event: a Tinkers tool a player wears down loses less
//! (or none) when its modifiers cancel points, as ToolDamageUtil.damage's modifier loop does in the original.
//! The tool's wear runs through ToolDamageUtil.damage over the stack (overslime, reductions, the tool's own damage and broken
//! state, the published durability, mining and attribute components) and the finished stack replaces the worn one; a piece the
//! event cannot rewrite (worn armor) keeps the host's own wear, less what the modifiers cancelled.
//!
//! ShareDurabilityModule runs in the same pass (its priority sits above the shield modules in the shipped
//! data, so it reads the amount before the soak): it damages its own tool directly and charges the shield
//! bar of a random other carried tool, which lands as a carried edit beside the wear.
use crate::{break_speed_hook::with_held_tool, common, modifier_entries::List, modifier_modules::{leveling_compute, Module}, tool_slot, tool_stack};
use pluto_lib::{CarriedEdit, CarriedSlot, CarriedStack, Identifier, ItemSlot, ServerLivingEntityEvents, ServerPlayer, StackSlot};
use std::sync::atomic::{AtomicU64, Ordering};

static SEED: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

/// A uniform value in [0, 1) from a xorshift generator; modifier chances are drawn one point at a time.
pub(crate) fn random() -> f32 {
    let mut state = SEED.load(Ordering::Relaxed);
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    SEED.store(state, Ordering::Relaxed);
    (state >> 40) as f32 / (1u64 << 24) as f32
}

/// The stacks ShareDurabilityModule may pick from, in the original's order: for a worn armor piece, the
/// other armor slots' stacks in the durability tag whose item differs from the damaged piece's; otherwise
/// the hotbar and the off hand (a mob holder: its main hand and off hand), skipping the damaged stack's own
/// place. Only stacks in `tconstruct:modifiable/durability` count (that tag names only Tinkers tools).
fn share_options(tool: &ItemSlot, env: &tool_stack::Environment, is_player: bool, worn: Option<&CarriedSlot>, carried: &[CarriedStack], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Vec<CarriedStack> {
    let durability = Identifier::parse("tconstruct:modifiable/durability").expect("valid tag");
    let target = |carried: &CarriedStack| in_tag(&carried.stack.item, &durability);
    let worn_armor = env.item.is_some_and(|item| in_tag(item, &Identifier::parse("tconstruct:modifiable/armor/worn").expect("valid tag")));
    if worn_armor {
        return carried.iter().filter(|carried| {
            matches!(carried.slot, CarriedSlot::Equipment(StackSlot::Feet | StackSlot::Legs | StackSlot::Chest | StackSlot::Head))
                && target(carried) && carried.stack.item != tool.item
        }).cloned().collect();
    }
    carried.iter().filter(|carried| {
        if Some(&carried.slot) == worn { return false; }
        let place = match carried.slot {
            CarriedSlot::Inventory(slot) => slot <= 8,
            CarriedSlot::Equipment(StackSlot::OffHand) => true,
            CarriedSlot::Equipment(StackSlot::MainHand) => !is_player,
            _ => false,
        };
        place && target(carried)
    }).cloned().collect()
}

/// Charges the shield bar of one carried stack (CAPACITY_BAR.addAmount): the stack gains the shield
/// modifier at level 1 when it lacks it, as choice.addModifier does, then its bar rises by `amount`,
/// capped at the bar's capacity. The shield is a capacity-bar modifier (barkskin in the shipped data);
/// a shield without a bar takes nothing. Returns the stack to write back.
fn charge_shield(choice: &ItemSlot, shield: &Identifier, amount: i32) -> Option<ItemSlot> {
    let registry = common::server_modifiers()?.read().ok()?;
    let mut data = choice.custom_data.clone();
    let mut saved = tool_stack::saved_modifiers(&data, &registry);
    if !saved.iter().any(|(entry, _)| entry.id == *shield) {
        let list = List::from_nbt(data.get(tool_stack::TAG_MODIFIERS));
        let list = list.with_modifier(shield.clone(), 1).ok()?;
        data.insert(tool_stack::TAG_MODIFIERS.into(), list.to_nbt());
        saved = tool_stack::saved_modifiers(&data, &registry);
    }
    let (entry, modifier) = saved.iter().find(|(entry, _)| entry.id == *shield)?;
    let bar = modifier.capacity_bar()?;
    let capacity = bar.capacity(&data, entry);
    let now = crate::capacity::amount(&data, shield);
    crate::capacity::set_amount(&mut data, shield, capacity, now.wrapping_add(amount));
    let mut slot = choice.clone();
    slot.custom_data = data;
    Some(slot)
}

/// ShareDurabilityModule over one wear of the tool: the extra direct damage for the tool itself and the
/// carried edit charging the picked stack's shield (None when no module shares or no option exists).
fn share_durability(tool: &ItemSlot, env: &tool_stack::Environment, amount: i32, is_player: bool, worn: Option<&CarriedSlot>, carried: &[CarriedStack], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> (i32, Option<CarriedEdit>) {
    let saved = tool_stack::saved_modifiers(&tool.custom_data, env.modifiers);
    let mut extra = 0;
    let mut edit: Option<CarriedEdit> = None;
    for (entry, modifier) in &saved {
        for module in modifier.modules() {
            let Module::ShareDurability { shield, grant, consume } = module else { continue; };
            let options = share_options(tool, env, is_player, worn, carried, in_tag);
            if options.is_empty() { continue; }
            // The direct damage already dealt by an earlier share module counts against the durability left.
            let current = tool_stack::current_durability(&tool.custom_data) - extra;
            let consumed = (leveling_compute(*consume, entry.effective_level()) * amount).min(current);
            let granted = leveling_compute(*grant, entry.effective_level());
            if consumed <= 0 || granted <= 0 { continue; }
            let pick = &options[(random() * options.len() as f32) as usize % options.len()];
            // A second share onto the same place builds on the first one's edit.
            let base = edit.as_ref().filter(|edit| edit.slot == pick.slot).map(|edit| &edit.stack).unwrap_or(&pick.stack);
            if let Some(charged) = charge_shield(base, shield, granted * amount) {
                extra += consumed;
                edit = Some(CarriedEdit { slot: pick.slot, stack: charged });
            }
        }
    }
    (extra, edit)
}

/// The wear after the tool's modifiers (the overslime shield soaks first, then each modifier cancels part of what is left, as
/// ToolDamageUtil.damage does) and the worn stack with that wear applied; the amount unchanged and no stack when the stack is not
/// a loaded Tinkers tool. ShareDurabilityModule's side effects ride along: its direct damage is applied to
/// the replacement (or added to the amount a host-worn piece keeps, which has no replacement to carry it),
/// and the charged carried stack comes back as the third value. The fourth value is whether the wear broke
/// the tool — ToolDamageUtil.damage's own answer, the share's side effect included, as damageAnimated reads it.
pub(crate) fn worn(stack: &pluto_lib::StackData, slot: Option<&pluto_lib::ItemSlot>, amount: i32, is_player: bool, worn_place: Option<&CarriedSlot>, carried: &[CarriedStack], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> (i32, Option<pluto_lib::ItemSlot>, Option<CarriedEdit>, bool) {
    let Ok(result) = with_held_tool(stack, |env, tool| {
        let (extra, edit) = share_durability(tool, env, amount, is_player, worn_place, carried, in_tag);
        let mut data = tool.custom_data.clone();
        let soaked = crate::capacity::absorb(&mut data, env, None, amount, &mut random);
        let left = tool_stack::reduce_damage(&data, env, None, soaked, &mut random);
        let mut broke = false;
        let replacement = slot.and_then(|slot| {
            let mut next = slot.clone();
            // The same modifier loop and the tool's own damage and broken state, then the visible stack follows.
            broke = tool_slot::damage_slot(&mut next, env, amount, true, true, None, &mut random).ok()?.broke;
            // ToolDamageUtil.directDamage: the share's own damage, unmodified, on top of the wear. In the
            // original the share runs inside damage's hook loop, before the amount lands, and damage's
            // answer is the final state — a share that finishes the tool off breaks it for the broadcast
            // too, though the share's own directDamage call never broadcasts by itself.
            if extra > 0 { broke = tool_slot::direct_damage_slot(&mut next, env, extra, true).ok()?.broke || broke; }
            Some(next)
        });
        // A piece the event cannot rewrite keeps the host's wear: the direct damage joins the amount there.
        let amount = if replacement.is_none() { left + extra } else { left };
        (amount, replacement, edit, broke)
    }) else { return (amount, None, None, false); };
    result
}

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::MODIFY_ITEM_DAMAGE.register(|wear| {
        static LOGGED: AtomicU64 = AtomicU64::new(0);
        let log = LOGGED.fetch_add(1, Ordering::Relaxed) < 6;
        let Some(stack) = wear.stack.as_ref() else {
            if log { eprintln!("Tinkers: item wear of {} by {}: the player's stack was not found", wear.item, wear.amount.get()); }
            return;
        };
        let amount = wear.amount.get();
        if amount <= 0 { return; }
        let Some(items) = common::item_snapshot() else { return; };
        let in_tag = |item: &Identifier, tag: &Identifier| items.get_tag(tag).is_some_and(|members| members.contains(item));
        // The formulas of tool damage modules read the player (light, temperature) as the tool wore.
        let (left, replacement, edit, broke) = crate::entity_facts::with_holder(wear.holder.as_ref(), || {
            worn(stack, wear.slot.as_ref(), amount, true, wear.worn.as_ref(), &wear.carried, &in_tag)
        });
        if left != amount { wear.amount.set(left); }
        if replacement.is_some() { *wear.replacement.borrow_mut() = replacement.clone(); }
        if let Some(edit) = edit { wear.edits.borrow_mut().push(edit); }
        broadcast_break(wear.player, &wear.name, wear.worn.as_ref(), replacement.as_ref(), broke);
    });
}

/// ToolDamageUtil.damageAnimated's broadcast, for the wear above: a tool this wear broke is
/// announced exactly once, here where the replacement the game keeps is set. The game never breaks
/// a Tinkers tool itself — its mirror stack stops one short of the game's break point — so the
/// break is this port's to announce. A wear with no replacement keeps the host's own wear, whose
/// break handling stays the host's; a wear that leaves the tool whole announces nothing. False
/// when nothing was broadcast.
fn broadcast_break(player: u128, name: &str, worn: Option<&CarriedSlot>, replacement: Option<&ItemSlot>, broke: bool) -> bool {
    if !broke { return false; }
    let (Some(replacement), Some(CarriedSlot::Equipment(slot))) = (replacement, worn) else { return false; };
    ServerPlayer { uuid: player, name: name.to_owned() }.broadcast_item_break(*slot, &crate::brush_hook::stack_of(replacement))
}

#[cfg(test)]
mod tests {
    use super::{broadcast_break, random};
    use pluto_lib::{CarriedSlot, Identifier, ItemSlot, MemoryPlayerOps, StackSlot, with_ops};

    #[test]
    fn the_generator_stays_inside_zero_to_one() {
        for _ in 0..1000 { let value = random(); assert!((0.0..1.0).contains(&value), "{value}"); }
    }

    #[test]
    fn a_wear_that_breaks_the_tool_broadcasts_exactly_once() {
        let mut game = MemoryPlayerOps::default();
        let replacement = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        let hand = CarriedSlot::Equipment(StackSlot::MainHand);
        with_ops(&mut game, || {
            assert!(broadcast_break(7, "test", Some(&hand), Some(&replacement), true), "the breaking wear broadcasts");
            assert!(!broadcast_break(7, "test", Some(&hand), Some(&replacement), false), "a wear that leaves the tool whole broadcasts nothing");
            assert!(!broadcast_break(7, "test", Some(&hand), None, true), "a wear the host keeps (no replacement) is the host's own break to announce");
            assert!(!broadcast_break(7, "test", None, Some(&replacement), true), "a wear at no equipment place names no slot to announce");
        });
        assert_eq!(game.breaks, [(7, StackSlot::MainHand, "tconstruct:pickaxe".to_string())], "exactly one broadcast, of the stack as it broke");
    }
}
