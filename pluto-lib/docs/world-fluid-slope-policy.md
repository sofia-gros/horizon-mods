# Bounded slope routing: PR716 implementation candidate

This is a source implementation candidate, not a completed PR migration. The
chosen PR is [#716](https://github.com/sofia-gros/horizon-mods/pull/716), head
`889fb19ff610984b5a69029ea978892ac090d7fd`. Its four metal families require
tick delay 30, drop-off 3 and slope search distance 3. Final cumulative source
`d82b7664cfda55a8a8e850e97808c77dcdbcb732` is used for renamed `_fluid` block IDs
and retained interaction/visual facts, rather than reinstating old block collisions.

The implementation base is ordinary
`9e032ef5b013487ea08b997eb08df84825fb09ba`. Horizon is
`41f822b659789780f58d32c63ddfb6887ea1b8bf`; Mixins is
`5a64af938ec86b00d53d16a408aabc0038fb3e57`. No official file, manifest, guard,
stage, launcher, lock or workflow changes are included.

## Implemented common service and real caller

- `fluid::SlopeDistance::new(1..=4)` is an owned validated routing value. The
  distance counts steps after the first adjacent cell, like native
  `level/fluids.rs::{slope_distance,spread_targets}`. Four reaches at most five
  horizontal cells from the origin. Out-of-range values are errors.
- `Declaration::with_slope_distance` selects it before initial registration.
  Definition's seven fields, its JSON schema, generated blocks/models, legacy
  register calls and explicit render rules are unchanged.
- The existing registry owns optional routing alongside definitions and render
  facts. One lock and one revision publish the entire batch. A conflict rolls
  back every map; an identical or legacy repeat preserves an existing policy.
  An unspecified policy preserves legacy all-direction behavior. It cannot be
  retrofitted onto an already registered legacy definition, replaced or removed.
- The existing game-private `world-fluid.rs::tick` resolves the immutable policy
  then calls `tick_with_slope` or the unchanged default `tick`. A poisoned/missing
  registry identity does not silently fall back. The lock is gone before world
  callbacks. No second registry, actor capability or scheduling hook is added.
- The optional kernel collects all chosen directions before horizontal writes.
  Minimum reachable downward-hole distance wins, all ties remain, and no hole
  keeps all eligible directions. An encountered unloaded read defers that
  horizontal pass and schedules the existing retry. The selector performs at
  most 968 reads and depth four, uses checked coordinate offsets, and never writes.
- Writes still go through the original native authoritative/sample check,
  `Spread::Fluid` protection, ordinary `set_block` and native scheduled tick.
  Vertical priority, finite levels, source/non-source ownership, default write
  order, current-cell decay and refusal behavior are preserved.

The selector is **not** a simultaneous world snapshot. Existing sample/replace
revalidation remains at each actual write. It does not grant permission to cross
or replace native walls, plants, foreign fluids, waterlogged cells or source
blocks. Native `is_water_hole` treats same-family fluid below as a routing hole;
that fact still grants no write to an existing source. The finite kernel remains
its own nine-level contract, not the original sixteen-state runtime.

## TConstruct connection and explicit activation blocker

`tconstruct/experiments/world-fluid-slope-candidate/candidate.rs` uses the real
Pluto `Declaration`, display pair and `SlopeDistance` for iron, gold, copper and
cobalt. `families.json` preserves their complete final original policy objects,
including the fields this candidate cannot implement. This is an excluded
consumer source candidate, with no manifest entry, startup registration or
content installation. It is not counted as ordinary family activation.

Enabling even these four metals would still omit their original burning damage,
travel/current/fall behavior, camera/fog and lava path behavior. The ordinary
custom blocks are not native `FluidKind`. Existing readonly contact queries
cannot apply effects; the native item-cloud Game frame does not authorize a
world-contact callback. Persistent curative/effect copy/save contracts also remain
separate. There is therefore no honest fully playable family claim in this unit.
Ichor/Cinderslime upward policies and all other families remain unactivated.

The minimal remaining shared contact connection needs an actual synchronous
native actor-owner phase, not a generic callback carrying only an ID or a delayed
queue. It must preserve current actor/body/level continuation after effects and
teleports. Until that exists, the four-metal ordinary registration remains blocked;
no owner=None, side table, fake entity or disabled damage is substituted.

## Validation and next gates

- Ten pure Python source checks pass: all three previous payloads reverse to
  exact base Git blobs/SHA256, unchanged Definition/content/native write and
  neighbor/bucket bodies, one registry, bounded source inventory, and 23-PR scope.
- Fourteen real API Rust tests are written in `fluid/slope.rs`, but **not run**.
  They cover 1..4 bounds, direct/shortest/tied/no-drop routing, depth convention,
  protected cells, unloaded retries, integer edges, read budget, refusal,
  default/explicit difference, vertical priority and registry atomicity/repeats.
- One excluded consumer Rust mapping test is written, but **not run**. It checks
  final IDs, source numeric facts, display mapping and no automatic registration.
- Independent source review found no concrete blocker in the routing/registry/
  existing native tick delegate. It is not Rust, planner or gameplay evidence.

Before ordinary intake: run the complete actual public API suite including these
14 tests, compile the explicit consumer candidate against that API, obtain a new
full composition plan and unchanged strict guards, and type-check the actual host.
Before PR716 completion/main/closure: implement and test the remaining family
semantics and acquisition/save/LAN routes. The old cloud actual-plan/guard result
and historical fluid API results are not evidence for this new unit. Actual
Game9/live operations remain independently unrun.

The 23-PR decisions are in `world-fluid-intake-20261107.json`; no PR is closed or
marked feature-complete. Central owns integration/main and any later PR closure.
