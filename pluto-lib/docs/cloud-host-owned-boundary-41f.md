# Host-owned cloud action: exact missing connection

Source-only proposal for official Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf` and Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`. No native source, installed manifest or runtime is changed. The attached-only candidate 70a589b2 is not adopted for ordinary right-click repair. No new production authority API is added here.

## Available owner and where it is lost

`Game::lan_guest_packet` receives the actual `&mut LanHost`, connection and packet. Its `UseItemOn` branch performs existing acknowledgement, placement, reach, finite-hit, world-height, protection and border work before the item callback. The source must remain authoritative for that ordering.

The branch calls `as_guest(host, conn, |g| ...)`, which delegates to `as_guest_with_host(host, conn, |g, _| f(g))`. The latter already supports an actual immutable host borrow alongside `&mut Game` for the callback. It calls `enter_guest`, enters `block_contact::in_level`, invokes `f(self, host)`, and calls `leave_guest` afterward. This is a real borrow opportunity, not permission to reenter or retain the roster. The callback still must establish Ready server state, actual current level, exact guest/motion/connection/UUID matching and the correct action phase.

The existing callback uses only `g`; it sets `forced_hit` and `guest_use`, then calls `use_item`. The host borrow is absent from each subsequent item method:

- `use_item(&mut self)`, wrapping `use_item_as_player` in `with_player_actor`.
- `use_item_as_player(&mut self)`, selecting the permitted hand and using `with_hand`.
- `use_hand(&mut self, block_hit)`, preserving right-click cancellation, block-first behavior, held item and cooldown checks, native item hooks and earlier item actions.
- `use_tool(&mut self, stack, on_block)`, preserving prior tool/bucket branches.
- `use_block_transformer(&mut self, stack, pos, face)`, the existing TConstruct block cloud hook location.

`with_player_actor` only temporarily sets `sculk.actor = Who::Player`; it neither attaches a roster nor resolves an actor. `native_use_item_on` receives Game/stack/position/face/claimed only and dispatches that same Game. Moving the cloud action to packet HEAD or before ordinary block behavior would change the established consumption/cancellation ordering.

## Why an extra numeric handle is insufficient

`command_guest` reads only `self.lan.as_ref()?.hub`. `native_entity_target` also finds server players through `self.lan`; its fallback `level_entity` does not enumerate Player/Guest. During the detached turn neither existing lookup constitutes authority for that player. `native_level` and Ready epoch can still identify the active level, but level validity is not player authority.

An internal host-owned insertion alternative could validate the real borrowed `LanHost` entry directly, together with the acting Game body, epoch, level and explicit action provenance. It must not manufacture an EntityRef and then treat the existing self.lan-based lookup as passed. It should remain a separate private boundary, leaving the current `insert` rejection behavior unchanged. Its commit must retain all descriptor/attribution/loaded-center/ID-range/capacity checks before take-ID/assign-ID/push, with no fallible operation or rollback promise afterward.

However, creating that function alone does not connect it: no actual host borrow currently reaches the TConstruct caller. The earlier descriptor and charge changes are not evidence that this gap has closed.

## Latest Mixins constraints on the smallest tempting shortcuts

These are static source constraints, not newly executed planner failures:

- Injecting directly into `as_guest` or `as_guest_with_host` is not presently established: they have an explicit generic return type `R`; `validate_target` rejects explicit generic parameters (`E_UNSUPPORTED_SIGNATURE`) before body-specific selection.
- Wrapping `g.use_item()` cannot add the outer host as context under the current `WRAP_OPERATION` contract: the wrapper must bind exactly the receiver/arguments and `$operation`, and extra context is unsupported.
- Wrapping the enclosing `as_guest` call would have to name its closure argument as an explicit concrete input type; current operation contracts reject opaque inputs. A closure type must not be forged as a function pointer or boxed trait object with different lifetime/dispatch behavior.
- A plain helper parameter named `host`, a lexical-capture declaration, or a location allowlist does not supply a borrow missing from the native call. A callback that captures an already mutably borrowed outer host is not an ownership solution.

This does not claim that every possible future Mixins extension is impossible. It establishes that these simple existing-boundary proposals are unsupported by the reviewed source. No broad function replacement has been prepared or authorized as a substitute.

## Smallest honest implementation contract to evaluate next

A real owner-aware item entry needs to start at the existing admitted `UseItemOn` callback using `as_guest_with_host`, retain its borrowed host only for that invocation, and propagate that context through the existing hand/item/tool ordering to the exact consumer phase. Native paths with no such context must remain explicit unsupported/ordinary fallback according to the selected feature policy. No TLS, pointer registry, ID side table, parked queue, dummy-host swap or Unowned fallback can replace that propagation.

The minimal direct native refactor would add an internal optional borrowed action context to the five item stages above (or equivalent explicit owner-aware companion methods), with legacy entry methods delegating to their original no-context path. This is a proposal only. It needs an accepted narrowly scoped source/extension mechanism; copying the full packet/item bodies is not a proven minimal solution. It must not reorder block hooks, spectator/cooldown checks, off-hand selection, advancement/audio/swing acknowledgements, `with_player_actor`, `with_hand`, `block_contact::in_level`, or `leave_guest`/later teleport processing.

Before implementing or enabling that boundary, require: actual native borrow acceptance; exact full-composition plan; owner/phase rejection tests; unchanged unrelated item-path tests; admission failures before cloud consumption; real success under the normal packet route; and callback movement/held-item continuation plus partial-error accounting. Client, local integrated owner, external guests, mobs and commands are separate paths, not interchangeable proof.

Until such a connection is demonstrated, the ordinary cloud feature is blocked. Other TConstruct transformations and effects should not be deleted to conceal this blocker.
