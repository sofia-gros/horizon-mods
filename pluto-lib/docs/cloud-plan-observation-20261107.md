# Cloud composition: actual e907 plan observation

This metadata-only candidate records a real complete plan. It does not prove Rust
typing, Game9 execution, cloud spawning, inventory charging, LAN, or gameplay.
No Rust payload, manifest, checker algorithm, official-source pin, protected
target, dependency entry, or hook order is changed.

## Evidence and exact changes

- Runtime: `e9073648b3c4cf4d49f27bc2e1ed60997f430586`, tree
  `afb6ac2ff3e824828cdde94e6e4686e0ffbe2094`.
- Controller: `7082868bcbf37ed3eabd32f40ddf0cf08a4ee30c`, run
  [42328100345](https://github.com/sofia-gros/horizon-mods/actions/runs/42328100345).
- Complete plan: 6,431,102 bytes, SHA256
  `dc4fbc3c54144f2f95728986b4e9bfb3700235655b18a3af3c8d1a6113dcb86b`.
- Published result:
  [c50b6acb](https://github.com/sofia-gros/horizon-mods/blob/c50b6acbe7f7eebe1032149dbcbf8cf6033b61f4/pluto-lib/docs/playtest-next-plan-result-20261107.json).

The recovered plan, receipt and package maps were checked against every size and
SHA256 in the artifact manifest before interpretation. The package maps bind the
runtime head/tree. The existing published 0412 contract was compared with the
actual schema-6 IR: all 19 module entries, 27 hook entries, 45 state entries and
16 hook-order sites match, except the intentionally null moved cloud-module
fingerprint. The observed fingerprint is
`540e1e6e0eacd338678a6e6a981a5496eabb927cbe4a20d7713830339677b042`.

Only four metadata leaves change: candidate runtime commit, candidate observation,
that fingerprint, and the explanatory note. Historical 9cc1 evidence remains
historical. `tests/cloud-plan-observation-delta.json` reverses these changes to the
complete original contract bytes with SHA256
`19f1c7a36f50f12805caeaaf99c2a3e33f8e273abc619f696fa52b5e648a70c8`.
The former null-rejection regression now reconstructs and tests that exact prior
contract. A new regression limits this update to those four leaves and checks the
unchanged checker, native-source metadata and five cloud helper files.

## Crafting changes after the earlier cloud cutoff

The ordinary crafting intake is present in the new plan and was not dismissed
because the 19-entry set happened to match. Its relevant actual sources were
compared with the earlier raw plan and the independently reviewed ordinary intake.

`source-crafting-station::open` adds exactly one call to
`tconstruct_station_transaction::ensure()`. Removing that line restores the full
old packaged module bytes. It is after the remote rejection, datapack preparation
and guest early return, but before local station block/menu changes. The guest
opener `source-crafting-lan`, guest-station-open hook and their bound states remain
byte/IR identical. The new station registration takes no Game argument, stores a
transaction callback and does not execute that callback during registration.

The two new Recipes-constructor hooks invoke the same OnceLock bootstrap before
their native bodies. It registers stored policy then pair derivation; the first
registration is not rolled back if the second fails. These registration functions
do not receive the active Game/guest or allocate cloud entity IDs. The five removed
legacy crafting hooks target recipe matching, remainders or recipe placement,
not the cloud actor/commit boundary. Their policy delegation is governed by the
separate crafting composition guard. This cloud observation does not assert that
all crafting behavior or those Rust types were executed successfully.

Earlier native IDs and inventory/menu effects remain allowed before a later cloud
operation. None of this inserts a callback into the checked successor, factory,
reserve, take-ID, assign and push tail. The metadata update introduces no broad
ID, position or count allowlist and does not turn a snapshot into an authority or
ABA guarantee.

## Executed checks and remaining gates

The 21 required official files were reacquired from immutable
`horizon-rust/horizon@41f822b659789780f58d32c63ddfb6887ea1b8bf` and checked against
both metadata and actual plan inputs. Five cloud helpers were read from immutable
e907 source, hash checked, and compared with their real packaged plan bodies.

The original complete checker rejected this actual plan for its missing reviewed
observation. The unchanged complete checker passes with this metadata against the
same full plan, official subset and helper files. All 38 pure checker regressions
passed (the prior 37 plus one metadata-only regression). No native process,
metadata command, new plan, apply or Rust build was run for this update.

The original CI reported eight passing and four rejecting guard groups. This
local cloud recheck is one resolved metadata condition, not a claim that the
entire guard set now passes. Central must compose the independently reviewed
metadata updates and run its normal all-guard gate before apply/build. The real
`horizon-client --lib` Game9 inventory and Windows default build remain pending.
