# Existing guest contact borrow phases and the missing continuation contract

**Source-only audit. No runtime change or new authority API.** This follow-up
uses official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and the existing
Pluto snapshot at `8715b1232c0fca133fe012f35830e2108e653d59` (published equivalent
`89744814b8d536a117e82f8ea3c8cdc9798faa95`). It does not run or supersede the
pending Game borrow compilation/guest observation fixtures. No official
checkout is modified and no discarded fork is read.

## Concrete phases already present in the native caller

The line references below are to `game/lan_actions.rs::guest_server_tick` at
the immutable official revision. They describe existing source, not a proposed
patch's borrow-checker or runtime result.

1. **Before the owner turn, 4577–4620.** Replay, vehicle/glide work and movement
   extraction precede the main `as_guest` closure. Spectators take a separate
   effects/shoulder path and return. Several flags and movement values are
   captured before later native work; a new extension must not call these
   universally fresh actor facts.
2. **Inside the current guest, 4620–4673.** Game reads health rules and equipment
   immunity/boat facts, borrows the actual current player, and constructs
   MotionWorld from player/world/assets/collision/motion/health resources.
   `fluid_base_tick` occurs at 4639. The caller then restores movement history
   and computes `tick_movement` and `start` from the received moves.
3. **Motion-owned contact and tail, 4674–4728.** Landing, movement statistics,
   climbability and impulse updates precede `server_block_damage(start)` at
   4714. Soul-speed block update, environment damage, fire contact, hazards and
   pose work follow, still through the same MotionWorld. These calls do not
   lend the surrounding mutable Game to an Inside callback.
4. **Existing whole-Game handoff, 4729–4758.** The last `w` use takes
   `w.player.events`. The caller then uses Game for cauldron updates, rain,
   queued Wither Rose effects, movement statistics, location effects, border
   damage, effect ticking, soul speed and shoulder handling. This is a real
   existing lexical release/reborrow boundary, but it is *after* the entire
   contact sweep and the native environmental tail.
5. **Later player/motion reborrow, 4759–4798.** The caller only checks
   `game.player.as_mut()` for presence, then regenerates/feeds, appends events,
   carries the earlier `tick_movement` and runs `server_travel`. After a Game
   pufferfish contact it again checks player presence, appends events and reads
   marked velocity before combat/armor handling. These checks are not a
   same-body/resource generation test for an interrupted Inside frame.
6. **After the owner wrapper, 4800 onward.** The `as_guest` result is processed,
   and jump/sculk/event notifications may use additional guest turns. The
   existing wrapper also restores the guest and flushes teleport notes before
   it returns. An observation from inside the earlier closure is not the final
   outer owner state.

The native calls intentionally use `let _ =` for the block/environment phases.
`server_block_damage` itself uses `?` for `effects_from_blocks`, so an inner
error skips its freezing tail, while the caller still proceeds to later phases.
A frame refactor must retain both levels of control flow. Converting every
failure into a whole-tick abort, retrying contacts or treating an error as a
successful completion would change behavior.

## Why the existing Wither Rose path is not an immediate-effect precedent

In `motion.rs:1820–1838`, accepted Inside contact is dispatched after visited
insertion and before the native action. The Wither Rose action only appends
`MotionEvent::InsideWitherRose`. `guest_server_tick:4739–4742` handles that event
later, after the sweep/environment/pose work. `creatures.rs:3854–3859` then
checks remote/difficulty and gives the player Wither.

That is a genuine native route with Game available, but choosing it for custom
fluid effects would choose deferred timing. It cannot be called a mechanical
implementation of TConstruct's immediate `entityInside` effect. A future API
could label such a phase explicitly for a consumer that wants it; this audit
does not add one or activate TConstruct through it.

## Read-only immersion can be separated from mutation, with limits

The existing `pluto_world_fluid_query::View` needs immutable World,
BlockRegistry and Behaviours from the same host context. It already owns the
registered-cell decoder, loaded-before-read behavior, query budgets and explicit
unloaded/error results. `query_heights` is not Forge's cached fluid-type height.

A prospective *upper Game caller* can prepare explicit adjusted body bounds
and use short shared reborrows of those three inputs without inventing another
fluid registry or storing an actor. The candidate timing to evaluate is around
the caller's existing fluid-base observation, before contact traversal. This
has not been compiled or shown equivalent to source fluid-height timing.

The adapter resides in the Game-side module and uses the higher `pluto-api`
crate. It must not simply be called from lower client-lib MotionWorld by adding
an unsupported reverse dependency. `MotionWorld.assets` exposes the registry,
but the same-context Behaviours input also needs an explicit upper-owner borrow;
it is not a field of Assets. The public existing Game `query_point/query_box`
wrappers also borrow Game as a whole, so calling them while `w` holds mutable
Game fields is not the proven implementation of this proposed observation.

Even a successful observation would provide owned measurements only. It would
not authorize effect writes, prove an actor survived a callback, convert an
unloaded result to dry, or permit resampling after movement while claiming it
was the earlier source snapshot. No new wrapper or runtime hook is added here.

## A narrower vanilla-effect route is a hypothesis, not an authority bypass

Official `add_player_effect` adds/merges, applies derived status, handles
absorption, then calls `mod_effect_added`. The latter dispatches native content
callbacks only for a non-remote Game and an ID satisfying `is_mod_effect`.
`is_mod_effect` is a namespaced-ID syntax predicate, not a registry existence
test. Built-in Poison/Slowness/Blindness IDs do not take those content callbacks.

This can narrow a future call-graph study; it does not make the whole operation
callback-free or prove a suspended contact frame can resume. Derived status
updates other native attributes/cache/health state, and other installed Mixins
can change the involved functions. A supported narrow operation would need its
actual complete-plan/source closure, body/resource invariants and runtime
rejection cases established. This audit supplies none of those capabilities.
It does not skip status updates or replace `add_player_effect` with list-only
mutation to avoid the missing contract.

The source's protected empty curative list is also still unrepresented by the
installed native instance. Selecting only these three built-in effects cannot
make the curative/copy/hidden/save gate disappear.

## Production gates remaining

- A real owner implementation must suspend the native per-contact frame while
  retaining its exact visited/hit/error/budget/freeze ordering. A data callback
  or post-motion event collector is not that implementation.
- After any effect operation, the owner must establish which body, level and
  resource context may resume. Mere presence, equal entity/epoch/dimension or
  a second teleport to original coordinates is insufficient. No general
  `can_resume` predicate is supplied.
- Player/gameplay effect representation and its per-instance copy/save/cure
  contract remain separate from the owned frame. Mob contact additionally
  owns a detached roster/rider/navigation path; these guest phases do not
  establish mob authority.
- Actual host borrow acceptance, no-callback trace equivalence, supported
  transitions and resource-backed local/LAN tests are still unrun. The previous
  library type proof and source checks cannot be relabelled as those results.

The companion JSON records the exact seven official sources and two existing
Pluto adapter blobs. The small source regression checks only the stated order
and representation anchors. Its corrected source-only run passed eight checks.
The first run passed seven and failed one because the test matched an earlier
spectator callback instead of the main owner closure; the recorded correction
only narrows that test's source slice. No runtime code was changed.
It performs no native compile, apply, stage copy,
Game execution or CI dispatch. This remains reusable-library design evidence,
not a completed TConstruct fluid-contact implementation.
