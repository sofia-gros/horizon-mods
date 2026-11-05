# Source use-item movement speed

This addresses the high-priority #379 Unburdened use-item slowdown item. It does
not implement the same row's Padded, offhand attack or Bonking behaviors.

Pinned source: TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`ToolClientEvents.handleInput`, `ToolStats.USE_ITEM_SPEED`, `AttributeModule`,
`MaxArmorAttributeModule` and `MaxArmorLevelModule`.

The authoritative local player's input-body refresh projects the source
movement speed into the native `item_use_speed` field while using a stock item
with the ordinary 20% slowdown:

- Held tool default 0.2, source tool base-stat additions and multipliers, and
  Unburdened's additive 0.1 per trait level. Tool stat is clamped to 0–1.
- Worn Unburdened adds 0.1 per level. Cobalamin adds 0.1 per level from its eligible
  slots. Featherweight adds 0.05 per level on tagged armor, including held shields.
- Melee Protection adds 0.05 times the greatest eligible effective level, not the sum
  across pieces. Its material traits combine with acquired five-unit progress,
  including fractional levels. Broken equipment and armor in the wrong class slot do not
  contribute attributes. Held-tool stat evaluation follows the source input
  hook, which does not separately filter brokenness.
- The combined source speed is clamped to 0–1. The native movement pipeline still
  performs vector/sneak handling. Its floating-point grouping is not claimed
  bit-identical to Forge's post-input multiplication.

Both hands use the actual active-use slot. Every native body sync first resets
its field; stopping use, removing equipment or invalidating resource definitions
cannot leave a persistent movement bonus. No extra saved state is introduced.
Current source definitions, traits and tags are re-read. Unknown stat modules,
changed modifier definitions, malformed traits and manually stored unsupported
incremental-upgrade counters for the other three modifiers fail closed.
Melee Protection uses its explicit five-unit defense acquisition contract. A custom modifier declaring another
use-item-speed effect is not silently ignored.

## Bounds and validation

This is a stock-source trait/runtime port, not new Station acquisition for these
modifiers. Nonstandard native use-speed components, passengers, remote clients,
external custom attribute commands and custom module implementations retain the
native path. Real LAN/prediction behavior remains unimplemented/unverified by this
local-player bridge. Broader movement, tool and armor completion is not claimed.

Tests compare all four source definitions and tool override examples, exercise
source arithmetic, and compile the unchanged runtime/wrapper with real host
ItemStack/NBT and a narrow Game/trait adapter. They cover offhand, ordinary bows
with worn effects, broken/wrong-slot equipment, unsupported state, reload and
stop-use cleanup. The adapter does not prove the actual material resolver,
render/input GUI, save/network round trips or gameplay.

Run `python3 tconstruct/tests/run_use_speed.py --horizon PATH`. The runner requires
an exact clean pinned host checkout, rejects fetched dependency drift against its
lock, uses locked/offline Cargo, and requires a named production-adapter pass.
CI separately compiles the complete transformed client and asserts named source
tests executed. Actual client/gameplay verification remains a separate gate.

## Native compatibility boundary

The pinned Mixins planner rejects RETURN and local STORE hooks in
`sync_player_body` because its closure contains `?`. A cancellable HEAD is used
only when supported source data changes the native speed. That path reproduces
the pinned native sync body, changing just the speed assignment and return form.
The runner compares its tokens to the exact clean pinned source before testing.
All unrelated body fields and the sleeping callback are exercised by the adapter.
No-effect and unsupported cases continue the original function.

This path bypasses other injections inside the original body when it cancels.
There are no other such injections in the pinned TConstruct bundle, and a test
rejects adding one unnoticed. Compatibility with external mods injecting into
that body is not established. A future narrow native post-sync hook should
replace this compatibility copy rather than extending it to unrelated features.

## Melee Protection acquisition integration

The prior resolver rejected any purchased modifier value, including Melee
Protection. It did not apply a five-times bonus. This follow-on admits only
Melee Protection's newly defined acquired progress: five stored units are one
level, with the source f32 partial-level correction. Other three traits retain
their integer arithmetic and purchased-modifier rejection. Damage protection
is still not implemented by this movement bridge.

This fix is based on the existing use-item-speed branch and is independent of
the defense-acquisition branch. Integrate both to enable the complete normal
acquisition-to-use-speed path. No new movement injection or compatibility-body
copy was added. The previously documented remote/LAN and native-sync limitations
remain.

Brief static review covered all level consumers, maximum aggregation and the
existing fixture's stored-data key. Source and runtime-adapter regressions were
updated but not run for this follow-on. No tests, builds, Mixins transforms, CI
inspection, version checks or merge were performed. Publication is paused.
