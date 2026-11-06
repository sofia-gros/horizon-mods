# Unapplied native item-action cloud candidate

**Source-only, excluded from the ordinary bundle. Not approved for activation.**
The old `insert` and `resolve_owner` are byte-identical after removing the named
addition. This is not a relaxed EntityRef resolver. The proposed new entry has a
*different, narrow caller precondition*: the completed native held-item action at
`Game::use_block_transformer`, reached through `use_item` / `with_player_actor`.
Matching a copyable observation does not itself grant authority.

Fixed source: Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf`, Mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57`, MOD integration runtime
`9cc1a01632913f01df1f0bcbb0b74b8bb9e80525`. Actual accepted complete-plan identity
for this candidate and native type result are **null**. The prior complete plan
was accepted for unmodified integration, but its cloud guard rejected source
identity. That is not this candidate's acceptance.

## Native owner, not a synthetic handle

- Guest packets enter `lan_guest_packet` with a real borrowed `LanHost`.
  `as_guest` / `enter_guest` finds that guest, swaps its real PlayerMotion and
  sets conn, UUID and entity ID from the same guest. `UseItemOn` sets OnBlock and
  invokes `use_item`. `with_player_actor` supplies `Who::Player`. The roster may
  be detached from `Game.lan` while this real native action executes.
- `native_use_as_player` creates a temporary real PlayerMotion/inventory, with
  no guest connection, and invokes the same `use_item` route. Local native use
  can also have no guest. Native `potion_owner(Some(Who::Player))` and
  `launch_thrown` retain **Player with guest=None**, not an unowned cloud.
- The new classification preserves both representations. It does not claim a
  persistent fake UUID/entity, make an EntityRef for the actor, call
  `ensure_player_uuid`, use a side table, queue a delayed effect, or reattach a
  host. The only new EntityRef is the successfully allocated cloud's return.
- Current checks require Ready authoritative server/epoch/actual LevelRef,
  current body, Player actor, internally complete guest tuple matching that
  body when present, block-use phase and hand. No-guest local/fake allows the
  native None or OnBlock phase. Hand slot and creative mode are retained for
  continuation checks. Remote, unready, partial tuple and other phases reject.
- Upstream packet `client_loaded()` / alive admission is **not** a fresh roster
  query in the detached helper. Raw Guest.loaded is not the same condition.
  No check here asserts that Game fields independently authenticate arbitrary
  direct calls. Entry/exit swap windows and caller-injected impersonation remain
  outside the contract.

Sources (exact whole-file SHA/blob pins in inputs.json):
`game/lan_actions.rs` enter_guest / leave_guest / UseItemOn;
`game/native_machines.rs` native_use_as_player;
`game.rs` use_item, use_item_as_player, use_tool, use_block_transformer,
held_slot, with_hand;
`game/sculk.rs` with_player_actor / sculk_actor;
`game/potions.rs` potion_owner / launch_thrown / lingering construction;
`game/native_entity_refs.rs`, `native_level_refs.rs`, `mod_api_host.rs`.
All are under https://github.com/horizon-rust/horizon/tree/41f822b659789780f58d32c63ddfb6887ea1b8bf/crates/horizon-client/src .

## Publication and consumer deltas (v2, after unadopted 16a6)

The new entry rechecks the frame and exact requested Player/guest attribution.
Its center-loaded / successor / construct / reserve / take-ID / assign-ID /
push tail is byte-identical to the existing insert tail. All recoverable errors
are before native allocation. No rollback after publication is claimed.

`source-fluid-cloud.rs` keeps the native-frame design reviewed at 16a6, but
removes the new count restriction and manual color mix from that candidate. It retains TConstruct's
4x input duration / 0.25 native scale, RGB, source label, order and cloud timing.
The original legacy struct literal omitted newly required native fields; this
candidate delegates those fields to the existing factory. It does **not** claim
that the old uncompilable literal previously proved native attribution.

`source-splashing.rs` has four reversible changes in its block method only:
observe before cooldown; track successful publication; set cloud success only
after Result::Ok; recheck actor/phase/slot/mode and held count/components before
item charge. Existing mixed glow still follows cloud, and original successful
amount selection, partial-error accounting and later wear policy are retained.
No missing cloud is counted as a successful cloud. An unsupported initial
cloud context returns an explicit error before cooldown, world effects or
charge, for both pure and mixed requests. The glow-only fallback added in the
unadopted 70a/16a6 candidates was not in the original ordinary source; v2 removes
it rather than guessing that an unadmitted action may continue. Normal admitted
guest/local/fake actions retain cloud-then-glow order. Existing partial errors
after an actual cloud publication still select the original cloud_stage and
perform the current-frame pre-charge check. No rollback is claimed.

The default `pluto_cloud::validate` still enforces 256, with the identical old
validation body and error precedence after a wrapper's always-valid default
budget check. Additive `validate_with_effect_budget(Descriptor, usize)` uses the
same body and an explicit finite count. It rejects multiplication overflow or
budgets impossible for either owned vector layout before resolving effects.
Zero still cannot accept an empty cloud. This is not a boolean unlimited mode,
constant MAX increase or early acceptance of unvalidated effects.

TConstruct passes the exact length of the complete sequence already admitted
and owned by its legacy parser. It neither splits clouds nor deduplicates or
filters entries. That closes the 257-entry rejection introduced by 16a6 without
changing the default API contract. All entries, including a conflicting flag or
invalid amplifier at index256, pass through the same validator.

Color is an explicit consuming operation, `Validated::with_native_effect_color`.
TConstruct first validates all fields/budget with explicit RGB0, then chooses this
native operation. It uses the same validated ordered visible effects and calls
`pluto_potions::effects_color`, a single delegation to original native
`potions::effects_color`. It accepts no arbitrary raw u32. Both default and
explicit-budget validators retain their RGB24bit input rejection. The native
operation's internally computed bits, however, retain native duplicate weighting
and u32 overflow/profile behavior, even if extreme wrapping arithmetic produces
bits outside RGB24. No capped hand-written sum or wider-integer mix is used.
A symbolic wrapping example at 16,777,217 entries produces 0xffffefff; it was not
allocated or executed. The source delegates that arithmetic, including native
panic behavior. The real Rust tests use only small counts (up to512), raw-color
rejection and the visible-effect selection. They do not execute huge arrays. Input admission and memory cost
remain the consumer's responsibility, exactly as for its existing owned array.
No native allocation/resource-failure behavior is promoted into a success.

Finite prepared cases (not run in Game): guest main/off; local/fake no guest;
partial tuple, wrong body, wrong phase; epoch/dimension/connection/UUID/body/
hand/slot/creative change; exact requested owner disagreement; source proves
success flag after publication and pre-charge checks before creative selection.
Four new classification/value Rust tests plus two unchanged insert tests and
three prior consumer policy tests are **prepared, unexecuted**. Eighteen Python
source/inverse checks pass. Those checks do not execute the native publication
or the mixed-effect callbacks.

## Required gates and remaining limits

1. Independent source review of actual caller authority and exact native fields.
2. Ordinary module placement, if adopted, moves source-fluid-cloud from the
   potions child into Game; inputs records the requested change but no manifest
   is applied here. Existing cloud factory/private lifecycle visibility must be
   accepted and type-checked in the real host.
3. Review the fixed actual complete installed plan's native action setup,
   restore, known payloads and calls against the cloud guard's concrete owner,
   readiness and publication invariants. The current broad guard cannot simply
   admit an ID/position prefix. Known noninterference candidates need exact
   payload/node/binding review. This does not create a new requirement to prove
   every arbitrary hostile mod or hypothetical callback history. No guard is
   edited here.
4. Real guest, local and fake action tests must prove Ready/level/hand admission,
   attribution, reserve/ID/publication order and original consumption order.
   Equality of observed fields does not encode an ABA history; that limitation
   is separate from whether a concrete wrong-owner/charge path is reachable in
   this fixed composition. No such new path is asserted without source/runtime
   evidence. Existing later `damage_with_cause` callback behavior is unchanged,
   not newly proven safe.
5. Compare default256 versus explicit 257/256/0/usize-overflow, invalid final
   flags/amplifiers and ordered native color in the real host tests. Nine Rust
   tests remain prepared only; v2 extends their cases rather than claiming the
   old source checks executed them. Resource allocation and actual world
   failure paths remain separate native tests.
6. No live owner, gameplay, Windows, render, contact timing, or cloud effect
   execution proof is supplied by this source checkpoint.
