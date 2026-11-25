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
//! The live wiring ships too, over the PlutoLib surfaces that were missing when the logic
//! landed: the interaction callback names the player (whose entity id the entity reach finds
//! from the uuid) and the reach starts and stops rides; the chest slot — whose carried number
//! is 102 in the host's armour order (feet 100, legs 101, chest 102, head 103) — is written
//! through the player's stack operations, which create a stack in an empty slot where the
//! host's own carried-stack read only ever names an occupied one. The wiring below is a thin
//! shell over the decision functions: a use of the pack on an entity runs
//! `interactLivingEntity` (the opening PASS when a foreign chest item has nowhere to go, the
//! pickup walk, then the chest bookkeeping), and a tick of any pack stack runs the worn
//! pack's part of `inventoryTick` plus the carry effect's trim.
//!
//! Reductions, each for a named reason: the blacklist fact is always false — the shipped
//! `tconstruct:piggybackpack_blacklist` tag is empty (the original creates it with no
//! entries) and PlutoLib has no entity-type tag query, so this is exact for all shipped data.
//! A mount is judged by the ride reach's snapshot when it is requested and applied at the
//! game's own sync seam; if the game then refuses it, the chest count stands high until the
//! next tick's trim, where Java's synchronous `startRiding` would have refused the pickup
//! itself. The trim runs on the pack's inventory tick wherever the pack is carried rather
//! than on the effect's own tick: a pack in the inventory but not worn trims to zero exactly
//! as the effect would, while a pack dropped out of the inventory entirely cannot trim
//! (nothing ticks for it) where the effect would have. The server's `updatePassengers`
//! packet and the carry icons ([`carry_icon`] keeps the mapping) are presentation. The carry
//! instance Java applies shows its icon; the port's effect instance has no icon field, so the
//! applied instance differs there only.

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

/// The pack item's id.
const PACK: &str = "tconstruct:piggy_backpack";

/// The chest slot's carried number (the host's armour order: feet 100, legs 101, chest 102,
/// head 103; the offhand is -106).
const CHEST_SLOT: i32 = 102;

/// The player's first-passenger chain: the entity ids riding it, outermost first, walked by
/// first passenger as `getEntitiesCarried` walks. Capped well past any real stack (the game
/// refuses cycles when a ride is requested) so a corrupt read cannot loop.
fn passenger_chain(player_id: u64) -> Vec<u64> {
    let mut chain = Vec::new();
    let mut current = player_id;
    while chain.len() <= 16 {
        let passengers = pluto_lib::EntityReach::passengers_of(current);
        let Some(first) = passengers.first() else { break };
        chain.push(*first);
        current = *first;
    }
    chain
}

/// Whether `entity` rides `vehicle`, directly or up its vehicle chain (`isVehicle`).
fn is_vehicle_of(entity: u64, vehicle: u64) -> bool {
    let mut current = entity;
    for _ in 0..16 {
        match pluto_lib::EntityReach::vehicle_of(current) {
            Some(found) if found == vehicle => return true,
            Some(found) => current = found,
            None => return false,
        }
    }
    false
}

/// Ejects every carried entity deeper than `max_count` (`matchCarriedEntitiesToCount` over
/// the chain: each ejected entity keeps its own passengers).
fn trim_to(chain: &[u64], max_count: usize) {
    for depth in trim_depths(chain.len(), max_count) {
        pluto_lib::EntityReach::stop_riding(chain[depth - 1]);
    }
}

/// `stack.split(1)` on the hand that used the pack: the stack shrinks by one, and an emptied
/// hand clears.
fn shrink_hand(player: &pluto_lib::ServerPlayer, hand_slot: i32, held: &pluto_lib::StackData) {
    if held.count <= 1 {
        player.set_stack(hand_slot, None);
    } else {
        let mut shrunk = held.clone();
        shrunk.count -= 1;
        player.update_stack(hand_slot, &shrunk);
    }
}

/// PiggyBackPackItem.interactLivingEntity: PASS before anything runs when the chest holds
/// something else and the inventory has no free slot for it; the pickup walk; on success the
/// chest bookkeeping; CONSUME when the pickup itself refuses.
fn interact(context: &pluto_lib::UseItemOnEntityContext) -> pluto_lib::ActionResult {
    use pluto_lib::ActionResult;
    let Some(held) = &context.held else { return ActionResult::Pass };
    if held.item != PACK {
        return ActionResult::Pass;
    }
    let Some(player) = &context.player else { return ActionResult::Pass };
    if context.entity < 0 {
        return ActionResult::Pass;
    }
    let Some(player_id) = pluto_lib::EntityReach::id_of_uuid(player.uuid) else { return ActionResult::Pass };
    let target = context.entity as u64;
    let chest = player.stack(CHEST_SLOT);
    let chest_is_pack = chest.as_ref().is_some_and(|stack| stack.item == PACK);
    if !chest_is_pack && player.empty_slots() == 0 {
        return ActionResult::Pass;
    }
    let chain = passenger_chain(player_id);
    let facts = PickupFacts {
        client_side: false,
        target_blacklisted: false,
        target_is_players_vehicle: is_vehicle_of(player_id, target),
        player_is_targets_vehicle: is_vehicle_of(target, player_id),
        passenger_chain: chain
            .iter()
            .map(|id| pluto_lib::EntityReach::kind_of(*id).is_some_and(|kind| kind.to_string() == "minecraft:player"))
            .collect(),
        target_is_player: context.kind == "minecraft:player",
        ride_accepted: true,
    };
    if !can_pickup(&facts) {
        return ActionResult::Consume;
    }
    let mount = *chain.last().unwrap_or(&player_id);
    if !pluto_lib::EntityReach::start_riding(target, mount) {
        return ActionResult::Consume;
    }
    let carried_after = chain.len() + 1;
    let hand_slot = match context.hand {
        pluto_lib::Hand::Main => player.selected_slot().unwrap_or(0),
        pluto_lib::Hand::Off => -106,
    };
    match chest_action(chest_is_pack, chest.as_ref().map_or(0, |stack| stack.count), carried_after) {
        ChestAction::EquipOne => {
            if let Some(foreign) = &chest {
                if !chest_is_pack {
                    player.give_stack(foreign.clone());
                }
            }
            shrink_hand(player, hand_slot, held);
            let pack = pluto_lib::StackData { count: 1, ..held.clone() };
            player.set_equipment(pluto_lib::StackSlot::Chest, Some(&pack));
        }
        ChestAction::GrowOne => {
            shrink_hand(player, hand_slot, held);
            if let Some(mut chest_stack) = chest {
                chest_stack.count += 1;
                player.set_equipment(pluto_lib::StackSlot::Chest, Some(&chest_stack));
            }
        }
        ChestAction::Nothing => {}
    }
    ActionResult::Success
}

/// The worn pack's inventoryTick plus the carry effect's own tick, run on any pack stack's
/// tick: while the chest holds the pack and the player carries something, the carry instance
/// is refreshed (2 ticks, ambient, no particles) and the chain is trimmed to the chest count;
/// while the chest holds anything else, the chain is trimmed to zero.
fn inventory_tick(tick: &pluto_lib::InventoryTick) {
    let Some(stack) = &tick.stack else { return };
    if stack.item != PACK {
        return;
    }
    let Some(player) = &tick.player else { return };
    let Some(player_id) = pluto_lib::EntityReach::id_of_uuid(player.uuid) else { return };
    let chain = passenger_chain(player_id);
    let chest = player.stack(CHEST_SLOT);
    match chest.as_ref().filter(|stack| stack.item == PACK) {
        Some(chest_stack) => {
            if !chain.is_empty() {
                player.add_effect(&pluto_lib::EffectInstance {
                    id: "tconstruct:carry".to_owned(),
                    amplifier: carry_amplifier(chain.len()) as u8,
                    duration: 2,
                    ambient: true,
                    particles: false,
                    // The original's six-argument construction (ambient, no particles,
                    // icon shown) renders a custom carry icon and counts in the HUD's
                    // effect shift, so the icon stays shown here.
                    show_icon: true,
                });
                trim_to(&chain, chest_stack.count as usize);
            }
        }
        None => trim_to(&chain, 0),
    }
}

/// Registration: the item's optional Mantle tooltip (PiggyBackPackItem extends Mantle's
/// TooltipItem), the interaction and the inventory tick.
pub(crate) fn register() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        pluto_lib::ItemTooltipCallback::EVENT.register(|tip| {
            if tip.item == "tconstruct:piggy_backpack" {
                tip.added.borrow_mut().push(pluto_lib::TooltipLine::translatable("item.tconstruct.piggy_backpack.tooltip".to_owned()));
            }
        });
    });
    pluto_lib::UseEntityCallback::EVENT.register(interact);
    pluto_lib::PlayerInventoryEvents::TICK.register(inventory_tick);
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
