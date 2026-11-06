# Cloud boundary candidate for the latest playtest

This is a source-reviewed metadata candidate on integration commit
`e128d61478971e247b1e3e2e09516cf99652f9fc`, not an accepted complete plan or
playable build. Target Horizon is `41f822b659789780f58d32c63ddfb6887ea1b8bf`,
Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`.

## Exact limited change

Only the revision label and ten of seventeen whole-file source SHA-256 pins in
`tools/cloud-insertion-boundaries.json` change. Replacing those values with the
recorded old values reconstructs the complete old metadata hash. The validator,
all five cloud runtime modules, helper hashes, module identities/visibility,
exclusive files, protected prefixes and state restrictions are unchanged.
There is no allowlist, fabricated node/fingerprint, new hook or consumer activation.

The supporting [immutable source audit](https://github.com/sofia-gros/horizon-mods/commit/06b5bff675efe1c96c55fe3ae01cc670cd6e1c94)
compares all seventeen exact official Git blobs, forty-one selected spans and
fifteen required Game fields. Forty spans and all fifteen fields are identical;
the changed dedicated-server loop preserves the selected open/Ready ordering.
This is targeted source evidence, not a whole-program alias, callback or type proof.

## Existing complete-composition blockers

The exact e128 manifest and payloads contain four modules and sixteen hooks that
the unchanged cloud guard rejects. All twenty manifest entries and twenty payload
files are identical to main `08afa992d5091e03360f3473ea93fd17aa793987`; PR945 did
not introduce these conflicts. The companion JSON records every entry, payload
Git blob/SHA-256, source link, immediate effects and required follow-up.

- Four modules: workstation tab delegation, spawn-protection queries, bottle fill
  delegation, and direct fluid-cloud creation.
- Sixteen hooks: command, guest close/tick/open, block-change cache cleanup,
  area/transform packet bookkeeping, glowing action/protection, materials-book
  request, crafting viewer updates, bottle use, and three splashing guest phases.
- `glow-resolve-guest-protection` directly targets native `enter_guest` at HEAD.
  Its body writes TConstruct protection state and calls the native protection
  query. That is narrower than changing native identity fields, but is not an
  automatic exemption from a protected actor-entry contract.
- Several hooks cancel native processing, swap acting guests, send packets or
  apply effects. They cannot be treated as harmless because their modules share
  a file or because native function text is unchanged before injection.

This candidate intentionally still rejects every one of these twenty overlaps.
A successful synthetic cloud-only shape is not a successful TConstruct plan.
Follow-up Guide modules or any new source changes require their own composition
review; unrelated additions are not silently certified by this inventory.

## Concrete consumer type and authority blocker

The existing `source-fluid-cloud.rs` constructs `Breath` without required
`victims`, and `PotionCloud` without required `settings`, `owner` and `guest`.
Those fields exist in both audited 624c and 41f native types. This is a static
initializer incompatibility, not a newly observed compiler diagnostic.

It also calls native `take_entity_id` and pushes directly. The one existing
`source-splashing::block` call does not pass an explicit source actor or level
handle. Original TConstruct assigns `context.getEntity()` as owner, so supplying
`None` or selecting a nearby/current player to make the struct compile is not a
valid migration. Pluto's existing insertion service resolves real handles,
requires Ready/server/current-level and matching attribution, prepares capacity,
then allocates/publishes. It deliberately rejects a detached active guest whose
LAN roster cannot be resolved. It must not be weakened to accommodate this caller.

The next ordinary repair must establish that caller's actual source context,
preserve TConstruct duration/scaling/effect policy, and distinguish failed
publication from success before consuming fluid. No such behavior is activated
by this metadata commit. Merely adding fields or a module-location exemption
would not resolve this issue.

## Validation and remaining gates

Eight pure Python tests validate the exact inverse delta, unchanged validator
and runtime payloads, seventeen source records and twenty manifest/payload pins.
Synthetic plan inputs exercise the unchanged validator: the isolated five-module
shape passes, every reserved overlap still rejects, and source/owner drift rejects.
These tests do not invoke Mixins, Cargo, a compiler or Game.

The first pure run had seven passes and one test-only `KeyError`: the inventory
used the hook's `kind=HEAD` where the enclosing manifest section was required.
The report now records `entry_section` separately. No metadata, runtime, guard
algorithm or acceptance rule changed to correct that fixture error.

Required before claiming latest compatibility:

1. Independently review the source delta and each effective overlap, resolving
   the concrete actor/publication blocker rather than weakening the guard.
2. Obtain an accepted **complete installed** 41f/5a64 plan. Bind exact package
   modules, inputs, ownership, resolved targets and all overlapping effects.
3. Pass all complete-plan guards, then apply and check the actual host library
   and binary. Historical 624c proof remains historical.
4. Run separately authorized native admission/construction/publication tests.
   Source checks and prepared tests do not prove actor continuation or spawning.
5. Build and launch Windows with production resources before claiming playability.
   Synchronous contact effects, native per-instance cure metadata and language
   merge/root-key experiments remain outside this candidate.
