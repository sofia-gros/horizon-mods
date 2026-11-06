# Local station transaction delegation, excluded candidate

Source only, based on integration 17f44dc36fc2df28547328aa8554286ae965b208,
Horizon 41f822b659789780f58d32c63ddfb6887ea1b8bf and Mixins 5a64af93.
The accepted full plan and exact target fingerprints are recorded in inputs.json.
That plan accepted the old hooks; it does not accept or type-check this candidate.
No ordinary manifest, old hook, LAN handler or registry has been changed.

## A separate authority

The host-independent placement API remains veto-only. The proposed native
`game::recipe_book::pluto_transactions` module accepts a synchronous handler with
the actual mutable Game, BookType, entry index and use_max argument. A matching
handler owns the local transaction, not merely permission for native placement.
This is an internal native capability, not a new independent crate or a promise
that a semantic ItemFacts value can commit inventory.

The proposed sole placement HEAD calls the unchanged veto/preview attempt first.
Denied means no transaction selectors or handler. Continue snapshots the new
transaction registry and evaluates read-only selectors in ID order, outside its
lock. No matching handler continues native. Multiple matches stop before a
transaction. Exactly one match invokes its handler once and always stops native,
including if that handler returned Continue. Neither callback effects nor panics
are rolled back by this dispatcher. Duplicate IDs do not replace registrations.

The reviewed TConstruct selector uses exactly `position(game).is_some()`. It must
not additionally require a currently valid crafting-station block or entity:
the old body rejects stale sessions, whereas filtering them would enable native
fallback. The complete old function signature and body are appended verbatim to
consumer-prefix.rs. Position is rechecked by that body. With the sole actual
selector there is no intervening Game mutation. Future selectors have an explicit
read-only obligation that a Rust shared reference cannot fully enforce.

## Registration before publication

`compose.py` adds one line to the existing station `open` function. It is after
the unchanged remote rejection and LAN-acting branch, but before entity insertion,
grid/side changes, session insertion and screen publication. The existing
ensure_datapacks call remains earlier. Registration is a static function-pointer
entry, not actor/world state; it invokes no selector, handler or Recipes method.
The consumer OnceLock caches Result. Duplicate/invalid registration is an explicit
panic before local session creation, not a silent fallback or a returned false.
This does not undo preceding ensure_datapacks effects. An unexpected panic inside
the initializer leaves OnceLock unset, as in std; arbitrary callback rollback is
not claimed. Normal registration contains no recursive call to open or ensure.

All module/helper sources in the actual plan were scanned for the exact generated
crafting_sessions mutable accessor. Only this open insertion and close removal
occurred; exact occurrences are recorded. This is an audit of this source closure,
not proof against aliases, unsafe access, later mods or future lifecycle changes.
An eventual complete-plan/source guard must pin that closure and registration
before publication. Hot installation into an already-running unregistered session
is outside this startup-only migration.

The independent constructor bootstrap for stored denials and pure Overslime
derivations remains unresolved separately. Registering this Game handler from the
lower client-library Recipes bootstrap would reverse the dependency direction,
so this proposal does not do that. No combined-registry rollback is claimed.

## The old transaction is not normalized

The verbatim body retains, in order:

1. Exact local session lookup; stale block/entity and bad book-entry rejection.
2. Persistent grid refresh, creative snapshot, menu_session invocation.
3. Exact nine-grid mapping and 46-slot menu check before item movement.
4. Native recipe placement eligibility, before/after full-stack snapshots,
   menu.place_recipe, no-drop and same-item/components count conservation checks.
5. Only the successful transaction's resulting grid, then station sync, optional
   ghost, and recipe_book_changed notification.

There is no serialization/reconstruction, new component equality, Wire approval,
new clone, consumption algorithm or change to native menu commit behavior in that
body. Its existing guarantees and limits remain its own. The common dispatcher
does not claim all-field rollback or a new inventory transaction proof.

The separate `crafting-station-lan-recipe-guard` is an immutable oracle here and is
not a composed output. LAN acting and remote branches remain outside registration
and the local position predicate. Unrelated local placements and remote packet
sends remain native when no transaction claims them, subject to the existing veto.
The actual Game::place_recipe body is pinned but not copied into the new handler.

## Composition still required

After actual plan/type/behavior verification, the intended ordinary change is:

- Add crate-visible Pluto transaction and TConstruct consumer modules under
  game::recipe_book. The consumer ensure function is crate-visible so the sibling
  station module can invoke it; the legacy handler itself remains private.
- Apply the one-line station registration change and replace the existing Pluto
  click helper with single-head.rs, adding the actual use_max formal binding.
- Remove only the now-delegated local station hook after proof, retaining the
  separate LAN handler. Keep the exact same legacy body through the consumer.
- Extend the mandatory placement composition guard to pin dispatcher, consumer,
  lifecycle closure and sole HEAD; do not simply relax exclusive-target checks.

The dispatcher prohibits handler reentry into Game::place_recipe by contract.
The actual legacy body only calls menu.place_recipe and has no such Game reentry.
There is deliberately no new TLS or actor-state guard. Unknown reentrant handlers
are not proved safe by this candidate; scope the complete-plan acceptance to the
reviewed closure rather than advertising generic arbitrary handler safety.

Current verification: 13 pure source/inverse checks PASS and the official whole
recipe_book/body bytes match. These are textual/composition checks, not runtime
registry or transaction tests. Rust, Game typing, registration startup, actual
plan/apply, two-registry composition, local/LAN execution and full guard resolution
remain UNRUN. No workflow or trigger is included.
