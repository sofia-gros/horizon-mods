# Guest-player actual-Game borrow probe: prepared, not executed

**UNAPPLIED / UNFINISHED.** This unit contains exact addition diffs for a future
isolated test-only copy of official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`. No official checkout or runtime
manifest is changed. No overlay was staged and no native compilation or runtime
execution was performed for this unit.

## Why a small stand-alone dependency fixture cannot prove this

The actual `Game` struct is private inside the gameplay crate's `game` module.
An external fixture cannot name it just because `lib.rs` exposes the module.
Copying its selected fields into a fake Game would not prove the native borrow
contract. The proposed probes therefore append a private child module to an
otherwise byte-unchanged **copy** of the real `game.rs`, using all real imported
types and constructor fields. The complete gameplay crate must type-check it.

`prepare_game_borrow.py` only creates/verifies two addition diffs inside this MOD
experiment. It has no stage, apply, Cargo or runtime mode. The positive and
negative patches are alternatives against the same exact source; do not stack
them or install either in the normal bundle.

## Positive compilation expectation

`game_borrow_positive.rs` contains two uncalled, test-only probes:

- `sequential_borrows` makes a short-lived real MotionWorld observation, ends
  that borrow, calls the complete native `Game::add_player_effect`, then makes
  another short-lived real MotionWorld observation. A passing compile check
  would show that this lexical borrow shape is accepted for the real types.
  It would not implement the pending Inside frame, validate its saved geometry
  or establish that the same actor/level remained eligible afterward.
- `observe_guest_teleport` enters the native `as_guest` owner turn, records the
  real current body position and optional native handle, calls the actual
  native same-level teleport path, then records both again. There is no copied
  Game, fabricated EntityRef or fallback owner. This function is not called by
  any production path and has not been executed in a fixture.

The latter is a future observation aid, not a continuation validator. A real
resource-backed fixture must separately establish the selected live guest and
check restoration of the outer owner after the native closure. An absent turn,
body or handle is evidence to report, not a successful test to skip. Native
handle absence does not get repaired by constructing one from matching numbers.

## Negative compilation expectation

`game_borrow_negative.rs` intentionally retains the real MotionWorld field
borrows across `Game::add_player_effect`, then uses `world.player.position`.
The expected rejection is **E0499 at that exact whole-Game call**. This is a
prepared expectation, not an observed compiler diagnostic.

Only run it after the matching positive real-crate check succeeds. A missing
dependency, visibility error, source drift, unrelated native type error or disk
stop is not the expected negative result. Preserve structured diagnostics and
verify that the rejected span belongs to this added probe. A later harness must
verify exact source, dependency/lock identity and before/after overlay seals.
It must not modify the selected official checkout or a frozen integration stage.

## What same-level movement does not prove

The official `EntityRef` stores world-open epoch and entity ID, not a movement
or body-generation stamp. The native `teleport_to_in_level` path changes player
position, velocity, fall state and teleport notes. Merely retaining an equal
handle is therefore not sufficient evidence that the collected Inside snapshot
can be resumed. Even equal positions cannot prove the absence of an intervening
teleport or mutation followed by a return to the same coordinates.

There is also a concrete lookup precondition: `native_entity_target` resolves
server guests through `Game.lan` before interpreting `lan_acting`. The usual
`tick_lan` path takes that host out of Game while it calls `tick_guests`.
The future probe records optional handles without forcing them to exist. A
normal guest turn can have a valid lexical owner while this particular lookup
route has no roster. That distinction must not be hidden by a fake handle or
mistaken for a complete continuation capability.

Consequently this unit introduces no rule such as “same ID/epoch/position means
continue.” Removal, body replacement, teleport, nested guest/level actions,
resource reload and callback errors still require the real frame owner's
specified transition behavior. Neither returning observations nor ending a Rust
borrow supplies that missing behavior.

## Validation status and next gate

Eight source guards pass against 19 pinned official files. They verify exact
addition-only diffs, real private types and constructor fields, positive/negative
source shapes, native guest entry and lookup/movement anchors. They do **not**
prove either compiler outcome or runtime authority.

The coordinator has held stage/copy/check work until the current host gate
finishes. The next permitted step is a separate, sealed real-crate compilation
probe through the established official cache/alias path, if its resource budget
allows. The official native handle tests themselves require prepared gameplay
resources and construct actual Game; no verified complete resource harness has
been supplied for this guest continuation experiment. Runtime movement,
restoration and contact effects remain untested.

Mob detached-roster/navigation-loan behavior is outside these guest-player
probes. No mob path is substituted or claimed by a player result.
