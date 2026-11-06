# Authoritative armor-wear observations

This development slice belongs to the grouped PlutoLib defense/equipment API.
It adds a host-independent read-only event at the official player's native
armor-wear consumption boundary. It does not migrate TConstruct armor policy,
calculate modifier effects, change native wear, or authorize equipment writes.

## Public contract

Use the single `horizon_client::pluto_inventory::armor_wear` API instance.
`register("example:observer", callback)` accepts a function pointer; duplicate
IDs fail without replacing another observer. `remove` affects subsequent
dispatch snapshots. IDs determine lexical callback order, and the registry lock
is released before callbacks. Registry poisoning suppresses dispatch; callback
panics propagate after native work and are not treated as rollback.

`Request` borrows these facts only during synchronous dispatch:

- `actor`: opaque current entity number, dimension and creative/spectator flags.
- `source_kind`: the damage catalog name at consumption, or `None` for an unknown
  native ID. The original event contains no attacker, original damage outcome,
  original equipment, or historical tag snapshot.
- `requested_damage`: the exact raw request, not item wear or a health delta.
  The native adapter does not observe requests satisfying `damage <= 0.0`.
  Positive and NaN requests retain their exact bits; no normalization is added.
- `before` and `after`: the shared `equipment::Equipment` semantic views, captured
  immediately before the native armor body and after its break/refresh tail.

The shared equipment DTO and component converter are reused. Main hand means
selected inventory slot even while the offhand acts. Empty and unavailable
slots remain distinct. Opaque retained component backing, including nested
stacks, makes component facts unavailable rather than empty. These views are
not native wire identity, a per-piece loss result, a commit receipt, or proof
that only wear changed equipment. Other native callbacks may have run between
captures. No callback receives `Game`, native stacks, mutation or cancellation.

Observers present at dispatch are called from one copied callback list. Changes
to registration during a callback affect later dispatches. Registration during
native wear can therefore affect the later observer list if observation was
already activated at entry; no-observer entry never creates a snapshot.

## Native phase and authority

The adapter targets `horizon_client::game::Game<'_>::hurt_armor` on official
Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`, with official Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02` as the coordinated source cutoff.

`HealthEvent::ArmorHurt { source, amount }` is queued at the native armor phase
and consumed by `Game::wear_armor`. The adapter does not second-guess whether a
hit ultimately reduced health. It preserves the exact native body: fire and
component checks, per-piece native wear callbacks/RNG, loss application,
broken-piece deletion, destroy-item feedback, and equipment refresh. Before
capture, the adapter repeats the same side-effect-free `damage <= 0.0` gate and
continues into the original method for those inputs. The unchanged native body
keeps its own gate. NaN does not satisfy that comparison and is not newly rejected.

A lowest-priority cancellable HEAD captures facts after this gate, calls an exact renamed copy
of the body once, then dispatches observation. This is after native break tails
but **before target RETURN handlers**. Higher-priority HEAD handlers run before
capture; a higher HEAD cancellation bypasses observation. No recursive target
call or ambient actor/equipment state is used.

Without observers or an authoritative active player, the HEAD continues into
the original native method directly. Remote/client-side and replay players are
excluded from observations. The local observation checks the same entity,
dimension, world root, guest actor/UUID/entity fields and authority flags before
and after native work. A change suppresses the observation without undoing or
repeating native work. Those comparisons are not a world generation token and
cannot distinguish replacing a world/actor with the same complete identity
tuple. They must not be used as persistent or transaction identity.

The source visibly calls `wear_armor` while guest state is active in
`lan_actions.rs`, before restoring the host. The adapter follows whichever
active authoritative actor invokes the native target. Live local-player, LAN,
world-swap and replay behavior still need runtime validation. This is not a
general event for all item-durability changes or every equipment mutation.

## Source and composition guard

The copied body carries the full official MIT notice. `armor-wear-source.json`
records the official file/function hashes, parser function fingerprint,
regeneration header/footer and **no interior body edits**.
`check_damage_equipment_plan.py` now requires this third caller along with the
ordinary/explosion callers. It verifies exact source, reviewed generated helper
bytes and the complete plan; it rejects bypassed interior hooks or a competing
lowest-priority HEAD. Allowed RETURN hooks run once after observation through
native cancellation handling. Matching a function name alone is insufficient.

The mandatory Pluto-owned preflight must run on the complete combined plan
before application with matching repository metadata and packaged helper bytes. Raw CLI application that
bypasses the guard is unsupported. Updating official Horizon requires reviewing
and regenerating this copied body and revalidating the adapter, even when its
public signature stays unchanged. This is deliberate source-maintenance cost,
not a permanent native extension point.

## Evidence and pending validation

Local lightweight checks passed four armor observer registry/API tests, six
armor source/composition tests and seven existing caller guard tests. The three
helper bodies regenerate byte-for-byte from the pristine official export and
reviewed transforms. This last check used synthetic guard inputs, **not an
actual new Mixins plan**. The new function fingerprint was computed with the
same `syn` token serialization and SHA256 procedure, cross-checked against both
previously verified caller fingerprints.

There is no plan/apply/native host check or live-game claim for this slice yet.
The next grouped cutoff must include the shared block/equipment facade
`5785d0c3` and recursive retained-wire query `29935573` (central `4ac667fa`), then
run complete preflight, apply and bounded host checks. Earlier defense builds
predate this new adapter and are not evidence for it. Consumer recovery remains
separate.
