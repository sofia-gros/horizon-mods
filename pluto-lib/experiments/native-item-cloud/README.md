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

## Publication and consumer deltas

The new entry rechecks the frame and exact requested Player/guest attribution.
Its center-loaded / successor / construct / reserve / take-ID / assign-ID /
push tail is byte-identical to the existing insert tail. All recoverable errors
are before native allocation. No rollback after publication is claimed.

`source-fluid-cloud.rs` is the prior reviewed-but-unadopted 70a589 request-policy
candidate with only frame names/description changed. It retains TConstruct's
4x input duration / 0.25 native scale, RGB, source label, order and cloud timing.
The original legacy struct literal omitted newly required native fields; this
candidate delegates those fields to the existing factory. It does **not** claim
that the old uncompilable literal previously proved native attribution.

`source-splashing.rs` has four reversible changes in its block method only:
observe before cooldown; track successful publication; set cloud success only
after Result::Ok; recheck actor/phase/slot/mode and held count/components before
item charge. Existing mixed glow still follows cloud, and original successful
amount selection, partial-error accounting and later wear policy are retained.
No missing cloud is counted as a successful cloud. A pure unsupported cloud
returns an explicit error before cooldown. The candidate retains the prior
70a589 explicit mixed glow-only attempt when initial cloud admission fails;
this error policy requires review and is not silent ordinary adoption.

Finite prepared cases (not run in Game): guest main/off; local/fake no guest;
partial tuple, wrong body, wrong phase; epoch/dimension/connection/UUID/body/
hand/slot/creative change; exact requested owner disagreement; source proves
success flag after publication and pre-charge checks before creative selection.
Four new classification/value Rust tests plus two unchanged insert tests and
three prior consumer policy tests are **prepared, unexecuted**. Fourteen Python
source/inverse checks pass. Those checks do not execute the native publication
or the mixed-effect callbacks.

## Required gates and remaining limits

1. Independent source review of actual caller authority and exact native fields.
2. Ordinary module placement, if adopted, moves source-fluid-cloud from the
   potions child into Game; inputs records the requested change but no manifest
   is applied here. Existing cloud factory/private lifecycle visibility must be
   accepted and type-checked in the real host.
3. An actual complete installed plan must pin the exact native action setup,
   restore, call chain and all intervening callbacks. The current broad cloud
   guard cannot simply admit an ID/position prefix. Known noninterference
   candidates need exact payload/node/binding review. No guard is edited here.
4. Real guest, local and fake action tests must prove Ready/level/hand admission,
   attribution, reserve/ID/publication order and original consumption order.
   Arbitrary direct calls, ABA swap-away-and-back and hostile callback mutation
   are not solved by equality of the observed fields. Existing later
   `damage_with_cause` callback behavior is unchanged, not newly proven safe.
5. Validation rejects >256 effects/invalid requests. The legacy parser had no
   same explicit effect-count bound; preservation for that domain is **not**
   established. Do not silently adopt a feature loss or label all recipes equal.
6. No live owner, gameplay, Windows, render, contact timing, or cloud effect
   execution proof is supplied by this source checkpoint.
