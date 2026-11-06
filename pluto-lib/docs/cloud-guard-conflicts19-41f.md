# Remaining 19 cloud guard conflicts: source triage

Exact ordinary base: `e128d61478971e247b1e3e2e09516cf99652f9fc` (tree `4bb4984ab0225731697d8d70c312cdaa2f247b4f`). Official target: Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf`, Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`. All 19 manifest entries and payloads are unchanged from main08afa. This report excludes the separately blocked `source-fluid-cloud`; attached-only candidate70a589 is not adopted. Later Guide/integration changes must be compared anew.

The companion JSON preserves each exact manifest entry, source blob/SHA/link, effects, dependencies, suggested next step and unresolved actual-plan identities. Every `admissible_now` is false. No guard, runtime, manifest or official file is edited, and no native plan/test has run for this triage.

## What can be narrowly distinguished

**Six isolated policy-state candidates:** `source-area-protection`, `source-area-guest-hit`, `source-area-guest-destroy`, `source-transform-guest-hit`, `glow-resolve-guest-protection`, `source-splashing-guest-head`.

Their reviewed direct footprint is native/guest read-only facts plus fresh collections or TConstruct-specific map insert/remove. The area plane is dependency-free arithmetic. The native protection predicate only reads dedicated mode, dimension, spawn/radius and operator UUID membership. Guest bounds/pose/dimensions are reads. None writes native lifecycle/Ready, Game's roster/acting identity, native level epoch, next_entity or breath_clouds. This is a concrete source distinction, not a claim that arbitrary helpers at these locations are safe.

`glow-resolve-guest-protection` nevertheless targets **enter_guest HEAD**, so it does overlap an owner transition. Only that exact noncancelling helper and its map/read dependencies could be considered while preserving the native transition. `source-splashing-guest-head` clears a pending operation and snapshots identities; it does not establish the authority of the later deferred effect hook. Its state types include an ItemStack, so the exact state/accessor and component ownership contract remains an input, not an interchangeable map name.

**One inventory-only candidate:** `command` handles exactly `tconstruct parts` and `tconstruct assemble`, then returns Break; all other commands Continue. It does not alter the native actor lookup/Ready/ID/publication fields, but changes inventory and command consumption. A bounded source exception would not prove command permissions or a later held-item continuation.

**One actual world-storage mutation:** `part-selection-block-change` is not only cache cleanup. At `World::set` HEAD, a TConstruct remote-client flag gates copy-on-write removal of a saved material-block NBT entry, besides clearing many mod maps. Pinning that flag's owner/setters and the chunk/saved schema is necessary before claiming it cannot affect server state. The native World::set/block/loaded contracts must stay intact.

## Actual side effects that must not be hidden

**Five allocator/callback-reachable entries:** `source-bottle-fill`, `source-empty-bottle-use`, `fluid-guest-close`, `fluid-guest-tick`, `crafting-station-lan-update`.

- Native `fill_held` can overflow inventory to `throw_from_eyes`, then `spawn_item`; the latter increments `next_entity`, publishes the item and invokes native join/spawn hooks. The short bottle wrapper is therefore not effect-free.
- Fluid return calls `lan_place_back` then `guest_menu_op`, and crafting viewer updates use that same native menu boundary. It drains crafted/equipped/brewed events, can call `as_guest` for callbacks, and invokes `drop_guest_items`. This review records that reachable path without pretending every invocation takes every branch. The final drop implementation and installed callback graph remain to be closed.

These are existing gameplay effects, not evidence of a bug by themselves. They do not automatically violate a cloud insertion's checks: an insertion revalidates at its own call. But they cannot be exempted solely because they sit outside the final cloud push statement or have one-line source wrappers.

**Three actor/action-changing entries:** `glow-left-guest`, `source-splashing-guest-return`, `source-splashing-guest-aim`.

- Glowing intercepts a packet, enters/leaves an actual guest turn, places blocks, performs wear/callbacks and acknowledges/sends updates.
- The splashing-return ID is misleading: its actual manifest kind is HEAD on `lan_swing_back`. It consumes deferred work, repeatedly swaps actor/targets, applies effects and finally commits fluid/wear after another actor swap. A comment claiming one packet turn is not a proof against callback/teleport/lifetime changes.
- The aim hook changes the real host guest's yaw/pitch and replaces a target/eye result. It must retain the exact prior native range checks, finite input assumptions, packet/hand binding and return contract.

**Three delegated composition closures remain unclosed:** `source-workstation-tabs-lan`, `guest-station-open`, `materials-book-request`. The tab wrapper reaches the native container path plus its TConstruct opener; that opener mutates storage, machine/menu state and sends packets. The book packet path invokes a mutable page resolver not audited here as a complete closure. The Guide owner should reconcile its later changes, rather than bless the old e128 source for them.

## Minimal build-oriented adaptation proposal

Do not move modules merely to evade a protected-file rule. The tab, area-protection and bottle wrappers access private native methods in their existing modules; sibling placement is not proven legal. Their module definitions themselves add no hook or automatic initializer, which can be distinguished in an exact installed plan. A future shared wrapper can retain the private owner, but must not hide its native effects.

The smallest plausible first guard adaptation is a **reviewed per-entry contract for the six isolated-policy entries**, then separately the two-command domain, not a file/prefix/ID allowlist. It must require:

1. Exact official revision/input bytes and actual complete installed plan, not the old ordinary-Pluto-only plan.
2. Exact mod owner, canonical file/target, kind, cancellation, priority/ordinal/require and every binding/type/source. Actual node/fingerprint values must be obtained from that plan, never guessed.
3. Exact helper/module bytes and visibility; closed read-only dependency spans; exact state types/constructor/accessor placement. No colliding module/state or competing hook is admitted silently.
4. For the enter_guest entry specifically, unchanged original owner swap/return behavior and the precise noncancelling HEAD phase. Ready/epoch/native handle checks and cloud reserve-before-ID/commit ordering remain fail-closed.
5. Actual host type verification and finite effect/phase tests. A source result alone is not installed compatibility.

The other twelve entries need their effectful closure or mutable-source owner reviewed separately. Three module definitions can potentially be admitted as exact inert definitions before their caller effects are qualified, but that alone must not claim end-to-end gameplay safety. None requires deletion of an existing feature as a shortcut.

Until the complete plan exists, retain all existing guard rejections. The counts above are evidence categories, not counts of repaired or compatible features.
