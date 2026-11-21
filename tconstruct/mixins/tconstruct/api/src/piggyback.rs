//! The piggybackpack and the carry effect (SlimeKnights, MIT; see LICENSE), from
//! `gadgets/item/PiggyBackPackItem.java`, `gadgets/capability/PiggybackCapability.java` and
//! `gadgets/capability/PiggybackHandler.java`: right-click a mob while wearing (or holding)
//! the pack and it rides you; each carried entity slows you by 5% through the `tconstruct:carry`
//! effect, at most three at a time.
//!
//! What ships here: the item (declared on the `minecraft:paper` template, `stacksTo(16)`, in
//! the general creative tab after the frames and stands as in TinkerGadgets.addTabItems), the
//! `carry` mob effect declaration (`MobEffectCategory.NEUTRAL`, colour `0xffffff` — the
//! `TinkerEffect(category, show)` default), its casting recipe (already shipped:
//! `data/tconstruct/recipes/fluids/gadgets/piggy_backpack.json`, a saddle cast in 1000 mB of
//! sky slime, `FluidValues.SLIMEBALL * 4`), the empty blacklist tag (the original's
//! `EntityTypeTagProvider` creates `tconstruct:piggybackpack_blacklist` with no entries), and
//! the complete decision logic of the Java classes as the pure functions below, unit-tested
//! against the Java control flow.
//!
//! What does not ship: the live wiring, walled with evidence. Java's behaviour runs in
//! `interactLivingEntity` and in the worn stack's `inventoryTick`, and moves entities with
//! `Entity.startRiding`/`stopRiding`. The host has the seams — the `interact_entity` hook
//! (claimable, `InteractEntityEvent { entity, kind, stack }`), `interact_entity_v2` (with the
//! hand), and the `inventory_tick` hook (`InventoryTickEvent { slot, selected }`) — but this
//! PlutoLib base dispatches none of them from `hooks.rs`, and no PlutoLib surface can make one
//! entity ride another (the entity reach reads and hurts; it has no passenger operations).
//! Wiring those two host hooks into PlutoLib events and adding a ride operation to the entity
//! reach is foundation work (the same class FND2 did for its surfaces); the functions below
//! are written against exactly the facts those callbacks will carry, so the consumer is a
//! thin shell over them when the seams land. Until then the item exists — which is also what
//! the advancements need (`world/piggybackpack`) — but using it on a mob does nothing.
//! The carry effect's icons (`carry`/`carry_2`/`carry_3` by amplifier, client side) are
//! presentation and ride along with the same follow-up; [`carry_icon`] keeps the mapping.

/// How many entities can be carried at once (PiggyBackPackItem.MAX_ENTITY_STACK).
pub(crate) const MAX_ENTITY_STACK: usize = 3;

/// The facts `pickupEntity` decides from, as an interaction callback will report them.
pub(crate) struct PickupFacts {
    /// `Level.isClientSide` — the pickup is server work.
    pub client_side: bool,
    /// The target's type is in `tconstruct:piggybackpack_blacklist`.
    pub target_blacklisted: bool,
    /// `isVehicle(player, target)`: the target is the player's vehicle, or its vehicle's vehicle…
    pub target_is_players_vehicle: bool,
    /// `isVehicle(target, player)`: the player is the target's vehicle chain.
    pub player_is_targets_vehicle: bool,
    /// The player's passenger chain down the first passenger, outermost first: for each
    /// carried entity, whether it is a player. Its length is how many the player carries.
    pub passenger_chain: Vec<bool>,
    /// The target is a player (`target instanceof Player`).
    pub target_is_player: bool,
    /// Whether the chosen mount accepts the rider (`Entity.startRiding(toRide, true)`).
    pub ride_accepted: bool,
}

/// PiggyBackPackItem.pickupEntity, decision for decision: false on the client, for a
/// blacklisted target, and when either is already the other's vehicle. Otherwise walk the
/// player's passenger chain (at most [`MAX_ENTITY_STACK`]) — refusing if a carried player
/// would stack under a player target — and ride the target on the chain's end if the end
/// carries nothing and the stack is not full.
pub(crate) fn can_pickup(facts: &PickupFacts) -> bool {
    if facts.client_side || facts.target_blacklisted {
        return false;
    }
    if facts.target_is_players_vehicle || facts.player_is_targets_vehicle {
        return false;
    }
    let mut count = 0;
    while count < facts.passenger_chain.len() && count < MAX_ENTITY_STACK {
        let carried_is_player = facts.passenger_chain[count];
        count += 1;
        if carried_is_player && facts.target_is_player {
            return false;
        }
    }
    // After the walk the chain end is the entity at depth `count`: it can take the target
    // only if it carries nothing itself (the chain ended there) and the stack is not full.
    let end_carries_nothing = count == facts.passenger_chain.len();
    if end_carries_nothing && count < MAX_ENTITY_STACK {
        return facts.ride_accepted;
    }
    false
}

/// PiggyBackPackItem.getEntitiesCarriedCount: the length of the first-passenger chain.
pub(crate) fn carried_count(passenger_chain_len: usize) -> usize {
    passenger_chain_len
}

/// The amplifier the worn pack's inventoryTick applies: carried count − 1, for a 2-tick,
/// ambient, icon-shown instance of `tconstruct:carry`. Only called while the wearer carries
/// something, so the count is at least 1.
pub(crate) fn carry_amplifier(carried: usize) -> i32 {
    carried as i32 - 1
}

/// What the carry effect does to movement speed: an attribute modifier of −0.05 with
/// MULTIPLY_TOTAL on `minecraft:movement_speed` (UUID `ff4de63a-2b24-11e6-b67b-9e71128cae77`),
/// which vanilla scales by amplifier + 1 — so the total multiplier for `carried` entities.
pub(crate) fn carry_speed_multiplier(carried: usize) -> f64 {
    1.0 - 0.05 * carried as f64
}

/// PiggyBackPackItem.matchCarriedEntitiesToCount: which depths of the passenger chain stop
/// riding when the chain is trimmed to `max_count` — every entity deeper than the kept
/// prefix ejects (keeping its own passengers, as `stopRiding` leaves them on it). The carry
/// effect calls this with 0 when the chest slot no longer holds the pack, and with the chest
/// stack's count while it does.
pub(crate) fn trim_depths(chain_len: usize, max_count: usize) -> Vec<usize> {
    (max_count + 1..=chain_len).collect()
}

/// What a successful pickup does to the chest slot (interactLivingEntity's second half).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChestAction {
    /// The chest was empty (or held something else, already moved to the inventory):
    /// one pack splits off the hand stack into the chest slot.
    EquipOne,
    /// The chest already holds packs, fewer than the carried count after the pickup:
    /// one more splits off and the chest stack grows.
    GrowOne,
    /// The chest stack already matches the carried count.
    Nothing,
}

/// The chest-slot outcome of a successful pickup. `chest_is_pack` is whether the chest slot
/// held this item (anything else there is first given to the inventory — which is also why a
/// full inventory with a foreign chest item refuses the interaction with PASS before any of
/// this runs); `carried_after` is the carried count once the new entity rides.
pub(crate) fn chest_action(chest_is_pack: bool, chest_count: u32, carried_after: usize) -> ChestAction {
    if !chest_is_pack {
        return ChestAction::EquipOne;
    }
    if (chest_count as usize) < carried_after {
        return ChestAction::GrowOne;
    }
    ChestAction::Nothing
}

/// The carry effect's icon by amplifier (CarryPotionEffect's client icons, clamped to the
/// third): presentation data for the follow-up that wires the effect.
pub(crate) fn carry_icon(amplifier: i32) -> &'static str {
    match amplifier {
        i32::MIN..=0 => "carry",
        1 => "carry_2",
        _ => "carry_3",
    }
}

/// Registration: the item's optional Mantle tooltip is the one piece of the pack that works
/// today (PiggyBackPackItem extends Mantle's TooltipItem); the interaction and inventory
/// wiring is the walled follow-up described in the module header.
pub(crate) fn register() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        pluto_lib::ItemTooltipCallback::EVENT.register(|tip| {
            if tip.item == "tconstruct:piggy_backpack" {
                tip.added.borrow_mut().push(pluto_lib::TooltipLine::translatable("item.tconstruct.piggy_backpack.tooltip".to_owned()));
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(chain: &[bool], target_is_player: bool) -> PickupFacts {
        PickupFacts {
            client_side: false,
            target_blacklisted: false,
            target_is_players_vehicle: false,
            player_is_targets_vehicle: false,
            passenger_chain: chain.to_vec(),
            target_is_player,
            ride_accepted: true,
        }
    }

    #[test]
    fn a_pickup_stacks_on_the_end_of_the_chain_up_to_three() {
        assert!(can_pickup(&facts(&[], false)), "the first entity rides the player");
        assert!(can_pickup(&facts(&[false], false)), "the second rides the first");
        assert!(can_pickup(&facts(&[false, false], false)), "the third rides the second");
        assert!(!can_pickup(&facts(&[false, false, false], false)), "three is the stack limit");
    }

    #[test]
    fn a_pickup_refuses_the_java_cases() {
        let mut f = facts(&[], false);
        f.client_side = true;
        assert!(!can_pickup(&f), "client side");
        let mut f = facts(&[], false);
        f.target_blacklisted = true;
        assert!(!can_pickup(&f), "blacklisted type");
        let mut f = facts(&[], false);
        f.target_is_players_vehicle = true;
        assert!(!can_pickup(&f), "already riding the target");
        let mut f = facts(&[], false);
        f.player_is_targets_vehicle = true;
        assert!(!can_pickup(&f), "the target already carries the player");
        let mut f = facts(&[], false);
        f.ride_accepted = false;
        assert!(!can_pickup(&f), "the mount refused");
        // A carried player under a player target is refused, but a mob target is fine.
        assert!(!can_pickup(&facts(&[true], true)));
        assert!(can_pickup(&facts(&[true], false)));
        // A full chain end that still carries something cannot take more.
        assert!(!can_pickup(&facts(&[false, false, false, false], false)));
    }

    #[test]
    fn the_carry_effect_scales_with_the_carried_count() {
        assert_eq!(carry_amplifier(1), 0);
        assert_eq!(carry_amplifier(3), 2);
        assert!((carry_speed_multiplier(1) - 0.95).abs() < 1e-9);
        assert!((carry_speed_multiplier(2) - 0.90).abs() < 1e-9);
        assert!((carry_speed_multiplier(3) - 0.85).abs() < 1e-9);
        assert_eq!(carry_icon(0), "carry");
        assert_eq!(carry_icon(1), "carry_2");
        assert_eq!(carry_icon(2), "carry_3");
        assert_eq!(carry_icon(9), "carry_3", "clamped like the Java icon picker");
    }

    #[test]
    fn trimming_ejects_everything_beyond_the_kept_prefix() {
        assert_eq!(trim_depths(3, 0), vec![1, 2, 3], "no pack in the chest: everyone off");
        assert_eq!(trim_depths(3, 2), vec![3]);
        assert!(trim_depths(2, 3).is_empty());
        assert!(trim_depths(0, 0).is_empty());
    }

    #[test]
    fn the_chest_stack_follows_the_carried_count() {
        assert_eq!(chest_action(false, 0, 1), ChestAction::EquipOne);
        assert_eq!(chest_action(true, 1, 2), ChestAction::GrowOne);
        assert_eq!(chest_action(true, 2, 2), ChestAction::Nothing);
        assert_eq!(chest_action(true, 3, 2), ChestAction::Nothing, "never shrinks on a pickup");
    }
}
