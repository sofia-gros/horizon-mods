# Remaining custom-fluid contact effect boundary

Read-only audit at official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`.
No contact effect is activated by the shared query or cloud insertion groups.

## What is already reusable

The registry, registered-cell decoder, immutable native World view and bounded
point/box contact queries belong to PlutoLib. The actual minimal resource loader
fixture exercised eight tests (three decoder plus five View tests); this proves
its generated fixture semantics, not production resources or gameplay.

`game/native_entity_motion.rs::native_entity_bounding_box` supplies an official
handle-based shape observation. Its server-player branch reads the guest roster
through `native_target_guest`, not the temporarily swapped PlayerMotion. It is
therefore not by itself proof of the exact body during a contact callback.
Mobs use the native entity walk; unsupported kinds return None. Future geometry
access must preserve which moment and actor representation it observes.

## Missing execution context

`block_contact.rs::HOOK` is a static function of dimension, Event and a fall-damage
reference. Inside carries entity ID/kind, block/state and position; it does not
borrow Game or the actor. `game/native_hooks.rs::native_block_entity_inside`
explicitly documents that the simulation owns the entity during the callback.
Sweeps also notify for dropped items and projectiles. Repeated hits of one block
in one pass are suppressed; idle overlaps still notify.

A deferred queue that later finds the entity would change the observed position,
entity lifetime, guest frame, damage ordering and possibly effect cadence. It is
not a mechanical implementation of TConstruct's immediate `entityInside`.
The safe next execution boundary must have a synchronous mutable actor context
or explicitly proved equivalent order, with real loaded-world and source fluid
immersion facts. It must distinguish unsupported/unavailable from no contact.

## Source-specific policy remains in TConstruct

At TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`TinkerFluids.java` declares 100-tick contact effects for Venom (Poison), Honey
(Slowness), Liquid Soul (Blindness), Earth Slime (Bouncy), Sky Slime (Ricochet),
and Ender Slime/Molten Ender (Enderference). Values and identities are content,
not generic PlutoLib defaults. Their `MobEffectLiquidBlock` requires a living
entity with positive height in that fluid type and an empty curative-item list.

`BurningLiquidBlock` first checks fire immunity and positive fluid-type height,
then sets fire, attempts lava damage, and plays its world-random sound only when
damage succeeds. Replacing that with a once-per-family volume sum, universal
burn timer, or queued damage would need separate source-equivalence proof.
The existing registered-cell query measures positive volume in its explicit
geometry, not Forge's full fluid-type height algorithm or callback ordering.

## Curative metadata is not represented by the current native player instance

The official `horizon-client-render/src/effects.rs::EffectInstance` contains
identity, duration, amplifier, ambient/visibility and a hidden chain. It has no
curative-item field. `update` can retain a hidden predecessor; `downgrade` restores
it. `PlayerEffects::saved` serializes only five top-level values. The generic
`horizon-world/src/status_effect.rs::EffectData` likewise has no cure metadata.

Official `Game::apply_consume_effect` (the ClearAllEffects arm in game.rs) clears
the player's entire effect list, then updates status. Explicit RemoveEffects,
commands, death and expiry are distinct removal paths. Simply protecting an
effect ID everywhere would incorrectly protect unrelated instances and removals.

A future shared cure mechanism needs instance-bound metadata and defined update,
hidden-chain, removal, save/reload and server/client behavior. TConstruct supplies
its empty cure set and effect policy. Supporting only the current active ID or
silently dropping metadata is not equivalent. No such mechanism is added here.

## Bounded completion criteria

1. Prove synchronous actor/world/immersion access and exact callback ordering.
2. Provide reusable explicit fire/damage/status operations without source values.
3. Establish instance-bound cure and hidden-chain behavior before applying these
   protected effects; preserve explicit command/death/expiry removal semantics.
4. Add real actor/mob/guest, save/reload and LAN tests against official assets.

These are technical integration gaps, not permission to guess an owner or ask
the user to choose arbitrary game behavior. The separately staged owner-bound
cloud insertion solves only its explicit actor/world admission and publication
boundary; it does not close these contact-effect requirements.

Sources: [official contact](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/block_contact.rs),
[official native shape](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/native_entity_motion.rs),
[official effect instances](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client-render/src/effects.rs),
[TConstruct definitions](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java).
