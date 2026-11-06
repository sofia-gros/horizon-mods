# Private native opening: excluded source candidate

This follow-up is separate from frozen latest ownership/type candidate `c5e4094b`
and runtime `29f8b52d`. No ordinary manifest, existing caller, packet path, public
opening operation or locale listener changes. The latest typing gate does **not**
include these files. New Rust/Game tests are unrun.

## Exact boundary

Append `native-open-storage.rs` and `native-open-game.rs` to the same private
`game::retained_book` module, after the existing storage/owned/view fragments, in
a future isolated experiment only. No second allocator or state carrier is added.
The generic safe-return `try_install_owned` is unchanged.

The new private installation implementation is generic only so its assignment
mechanics can be exercised without a Game. Its sole intended runtime caller is
the concrete BookScreen operation. It is not an author-facing generic mutation
API. After identity admission succeeds, it revokes opening, resets revision,
performs the same `Option` assignment as native `set`, then publishes the new
opening. This preserves the old concrete value's native destruction position,
rather than using `replace` and delaying destruction until after the input tail.
A destructor panic returns no owner and leaves no observable opening. It does
not promise candidate recovery or rollback after assignment began.

The Game candidate admits only a nonempty, non-lectern read-only View. It does
not establish client/actor/opening permission: its still-unconnected trusted
caller must do that. Candidate pages, style metadata and page index are not
converted or sanitized here. No current-input gate is reused as retained owner
or screen-opening authority.

There are three normal outcomes:

- Rejected before installation: return the exact candidate, with no input/capture
  tail and no slot change. This includes identity exhaustion/unavailability.
- Current: install once, destroy the previous concrete value in native order,
  clear keys/right state and call native `capture(false)` once; return the owner
  only if the attachment/opening still matches afterward.
- Superseded after effects: no owner is returned. Never restore a prior screen,
  take an unrelated current candidate, or retry. A panic during the tail produces
  no normal result and remains an attempted/unknown outcome.

Revision-only navigation does not revoke an opening. Revision exhaustion does,
as the original private storage does. Any later update still needs its own fresh
validation; Current describes only the return instant. All native code remains
trusted, and the model is not a sandbox against arbitrary native mods or unsafe
aliases.

## Source evidence and limits

The exact latest native open/tail/stop/reset regions are already pinned in
`../../docs/guide-owned-open-tail-boundary.json`. Native `open_book` is byte-same
on 624c and f4dc. This candidate copies its three tail statements with only the
receiver name changed. Source guards keep the order and one invocation, but are
not an actual Game execution or whole-plan composition proof. Original native
source is MIT-licensed; see the retained prototype's `HORIZON-LICENSE.txt`.

The latest BookScreen/Text/Style/EditBox declarations inspected for this design
contain owned values rather than stored closure/trait-object callbacks; no custom
Drop implementation appears in those defining files. That is a bounded source
observation, not a complete-crate, foreign-hook, allocator, destructor or panic
proof. The operation therefore keeps native assignment/drop ordering rather than
assuming destruction is inert. `capture(false)` can stop mining, alter tutorial
and remote mining state and manipulate the window. It cannot be replaced by a
modeled no-op and called equivalent.

## Prepared checks, not passing results

The std-only fixture includes the exact original retained/owned source and the
new private storage implementation. Its six new cases cover failed allocation,
old destruction before a modeled tail, panic identity revocation, page-only versus
attachment change, identical reopen/close and cross-slot/revision exhaustion.
The original seven storage tests remain included; a future run must inventory all
13 and select six new tests with 7 filtered. The ten earlier owned-opening tests
remain in their separate unchanged fixture; they are not copied into this one. It does **not** include the Game
adapter, real BookScreen, window, native tail, locale or LAN. No fake BookScreen
stands in for those types.

A later independently reviewed type gate must append these two fragments to a
separate full latest experiment, validate the full direct-site inventory and
all source hashes, and compile the real Game/client library/binary. A later
ordinary integration additionally needs exact caller and whole-plan guard review,
opening permission, post-callback actor/catalog/request validation and actual
Client/prompt/overlay/close/reopen acceptance. Existing LAN remains unchanged.

Five final Python source checks passed on 2026-11-06 with bytecode disabled.
They check exact fragments, assignment/tail spelling, six-test names and absence
from the two ordinary manifests. No Rust, Game, lifetime or LAN pass is claimed.
