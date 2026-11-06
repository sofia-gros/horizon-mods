# Legacy save and synchronous contact: minimum migration design

**UNAPPLIED / UNFINISHED.** This proposal targets official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`. It changes no official source,
production save format, actor state, callback registration or TConstruct policy.
The new inspection fixture is not an installed migration.

## What the native owners actually do

All references below are pinned by `experiments/effect-instance-proposal/prepare_legacy.py`.

- `save.rs: Level::read_player` normalizes `active_effects` into five-field
  tuples. The selected expression accepts every native numeric Tag type, uses
  signed Byte as unsigned for amplifier only, defaults duration/amplifier/ambient
  to zero and particles to true, and omits entries without a string ID. The
  native text helper replaces invalid UTF-16. Non-List roots become empty.
- `effects.rs: PlayerEffects::from_saved` is a later, different boundary: it
  resolves registered IDs, skips missing definitions, constructs/clamps native
  instances and calls `add` in input order. Repeated IDs can merge. A tuple list
  is therefore not a list of independently restored instance identities.
- Local `Game::sync_level` overwrites `level.active_effects` from live effects;
  `restore_effects` rebuilds `PotionState` and resolves those tuples. Guest
  `playerdata::apply/store` use the same tuple pair, not an alternative lossless
  persistence path.
- `playerdata::load_owner` prefers the named single-player file already loaded
  in a `Level`, then the guest's own file, then the opening fallback. Migration
  must keep that precedence; choosing a convenient stale raw file is unsafe.
- `playerdata::save_owner` stores state, writes the named owner file, logs a
  write failure, then independently writes the UUID guest file. This is not a
  multi-file transaction. A successful second write cannot prove both copies
  were replaced.
- Guest autosaves own an immutable `Snapshot { uuid, name, file, tag }` before
  enqueueing to one FIFO writer. `save_after` drains prior work before logout.
  Proposed metadata must be captured in the same snapshot, not read from a live
  instance after it has changed or from another guest after a swap.
- `write_dat` encodes/compresses before writing `<file>_new`, renames the old
  destination to its backup, then renames the new file into place. The second
  rename can fail after the first succeeded. Native guest load has a `.dat_old`
  fallback. This proposal does not prove filesystem crash durability or atomic
  replacement of several files.

## Smallest safe present step: inspect without adopting or dropping data

`legacy_view.rs` borrows the original parsed Tag and returns the exact native
five-field projection, while retaining entries that the native filter would
omit. It records malformed roots, lossy IDs, hidden records, independent icon
fields, unknown fields and amplifiers outside the proposed record's range.
Bounds on entries, fields and ID units are checked before conversion/allocation.
Unknown subtrees are only borrowed, not recursively cloned or interpreted.
Normal native parser limits are still required for the original load.

Seven prepared Rust tests compare the projection with the **exact extracted
native expression**, using complete unchanged official NBT and `nbt_value`
modules. They cover all numeric types/defaults, unsigned amplifiers, omissions,
invalid UTF-16, order/duplicates/unresolved IDs, retained hidden/icon/unknown
fields, and allocation limits. This is expression/inspection proof only. It
neither runs `Level`, chooses a registry nor executes `PlayerEffects::add`.

No `ready_to_migrate` flag, persistent key or writer is exposed. In particular,
absence, an empty list, and a malformed original root remain distinguishable.
Raw parsed Tags are retained, not original NBT wire bytes; native parser
normalization still applies. Read-only observations confer no mutation authority.

## Required next adoption boundary, not implemented

A future owner must resolve the following together before replacing any record:

1. Give each active and hidden instance its owned record, preserving the existing
   clone/update/downgrade rules. Decide how unknown native fields and independent
   `show_icon` survive. The current proposed five-field Record cannot represent
   them, so it must not claim a lossless legacy conversion.
2. Validate the whole selected collection and resolve IDs before publication.
   Unknown IDs, duplicate/merge behavior, malformed data and limits need an
   explicit preservation/error path. Do not silently reuse today's skip behavior
   as authorization to erase newly introduced payloads or hidden instances.
3. Perform local and guest load/store changes coherently, including owner-file
   precedence, swap ownership, live-state capture and FIFO snapshot ordering.
   Retain source data on failure without later replaying stale effects over live
   state. A raw side table keyed only by effect ID cannot meet instance identity.
4. Preflight the entire new snapshot before its writer receives it. Preserve
   errors separately for each owner destination, and keep callbacks tied to the
   actual native completion path. The current logging-only two-file helper does
   not provide a combined success/error contract.

These are design gates, not user-facing policy defaults or an adopted schema.
The current experiment makes no save call and therefore cannot partially
replace a player's data when inspection fails.

## Synchronous contact authority and guest lifetime

`Game::as_guest_with_host` calls `enter_guest`, runs the callback within
`block_contact::in_level`, then calls `leave_guest`. Entry and exit each swap
`PotionState`/guest effects as well as body state. An already active guest is not
re-entered: `enter_guest` returns `None` for the same `lan_acting` connection.
A proposed operation must use the live lexical owner context, not resolve a
connection later and assume its saved roster currently owns the live effects.

`MotionWorld::check_inside_blocks` gathers `(iteration, position, state)` hits
before processing them. It captures `alive` for the sweep, applies shape/inside
checks, inserts the visited position, emits `Inside`, then performs the native
per-block action. The exact collected states/order and visited set are part of
this boundary. A callback that can mutate Game/world or teleport/remove an actor
changes assumptions in this traversal; simply dropping the MotionWorld borrow,
calling arbitrary code and recreating it is not yet a proven continuation.

`Game::add_player_effect` needs native status/absorption updates and mutable Game
callbacks, not merely the effect list. The minimum viable upstream design must
synchronously end the relevant temporary borrows, invoke that complete operation
on the same supported local/guest owner, and resume a defined continuation before
the next per-block action. It must explicitly define actor/world changes and
reentrancy. Remote clients, prediction replay and collision-only probes cannot
receive the server mutation boundary. No public boolean or numeric epoch check
alone supplies this authority.

The new source guards confirm the current ordering/swap anchors only. They do
not execute actor swapping, contact callbacks or a continuation. No mocked Game,
unsafe reborrow, delayed effect queue or substitute mutation sink is introduced.
