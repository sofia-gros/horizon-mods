# Consumed-stack cure boundary: minimal unapplied proposal

**UNAPPLIED / NOT NATIVE-VERIFIED.** No runtime manifest, official checkout,
effect instance, save format or gameplay policy is changed. This source-only
unit is separate from the pending actual-Game borrow probe and the protected
effect-instance prototype. The target remains official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` / Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`.

## Existing official seam, rather than another API

`horizon_world::consumable::ConsumptionWorld::apply_effect` already receives
the effect, level and consumed `StackRef`. Its `on_consume` passes that stack
through every ordered effect after listeners and before the event/decrement.
The separate `consume_effect::EffectWorld::remove_all_effects` does not receive
the stack. These are real reusable kernel contracts, not an implemented Game
owner adapter. The inspected concrete Game caller uses its separate
`finish_consuming` and `apply_consume_effect` implementation, not that kernel.

The concrete `finish_consuming(info, consumable, stack)` has the consumed stack,
but its single `apply_consume_effect(effect)` call drops that input. The exact
official ClearAllEffects arm then clears the list and applies derived status.
RemoveEffects, Native, teleport and sound are separate arms. This is the
smallest missing data passage to examine before inventing a new cure API.

## Exactly what the proposal changes

The sibling `UNAPPLIED-consumed-stack-cure-boundary.patch` is generated from
seven hash-pinned official source files and modifies only two in the displayed
diff. It is never applied by its generator.

1. Pass the existing borrowed `stack` through the one private
   `apply_consume_effect` call. Do not clone another live hand or resolve an ID.
2. Accept that explicit borrow in the private dispatcher.
3. Move only ClearAllEffects' two statements into the private
   `clear_effects_from_consumed_item(&ItemStack)` helper. Its body remains exactly
   `effects.clear()` followed by `apply_effect_status()`.

The new helper deliberately does **not use** its stack argument yet. There is
no filtering, new registry, public capability, milk default, callback, actor
lookup or mutation permission. This only makes the actual input available at
the existing clear phase. It does not establish item-curing behavior or claim
that introducing an unused argument completes a feature.

All other dispatcher arms and the remaining Game/consume source are retained
byte-for-byte after reversing those explicit edits. In particular:

- Particle/sound emission, statistics, consume criterion, food and stew
  listeners still precede the ordered consume-effect loop.
- ApplyEffects retains its probability draw and ordinary native effect calls.
- RemoveEffects still removes selected IDs and then updates status. It must not
  gain the item-curative policy merely because it also removes effects.
- Teleport and Native callbacks remain at their original positions in the list.
- The eat/drink event, current-hand decrement and use remainder remain afterward.
- Commands, expiration, death, targeted removal and arbitrary list clear callers
  are untouched. No clear-and-readd operation is proposed.

## Snapshot data is not actor or current-slot authority

The immediate-use path clones the held stack in `start_eating`; its immediate
finish branch checks `remote.is_none()`. The timed path passes its existing
stack through `complete_using_item`. The private finish/dispatcher functions
themselves do not establish a universal server-only admission contract.

The argument identifies the supplied consumption snapshot, not necessarily the
current contents of a hand after listeners/native callbacks. Official
`consume_held` still reads the current `held_mut` after effects. Changing it to
decrement an earlier saved slot would be a separate behavioral change.

Native callbacks and teleport can execute earlier in the same effect list.
Therefore a beginning-of-consumption actor/position/slot snapshot cannot be
promoted into authority for a later cure. The proposed helper acts at the same
current Game phase as the original clear, without claiming a persistent actor
binding or solving reentrant body/world changes. It adds no new callbacks, but
preserves all existing ones, including derived-status hooks. Runtime ownership,
client/server and local/LAN scenarios remain unexecuted.

## Source policy and the still-missing instance contract

The exact TConstruct `MobEffectLiquidBlock.entityInside` at
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2` checks positive fluid-type height and
LivingEntity, obtains a fresh supplied effect, assigns a new empty curative
item list, then calls `living.addEffect`. Its effect IDs, duration and empty
curative choice remain TConstruct policy. The source does not supply Horizon's
actor borrow, copy/merge/hidden-chain storage or authoritative client contract.

Current official player effects have no per-instance curative metadata, and
their five-field saves omit hidden chains. The earlier negative Mixins state
probe and the unapplied owned-payload proposal remain separate evidence.
Protecting all instances of an effect ID, looking them up later in a side table,
or inheriting a replaced client's payload would not satisfy this contract.

Before this seam may filter anything, an owned instance representation must
define update/copy/hidden restoration, explicit removal and local/LAN save
behavior. The earlier proposed rule retaining active payload on ordinary
updates is a proposal with bounded tests, not proved mixed-source Forge parity.
The existing immutable Pluto curative value also does not attach itself to
native instances automatically.

## Evidence and remaining gates

`prepare_consume_stack.py` has no apply/build mode and can write only the sibling
unapplied patch. `test_consume_stack_source.py` checks exact regeneration, the
two-file scope, reversible byte equality, phase order, caller input, absence of
new policy/owner lookup and unchanged instance/save source. These are source
checks, not Rust compilation or Game execution. Execution counts belong in the
[separate evidence JSON](../experiments/effect-instance-proposal/consume-stack-source-evidence.json);
source presence alone is not a pass. The initial pinned source run passed all
eight checks, without staging or native compilation.

Actual host acceptance, same-call effects, item-cure filtering, guest/body
continuation, save/reload and mixed-source semantics remain blocked/unproved.
The queued Game borrow/renderer CI work is not part of this unit, and no local
native fallback was run during the capacity hold.

PlutoLib remains optional shared infrastructure above Horizon/Mixins; TConstruct
chooses Pluto APIs. A future supported native seam would be adapted once inside
Pluto, while the consumer selects its own cure policy. This proposal is not an
official API change or a reason to modify/contact the official repository.

Sources:
- [Official consumption owner](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/consume.rs)
- [Official Game dispatcher](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game.rs)
- [Existing consumption kernel](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-world/src/consumable.rs)
- [TConstruct contact policy](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/block/MobEffectLiquidBlock.java)
