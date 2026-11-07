# Remaining storage/equipment authority boundaries

Read-only audit at MOD source `4d82580b32a1c040001add8c853d93d331bc493a` and official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`. The additive native Wire boundary passed its actual applied-items 8-case proof. No consumer or implementation changed in this audit. “Active” below means present in the TConstruct manifest, not that the complete TConstruct package or live runtime has been verified on this host.

## What the completed APIs already own

- `inventory-api/src/lib.rs`: movement proposals, ordered candidate selection, caller-defined snapshot checks and encode/decode guards. None publishes a live edit.
- `inventory-api/src/storage.rs`: independent partitioned proposal copies, one load per key, failure before publishing. No authoritative commit/rollback callback.
- `stack-storage.rs`: strict native save eligibility, canonical Tag decoding, recursive retained-wire query and observable snapshot comparison.
- `stack-snapshot.rs` plus native `item_data::pluto_wire_read`: additive value comparison of retained baselines, not actor/world/slot authority.
- `nbt-storage.rs`: non-mutating owned-key compound proposals with exact unknown-field/UTF-16 preservation; caller owns schema and aliases.
- `inventory-api/src/equipment.rs`, `equipment-view.rs`: captured semantic read facts, explicitly no write or transaction token.
- `armor-wear-observation.rs`, `block-feedback.rs`: source-guarded native observation phases. The former's observed actor/world tuple is explicitly not a generation token.

The remaining difficult boundary is which live object may be changed at which native phase, not another representation codec.

## Priority 1: active held-item wear and the native calculation/commit phase

Manifest-active callers:

- `tconstruct/mixins/tconstruct/wear.rs:1-13`, target `Game::mine_block_with_held`: clone `inventory[selected]`, calculate loss, then mutate the current `inventory[selected]` without a captured-stack comparison; finally update TConstruct's current used-stat path.
- `ranged-wear.rs:1-13`, target `Game::damage_held`: clone acting hand, calculate loss, then accept the current acting-hand stack by item ID only.
- `melee-wear.rs:1-21`, target `game::mace::Game::weapon_wear`: calculate custom material attack cost and loss, then accept the current acting-hand stack by item ID only.
- Other concrete same-pattern consumers include `source-glowing-walk.rs:23-26` (armor slot 3) and `source-projectile-add.rs:14-17` (acting hand). Their material/overslime/cause policy remains TConstruct-specific.

`source-overslime-host.rs:20-35` temporarily writes a per-actor modifier-cause context, invokes `Game::durability_loss`, restores the previous context, and returns calculated loss plus overslime spend. On the exact official source, `game/enchant_effects.rs:69-86` performs enchantment evaluation and consumes world RNG; it does not itself call the native inventory-damaged hook. Therefore a stale-slot bug is not claimed reproduced by this read-only review. Additional Mixins handlers around that mutable-Game method can introduce reentrant context or slot changes, and a future common boundary must define that composition.

The native **wear** operation is different: `game/enchant_effects.rs:90-108` calls durability loss, conditionally calls `native_item_damaged` for a server player, applies cancellation/point changes, then triggers item-durability advancement. The trigger (`game/advancements.rs:1361-1387`) can change advancement state. Rejecting a later slot write cannot claim rollback of RNG, native hooks or advancement effects, and must not rerun them.

Native commit tails are different as well:

- `game.rs:13603-13636`, `damage_held`: acting hand, native max lookup (special trident path), damage, broken-stack removal, broken statistic and equipment-break feedback.
- `game.rs:13137-13210`, `hurt_armor`: collect four indexed native losses first, then mutate/remove pieces, then per-piece break feedback and equipment refresh.
- `game/shield.rs:118-186`, `after_block`: capture explicit active-use slot, possible sound RNG/audio, threshold/base/factor calculation, native wear, explicit-slot mutation/break/use cancellation, then attacker-disable handling.

TConstruct `transaction.rs:1728-1748` intentionally retains its broken tool and owns `tic_max_damage`, `tic_broken`, source unbreakability and overslime policy. Moving this policy into a generic Pluto wear algorithm would be incorrect.

**Borrowed-capability conclusion:** a synchronous mutable slot borrow can prevent reentrant slot changes only while no callback receives the surrounding mutable Game. The existing calculation and native side-effect methods need `&mut Game`; a slot borrow cannot remain held across those calls. Releasing/reborrowing needs explicit physical slot and actor/world scope revalidation, and does not undo already-consumed effects. A semantic equipment snapshot or the new value comparator cannot supply that authority. A safe next implementation would first establish a source-guarded exact native phase contract, or a proven native lifecycle/generation token, rather than expose a general public commit closure.

## Priority 2: machine metadata/live inventory/notification ordering

`source-melter-store.rs:190-207` now uses shared partition staging, then commits under one exclusive `&mut World` without callbacks inside that loop. All referenced chunks were loaded during staging; there is no demonstrated interleaving that removes one between the two loops. Do not relabel defensive `get_mut` failure branches as a reproduced partial-write bug.

Other callers span multiple representations and native notification work:

- `source-chute-hopper.rs:136-150` writes live hopper state, attempts saved metadata, rolls the live state back on that save failure, then updates a projected local container and notifies.
- Its casting route at `:273-283` publishes raw saved state and both live inventories before `container_changed`.
- `source-machine-runtime.rs:302-315` saves Casting Tank metadata/live slots and calls comparator/container notifications. Callers at `:370-374` and `:390-392` update the acting hand **after** that helper returns.
- The Cannon held-container path at `:433-448` revalidates, saves metadata, calls comparator/container notification, then updates/gives the held result.
- Official `game/machines.rs:565-585` marks storage and invokes native block/comparator processing through `with_level`; it is not a purely informational message.

The generic staging and patch APIs deliberately do not own those sequences. Before extracting a commit adapter, record its exact representations, callback boundaries, raw rollback values and notification phase. No callback-safe atomicity or live actor/slot guarantee follows from `stage_updates`, `patch_compound` or equality alone. This audit does not reproduce a notification-induced inventory change.

## Priority 3: existing comparison/query adoption, not a new API

Remaining repeated count/damage/native-component and outer-wire checks exist in `source-machine-runtime.rs:281-283`, `:349-353`, `:440-441`, `:704-706`, and `:799-801`. They do not all have the same contract: some reject retained wire as policy; others combine save eligibility with revalidation. The existing shared query and comparators cover the mechanical pieces, but replacing them requires explicit runtime-bundle/nested-wire/private-baseline acceptance decisions and actor/slot scope checks. No blanket replacement is proposed.

In contrast, `source-casting-tank.rs:61-69` compares decoded saved-slot identity to decide whether raw bytes/fields may remain unchanged. This is a persistence decision, not a live snapshot. It must not automatically gain runtime BundleSelected or retained-baseline identity, and the stricter live comparator is not a drop-in replacement.

## Dormant armor path is not an active mutation consumer

`source-armor-wear-record.rs`, `source-armor-wear-end.rs`, and `source-armor-wear.rs::matched_losses` contain a future record/match/apply path. The record and end injections are in `tconstruct/disabled-injections.toml:54-67`, not the active manifest. `matched_losses` uses native PartialEq and does not bind the original armor index; the pending record stores `(ItemStack, loss)` keyed by optional actor connection. These are real recovery requirements if that path is restored, but must not be reported as currently active or repaired by the new comparator. The enabled start hook alone does not make the complete wear path active.

## Recommended next gate

No new generalized mutation API should be added from this audit alone. First choose one active caller, preferably ranged held wear, and establish the exact calculation/commit scope and native effects on the approved official target. Require phase-order and slot/hand/actor changes, same-item component replacement, nested/private wire, cancellation, RNG/hook single-execution, break feedback and local/LAN behavior evidence before consumer migration. Keep source-owned durability and machine schema rules local. Existing read-only block/armor observations remain read-only.

The active ranged boundary was subsequently investigated and one official plan was rejected. See [the ranged invocation audit](ranged-wear-phase-audit-20261105.md) for the exact native order, physical-hand scope and diagnostic.

The concrete machine publication/notification phase is detailed in [the machine commit audit](machine-commit-phase-audit-20261105.md), including the native spill boundary and the limits of the existing bounded fixture.
