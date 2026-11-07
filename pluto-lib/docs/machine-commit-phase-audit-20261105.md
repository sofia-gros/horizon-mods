# Machine storage publication and notification boundary

Read-only audit on 2026-11-05. Reviewed MOD source `4d82580b32a1c040001add8c853d93d331bc493a`; the files below are unchanged at central cutoff `82386588`. Native source is official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`. No source mutation, native plan, test or runtime reproduction was performed for this audit. The complete Pluto package proof does not establish the complete TConstruct consumer bundle.

## Concrete callers and publication order

`source-machine-runtime.rs::save_casting_tank` (lines 302–315) validates the proposal's save eligibility, requires an existing two-slot Chest, reads raw metadata, and checks that the current live state can reproduce it using the caller's preservation codec. It computes the old comparator, prepares preserved new metadata, then:

1. Writes the saved machine metadata through `source-melter-store.rs::write`.
2. Copies the two proposed native stacks into the live Chest.
3. Runs `notify_casting_comparator` when comparator strength changed.
4. Always runs `Game::container_changed`.
5. Returns true.

`save_cannon` (lines 402–412) has the analogous sequence for its single live stack and Cannon-specific preservation rules. Its old-state canonical check deliberately uses `tank_to_nbt_preserving`; this is not interchangeable with Casting Tank's full saved-state identity.

Between the metadata write and live-slot copy, the actual code calls no Game callback. The metadata writer receives one exclusive `&mut World`, obtains the existing chunk, performs copy-on-write saved-source changes, and marks the chunk modified. The earlier Chest check cannot ordinarily be invalidated by a concurrent safe borrow inside that callback-free interval. This review does not turn the later defensive `get_mut` into a demonstrated partial-write bug.

The callers finish other parts of the exchange only after the helper has notified:

- Casting Tank fluid exchange (lines 370–374): machine save and notifications, then creative gift or acting-hand replacement and optional `give_or_drop`.
- Casting Tank item removal (lines 378–381): machine save and notifications, then `give_or_drop`.
- Casting Tank insertion (lines 390–392): machine save and notifications, then the reduced acting-hand stack.
- Cannon fluid exchange (lines 433–448): revalidate snapshots, write only preserved tank metadata, comparator/container notifications, then creative gift or acting-hand replacement/gift. This route intentionally leaves the Cannon live item unchanged.
- Cannon item exchange (lines 456–481): save/notify before clearing or filling the acting hand. The branch returning a Cannon stack to an already occupied player's inventory first stages `inventory_add` on copied inventory/offhand, saves/notifies the Cannon, and then replaces the player's inventory/offhand with those proposals.

These are several publication steps separated by native work, not one callback-free atomic exchange. Current callers re-resolve the acting hand, creative flag or inventory after notification; they do not own an invocation token for the earlier physical hand slot. This is a contract gap to define before generalizing, not a reproduced stale-slot exploit.

## What native notification actually does

`notify_casting_comparator` calls `Game::with_level` to run `Behaviours::update_neighbour_for_output_signal` (source-machine-runtime.rs lines 317–321). It is skipped when the numeric comparator value is unchanged.

Official `Game::container_changed` (game/machines.rs lines 565–585) is not a passive event counter. It synchronizes a chiseled bookshelf when applicable, marks Shelf updates when applicable, enumerates a valid double-chest partner, marks chunks modified, and calls `with_level` for each container position. The reviewed TConstruct helpers require Chest storage, so the bookshelf/Shelf cases are not automatically taken. A partner also requires the actual block's chest state; a generic Chest storage variant alone does not imply a double chest.

The actual neighbor path is:

- `level/signals.rs::Behaviours::container_changed` reads current block state and, unless air, dispatches output-signal updates.
- `level/update.rs::update_neighbour_for_output_signal` (lines 679–695) visits neighboring comparators, including one behind a conductor, and enters the neighbor update queue.
- `add_and_run` and `run_updates` (lines 607–644) execute synchronously when this is the outer update. If already inside an update chain, they enqueue work for that existing chain instead.
- `level/hooks.rs::neighbor_changed_oriented` first calls the level's neighbor observation, then native block-class behavior. `GameLevel::observe_neighbor_changed` (game.rs lines 1204–1213) dispatches the authoritative native observation with an immutable level receiver.
- Comparator/diode behavior can schedule work or remove an unsupported block and produce drops (`level/redstone.rs::diode_neighbor_changed`, lines 667–680).

`Game::with_level` is also more than the closure it receives. It builds a scoped `GameLevel` over the mutable world and other disjoint fields, captures current player collision facts, runs the supplied closure, **then calls `flush_spill`**, and flushes LAN particles/sounds if a LAN host is present (game.rs lines 4595–4783). This scoped level does not itself retain a mutable player inventory slot or a whole mutable Game. The immutable native neighbor observation is not evidence that an ordinary callback can directly rewrite the acting hand.

After that scoped level returns, however, `flush_spill` resumes with mutable Game (game/spill.rs lines 733–765). If spill is nonempty, it can retell used-drop runs through `native_use_drops`, spawn item entities, award experience, and close a removed container. `spawn_item` runs entity-join handling and native item-spawn observation (game.rs lines 4093–4134). These effects are real native work before the TConstruct caller's remaining held/inventory publication, and their invocation/visibility must be preserved or deliberately changed. This audit does not claim that an ordinary comparator always produces spill, that a particular callback mutates equipment, or that all nested updates execute before return.

## Existing fixture evidence does not cover this phase

`tconstruct/tests/rust/cannon_held_fixture.rs` uses the actual selected consumer functions, native ItemStack/NBT and inventory-add behavior, with deliberately bounded host doubles. In particular:

- `container_changed` only increments `updates` (line 54).
- `notify_casting_comparator` only increments `signals` (line 80).
- `give_or_drop` only appends to `gifts`.
- The fixture's held cell is independent of its inventory/selected fields rather than the native late-bound physical-slot resolution.
- Its injected failures happen in datapack preparation or the inventory-move callback before commit. Those tests do not execute native neighbor simulation, spill flushing or actor/hand changes during notification.

The existing successful cases remain valid for their recorded scope. They must not be cited as proof that machine/held publication is atomic across native notification. Adding a synthetic post-notification mutation to this double could test a proposed policy, but would not reproduce an actual native callback or establish host authority.

## Extraction decision

The shared codecs, owned-field patching, proposal staging and value comparators already cover the mechanical non-publishing work. No additional generalized commit service is justified by this audit alone.

Moving the held/inventory publication before notifications would change what native observations and effects see. Splitting each helper into a write phase and a notification receipt could make that order explicit, but is not a semantics-preserving replacement without a caller policy decision and a native phase proof. Retrying after notification or rejecting only the final held write cannot undo saved machine state, world updates, emitted native effects or already-created entities. A stronger value comparator alone cannot solve this.

The next useful gate is one actual native machine exchange with explicit assertions for the before/after visible machine and physical held slot, effect order and invocation counts, preexisting/new spill, queued versus outer neighbor work, creative behavior, local/LAN actor scope, and failure before any publication. Keep the current TConstruct schemas, aliases, retained-tool rules and distinct Cannon/Casting Tank policies unchanged while defining that gate. Do not expose a general public write callback or silently reorder the current code.
