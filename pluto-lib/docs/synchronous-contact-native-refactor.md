# Synchronous contact: proposed native refactor, not an enabled extension

**UNAPPLIED / UNIMPLEMENTED owner integration.** The design and source-order
checks below target official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`.
No official checkout, existing runtime implementation or manifest is changed.
There is deliberately no new generic authority API: an abstract callback driver
would not supply the missing real Game borrow or live mob owner.

## Proposed first scope

The smallest explicit owner to target is the **server's guest-player** path
`Game::guest_server_tick -> as_guest -> MotionWorld::server_block_damage`.
The guest's effects/body are already swapped into Game inside that lexical turn.
This scope does not automatically include local motion, passenger prediction,
replay, mobs or items. Those constructors and lifetimes differ.

The proposed split point is after the original read-only `Inside` dispatch and
before the current native per-block action. Keep that original dispatch intact.
A future Game-side effectful callback would run at the new split point, while no
MotionWorld, PlayerMotion, World or registry-definition borrow remains alive.
It must finish synchronously before the pending native action is resumed.
Returning a batch to apply after all blocks or after the tick is not equivalent.

This is a native source refactor requirement, not a field insertion or a Mixins
HEAD hook around the whole traversal. No refactor patch is presented as
compilable/applied, because the owner and invalidation contract below remains
unsupplied.

## Exact player frame decomposition required

These are proposed **private owned frames**, not public actor capabilities.
Their names describe required responsibilities, not existing native types.

1. `ServerBlockDamageFrame` owns the original `from`, the current phase and the
   original result/error path. Initialization must set `old_position`, copy/reset
   powder-snow flags once, then run block effects. Thawing and periodic freeze
   damage run only after that block-effects phase returns successfully, exactly
   as they do now.
2. `BlockEffectsFrame` owns the drained movement history and any final movement
   appended by the original distance check. Clearing history precedes spectator
   exclusion. It retains `damage_only`, one visited set, the entered powder-snow
   sequence, movement index, axis order/cursor, current `from`, the per-movement
   budget of 16 and the one-iteration endpoint fallback. `StepOn` and its current
   native actions stay before the first Inside suspension. Powder-snow grouping
   and freeze application stay after all movement segments; no grouping per
   yielded contact is allowed.
3. `InsideFrame` owns the current `from/to`, deflated end box, `moved_far`,
   max-iteration bound, collected `(iteration, position, state)` hits, cursor
   and native `iterations + 1` result. The existing sweep's `alive` observation
   and state snapshot are not recomputed after each callback. Registry/shape
   resolution and the special collision checks still occur at their current
   per-hit point; do not precompute them all during the sweep.
4. A pending Inside phase contains the accepted contact and the **unexecuted
   native tail**. The visited insertion and original Inside dispatch have already
   happened. After the owner callback, resume that tail exactly once; never
   redispatch/reinsert the same contact. The `damage_only` continue advances to
   the next hit, not out of the whole traversal. The non-damage tail, including
   `EnterBlock`, remains ordered exactly as now.

No frame may retain a borrowed definition name or a reference into Game. Any
owned descriptor is merely contact data. A copied numeric entity/state ID is
not proof that the same body, level or resource owner still exists.

## Game driver boundary and the missing validity rule

Inside the existing `as_guest` closure, split the long-lived MotionWorld borrow
around each pending contact:

- Construct a short-lived MotionWorld from the actual current Game fields.
- Advance only until the next pending contact or native completion, then end
  that borrow completely.
- Invoke the complete native Game effect operation in the same live guest turn.
  `add_player_effect` includes collection update, effect-derived status,
  absorption handling and mutable Game callbacks, in that order.
- Validate the real continuation owner and reconstruct the short-lived view
  before executing the native tail. Keep later soul-speed, environment damage,
  fire contact, hazards and pose updates after the complete server-block phase.

**This is not yet an implementable authority promise.** The full Game callbacks
can change the world, actor/body, active guest, resource pack or lifecycle.
Epoch/ID equality alone does not establish the complete continuation contract.
The native owner must supply an explicit, unforgeable binding to the current
body/level/resources and define how that binding is validated after callbacks.

The design intentionally does not choose a silent default for teleport, removal,
respawn, reentrant guest/level changes, resource reload or callback errors. It
must not guess whether to continue with captured states, restart the sweep or
abort the rest of the tick. An unsupported path must be excluded before enabling
this callback; a side effect cannot be rolled back merely because validation
fails afterward. These transition rules need a coherent owner implementation
and actual-host tests before a mutation hook is enabled.

## Why mob contact requires a separate larger refactor

The official mob path differs in all of these source-observable ways:

- `ai::effects_from_blocks` uses interpolation steps from 0 through a clamped
  `ceil(longest/0.2)` count, then X/Y/Z loops over deflated boxes. It is not the
  player's sweep/shape routine and does not share its air/portal exclusions.
- Mob `Inside` emits **before** `visited.push`; player Inside emits **after**
  `visited.insert`. Mob native actions re-read `w.block_name` after dispatch;
  player actions use a state/name from the collected hit. A common normalized
  contact queue changes observable ordering/read semantics.
- `Game` takes `self.mobs` into a local vector before simulation. While the
  current mob ticks, `Game.mobs` is not that live roster. Looking up its numeric
  ID in Game can fail or inspect newly spawned entries instead.
- `MobWorld` borrows immutable Game resources and can contain `Some(self)` as
  `&dyn ModEffectTicks`. Lending mutable Game at the same time is not legal.
- `riders::tick` may hold split mutable rider/vehicle references and loan a
  navigation object across `rider.tick`. Suspending only the inner block loop
  does not release those outer references or restore the navigation loan.
- Mob effects are `(id, duration, amplifier)` tuples. `hidden_effects` means
  hidden particles, not the player's boxed hidden-instance chain. Mob immunity,
  flag updates, duration replacement and absorption also use different methods.

A mob implementation would need an owned frame for the interpolation/XYZ
cursor and visited list, **plus** the enclosing AI tick continuation, rider
navigation loan, local roster, snapshot indices, pushes/random/events and Game
roster restoration. New mobs are appended ahead of the old roster by the
existing outer owner. Temporarily shuffling that roster for a callback cannot
be assumed to preserve indices, removals, passenger relationships or spawn order.

Therefore the proposed first effectful extension excludes mobs. A separate
native roster/owner redesign must be reviewed before providing a mob callback;
player-frame types or the cloud-time Game lookup are not substitutes. This does
not claim TConstruct mob-contact parity.

## Validation gates before any later activation

1. With no new callback, an actual native refactor must produce the same contact,
   native-action, error, visited, budget and freeze-group traces as the original
   for local/guest scenarios in scope. Current source checks only identify those
   invariants; they do not prove the refactor.
2. Compile the real Game/MotionWorld ownership split, without mock owner types,
   unsafe reborrows or a pointer/ID side table.
3. Exercise same-call effect/status/absorption callbacks and all supported owner
   transitions. Prove no continuation or authority escapes to later work and
   that guest restoration remains correct on each supported exit.
4. Prove remote/replay/prediction/collision-only paths cannot invoke the mutation
   extension. Add local and mob support only after their separate frame/owner
   proofs, not from the presence of the same entity ID or a dimension string.
5. Only then connect reusable PlutoLib services. TConstruct-specific effect
   selection, timing policy and curing remain consumer-owned.

`test_contact_order.py` pins 13 official files and checks ten groups of exact
source-order/representation constraints. It is a read-only source regression
suite, **not** a native execution, borrow-checking, callback or gameplay test.
