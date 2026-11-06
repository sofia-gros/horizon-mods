# Guest contact continuation: bounded observation fixture, still unapplied

**UNAPPLIED / UNEXECUTED.** This is a next-step fixture helper and exact-source
contract inventory. It is not in the two-module Game borrow compilation probe,
any installed manifest or production callback. No Game has been constructed or
moved by this unit. Mob continuation is not implemented.

## Newly checked native boundaries

The selected official revision remains
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`.

- `game/lan_actions.rs::as_guest_with_host` enters a real roster-backed owner
  turn, invokes the closure, then explicitly calls `leave_guest`. A normal
  closure return containing `Err` still reaches leave. A panic that unwinds
  across the closure does not execute that explicit call. `block_contact::in_level`
  restores its thread-local dimension with Drop, but that does not restore the
  whole guest/body swap. Do not catch a panic and claim the same Game is safely
  reusable from this wrapper alone.
- `leave_guest` restores acting fields, swaps the outer player back and publishes
  the guest body into the hub. It then calls `flush_teleport_notes`. That function
  drains pending notes and invokes `native_player_teleported`, entering another
  `as_guest` turn when needed. The return from the inner action is therefore not
  the end of notifications or possible reentrant mutations. Any observation of
  restored outer state must be made after the entire wrapper returns.
- Same-level `teleport_to_in_level(Target::Player, ...)` stops riding, cancels
  pending trips for the acting guest, changes position, zeros velocity and fall
  distance, clears on-ground, changes previous position and queues notifications.
  A second teleport back to the original coordinates is not rollback.
- World-open epoch is present during Opening and Closing as well as Ready. It
  is neither an admissible action phase nor a body/movement generation.
  `run_level_teleports` can call `new_body(true)` before changing the hub's level;
  equal player identity cannot certify that a pending motion frame owns the same
  body or geometry.
- The guest server tick already ends a MotionWorld borrow before deferred native
  effect calls and later reborrows the current player. That proves the existing
  caller has lexical places to own Game, not that an Inside traversal may be
  interrupted and resumed there. It still carries previously computed movement
  through its later operations. Reordering contact callbacks to that later
  point would change the requested synchronous semantics.

## Smallest proposed runtime observation

`guest_continuation_observer.rs` is a private, uncalled fixture module for an
isolated copy of **applied v15**, not pristine Game. It uses the already type-checked
Pluto `native_lifecycle::pluto_lifecycle_readiness::ready_server_epoch` adapter to
reject non-Ready or unsupported authority roles without reading private fields
from outside their owning module. This observation depends on that applied-v15
component; it is not a claim that pristine upstream exposes the adapter.

A future real-resource harness must supply a genuine Game, detached LanHost and
one connected, placed, loaded, living, unmounted guest with an existing body in
the current dimension. The helper does not create or repair that setup. It checks
those narrow fixture preconditions before entering the existing owner wrapper.
Inside `as_guest_with_host`, the actual host roster remains available read-only
and is compared with the body and acting identity exposed by native entry. No
fabricated EntityRef, pointer table or fallback numeric authority is used.

It records, without deciding whether to continue:

1. Outer acting/body facts before entry.
2. Guest body, Ready epoch, optional native handle and pending-note count before
   native teleport, after a distinct finite destination, and after a second
   native teleport to the original coordinates.
3. Outer facts and the guest roster position after the complete native wrapper,
   including leave and notification flush, returns.

Missing entry/body/handle or changed outer facts remain observations or explicit
failures. None may be treated as a skipped success. Before the second explicit teleport it rechecks the observed acting identity,
dimension, Ready epoch, body ID and expected destination. This confines that
fixture operation; it cannot detect an equal-ID body replacement and is not a
contact continuation rule. The returned `OwnerFacts`
are a selected observation set, not a complete snapshot or proof that all owner
state was restored. The helper supplies no `can_resume` predicate. It does not
catch panics or assert rollback after a partially performed native operation.

## Required cases and honest acceptance levels

- **Prepared valid same-level round trip:** verify native entry and that each
  intermediate position is the requested one, while final coordinates may equal
  the start. Inspect velocity, fall and ground changes and notification effects.
  Equality of coordinates or handles must not produce a continuation decision.
- **Normal closure result versus wrapper completion:** observations after the
  wrapper must include any changes from teleport notification callbacks. The
  outer observation is not taken from inside the guest closure.
- **Absent/ambiguous guest, wrong dimension, missing body, mounted/dead/unloaded
  guest, remote role, Opening/Closing or any existing outer guest turn:** reject this
  bounded fixture case rather than quietly manufacture readiness. This is test
  setup selection, not a general production authority policy.
- **Handle unavailable in detached-host path:** retain None. The actual native
  handle resolver depends on `Game.lan`; its absence does not justify making a
  handle from the roster ID or disabling the identity assertion.
- **Teleport followed by notification-induced mutation:** compare actual final
  observations after the flush. A future callback fixture must use genuine
  registered callbacks and record their order; this unit does not replace them
  with a modeled state machine.
- **Panic/error and body/level replacement:** normal Result-return can be observed
  with the native wrapper. Panic needs an isolated disposable native test process;
  body replacement and cross-level moves need separately owned setup and explicit
  expected native semantics. None are enabled by the same-level helper.

Ten source guards pass against 21 immutable official files plus the exact
applied-v15 readiness module. They pin native
ordering, epoch semantics, teleport effects, cross-level replacement and the
prepared helper's actual-type/no-fallback shape. They are source checks, not
execution of these cases. The helper itself has not been compiled or staged.

## What still blocks synchronous contact activation

The current native static contact callback carries contact data and mutable fall
amount, not the whole-Game owner required by effect application and its hooks.
The proposed frame split still needs an actual owner implementation and a rule
for a changed body/level/resources after an effect callback. There is no verified
body-generation stamp to make an old frame valid. Stopping the remainder, replaying
hits, resampling at a new position or continuing stale geometry are different
behavior choices; this fixture does not silently select one.

The immediate native gate remains the frozen positive/negative Game borrow check.
Only after it and a resource-backed observation pass can a concrete caller-owned
continuation implementation be evaluated. Mob AI uses a detached live roster and
rider/navigation loans; neither this guest fixture nor a successful player compile
check supplies that separate ownership contract.
