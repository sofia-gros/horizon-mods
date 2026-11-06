# Damage equipment context (development)

The block-feedback follow-up on this branch adds one generic callback boundary
to each copied caller. See [block feedback](block-feedback.md). Validation below
and `damage-equipment-validation.json` describe the earlier `2b32def7` equipment
cutoff, not the newer callback extension.

This is part of the grouped player-protection API, targeting only official
Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and official Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. It is source-only, with no fork,
WASM dependency, new native fields, pointer identity, equipment registry or
component serialization. TConstruct formulas and provider migration are not
included.

## Ownership and handoff

`horizon_client_lib::pluto_equipment::Snapshot` clones the native ItemStack
values without filtering counts, damage, opaque/custom components or unknown
fields. It is not a second component DTO or a transaction. `get(EquipmentSlot)`
returns `Some(None)` for a captured empty slot, and outer `None` for unsupported
body/saddle slots. Head, chest, legs, feet, main hand and off hand are captured.
The snapshot records the current actor's entity number and dimension name;
these are contextual metadata, never persistent or globally unique identity.

The Game adapter reads the active actor's armor/hands and dimension, immediately
before borrowing that same PlayerMotion for its native hit. Native `as_guest`
turns have already swapped that guest's inventory/motion/world into Game at this
point. Client-side games and speculative replay motions supply no snapshot.
The snapshot is borrowed by the native protection adapter, which projects it
through the shared semantic ItemFacts converter for registered providers. It is never installed in a thread-local/global ambient store, so
nested or unadapted hits cannot inherit the outer snapshot. Errors, cancellation,
unwind and normal return release the ordinary local value.

Providers receive the host-independent request and per-ID contribution capability.
`request.equipment` exposes the shared `EquipmentSlots` projection; no native stack
or uncaptured active-use state escapes. See [the facade](protection-provider-facade.md). They
must handle `None` explicitly. The caller's pre-hit snapshot is read-only, not live
post-wear inventory and not permission to commit inventory changes. Consumers
own their component interpretation, eligibility and formulas. The lower-level
`health::pluto_protection::hurt_with_equipment` API accepts a fresh snapshot;
its caller must guarantee same-world capture because a bare PlayerMotion does
not identify its Game. The supplied Game adapters establish that association
without a registry or address key. A mismatched actor/client/replay request is
rejected by this explicit low-level entry point.

## Two pinned native callers

The native exact pre-hit local hook is rejected by current official Mixins:
`MODIFY_VARIABLE` on `hurt_player_from` encounters the opaque `world_stations!`
macro. Rather than approximate with an earlier ambient snapshot, PlutoLib owns
lowest-priority cancellable HEAD adapters for:

- `Game::hurt_player_from`: normal directed/player/mob/projectile dispatch.
- `Game::explosion_hurt_player`: explosion player dispatch.

With no registered protection provider, each adapter immediately continues the
original native function. Otherwise it executes an attributed copy of that
exact official function. Only receiver/function renaming, a local equipment
capture at the native hit site, and the explicit native-hit handoff differ.
Armor/shield decisions, damage gates, RNG, attacker credit, knockback, outcome
handling and native post-hit tails are preserved in place. Capture is after the
other HEAD hooks and native pre-hit work. There is no native-function reentry in
these two callers. Their RETURN modifiers run once through Mixins' normal HEAD
cancellation return chain; its ordinary validation still applies.

The copies are MIT-licensed official Horizon code. See
`mixins/pluto_native_creative/HORIZON-DAMAGE-LICENSE.txt` and the inline explosion
notice. They increase maintenance cost: every official source change requires
fresh review, regeneration, exact drift checks and host validation. This is a
pinned mod adapter, not a new official Horizon API or a permanent compatibility
claim.

Before applying **all installed bundles together**, run both guards:

```sh
python3 pluto-lib/tools/check_protection_plan.py COMPLETE_PLAN.json
python3 pluto-lib/tools/check_damage_equipment_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE
```

The second guard checks source file/function hashes, native plan fingerprints,
regeneration from the exact original body with only reviewed substitutions,
and the exact helper source embedded in the plan. It rejects every interior
hook on a replaced caller (which would otherwise be skipped), duplicate or
missing replacements, and competing lowest-priority HEAD hooks. Higher-priority
HEAD hooks may run/cancel before the replacement. RETURN hooks must also pass
the official planner's cancellation/capture checks. Never bypass a rejected
combined plan; coordinate a shared adapter instead. The common stage.py
preflight must enforce these checks before application. Applying this bundle
through a raw launcher/CLI route that skips them is unsupported, even when the
isolated source compiled.

## Validation scope

The grouped candidate's official plan resolves six injections, five modules and
zero state declarations. Both source/ordering guards and their twelve focused
Python tests pass. Exact runtime source `2b32def7da6e211ec7581d1ff96d27486bba72bc`
passed official application, the post-apply guards, and
`cargo check --locked --offline -p horizon-client-lib --lib -p horizon-client
--bin horizon-client`. Both native packaging tests pass.

The two native snapshot tests were **not executed**: their test compilation was
interrupted with exit 130 before the shared 10 GiB free-space floor. It reached
horizon-client-lib compilation; no pass/fail result exists for these tests.
No gameplay parity or live provider lifecycle pass is claimed. Machine-readable
evidence is in [the validation record](damage-equipment-validation.json).

This covers these two caller paths only. Fall, fire, drowning, potion and other
environmental routes remain `None` unless separately adapted. It does not
implement persistent equipment-derived state, inventory writeback, shield
feedback callbacks, or recover the old TConstruct state declarations. Full
combined package validation and live host/LAN/prediction scenarios remain
separate integration work.

## Selected main-hand correction

A later source review found that the initial adapter used `Game::held()`, which
means the currently acting hand. During an offhand action that could duplicate
the offhand snapshot and omit the selected main hand. The current adapter reads
`game.stack_in(game.selected)` explicitly. This correction is source-proven
against official Game::held/held_slot/stack_in and has a static regression
contract; the earlier2b32def7 host check predates it. The next grouped host check
and an offhand-action gameplay fixture must cover the corrected adapter.
