# Native-item cloud composition guard candidate

This is a **source-only, incomplete admission candidate** on integration
`6ec43252746dfe905a40b6c9f8cb857d087affb8`. It changes guard code, metadata and
guard tests only. The five Rust payloads, TConstruct manifest and ordinary
behavior from cloud intake `c97771ccb2af0a5fa9e0bf5129ce1f9c065e0a60` are not
changed. No new native run, type check, actual Game test, spawn, LAN or Windows
play result is claimed.

## Evidence and mandatory remaining rejection

The original composition was observed in run `42236543685`, runtime `9cc1a016`,
plan SHA256
`7139236d6484c9cf5a862d3e8358d60b2b4e0bd3a8c4c0968c6d313735670f8b`.
The published observed-IR ledger is commit `280f4621`,
`pluto-lib/docs/cloud-guard-conflicts19-41f.json`. This supplies real node,
fingerprint, owner, binding, phase, priority and payload identities for the
old composition; it is not an observed c977/6ec432 plan.

The moved `source-fluid-cloud` module has **null fingerprint**, and
`candidate_observation` is null. The real checker therefore rejects every
candidate plan until the coordinator obtains and reviews the actual new plan.
No fingerprint is copied from an adjacent Game module. No null matches a
wildcard. Recording this pending state is not an instruction to skip a guard.

Before actual admission, compare the new full plan with this contract, record
the new runtime/evidence commit and plan hash, and populate only actual observed
placement identity. Any other payload/IR change needs its own review. Then run
the normal full guard set, apply/type/build gates and the nine named tests in
the actual `horizon-client` Game-containing test artifact. Passing the nine
value/type tests still would not be a live cloud publication or charge test.

## What changes in the guard

The former blanket rejection of the nineteen reviewed entries is replaced with
exact contract matching, not an ID, file, phase or count allowance:

- Every selected module and injection must occur once with the reviewed owner,
  visibility, placement, complete schema-6 IR and exact UTF-8 payload hash.
- All injection fields are compared, including bindings, cancellation, priority,
  selector, ordinal, require, output contract and node/fingerprints. Extra fields
  and boolean/integer substitutions reject. Default omitted `pub(crate)` is the
  sole module serialization normalization.
- Reviewed direct state rows bind actual type, initializer, owner, placement and
  constructors. Missing, duplicated, aliased or changed rows reject.
- Unknown hooks on the original exclusive files/prefixes, known item entry
  targets, exact protected sites and selected helper interiors still reject,
  regardless of hook kind or a changed ID. Unknown modules in original exclusive
  files and Game/lifecycle state/constructor rewrites still reject.
- Existing local helper-byte checks and the official source/plan-input binding
  remain. The seventeen official pins are adapted using the separate 41f source
  audit, not by accepting any drift. Four directly relied-on native files are
  added: `native_machines.rs`, `sculk.rs`, `lan_crafting.rs`, `native_hooks.rs`.
  The first two are also exclusive: the observed composition has no hook or
  inserted module there.

The explicit contract currently contains nineteen modules (five Pluto modules,
three previously conflicting modules, the moved cloud consumer, ten direct
dependencies), twenty-seven hooks and forty-five directly used state rows.
These counts describe the artifact, **not an acceptance criterion**. They do not
freeze all 321 modules/322 hooks, or a name-search closure over unrelated APIs.

## Why the nineteen operations do not imply an allocator/owner bypass

Each of the nineteen entries has its own source rationale in the contract.
The key distinctions are:

1. `enter_guest` only adds the exact protection-map observation before the native
   owner setup. Area/transform packet hooks only update their policy maps. Those
   numeric entries never become cloud authority.
2. The glow action is a PlayerAction branch and the book resolver is a specific
   CustomPayload branch. Native UseItemOn takes neither. The exact nonmatching
   Continue branches are retained; this is not a claim that their other branches
   are effect-free or that book/UI correctness was established here.
3. The splashing packet HEAD clears pending work before UseItemOn. Only the exact
   two interact writers insert that pending record in the observed composition.
   Their source and entry hooks are fixed. The swing-back target-swap path has
   no pending record from this block-cloud action. Interact's effects remain a
   separate behavior, not a new cloud authority source.
4. Menu returns, bottle filling and machine operations can allocate item IDs and
   perform native callbacks. They are **earlier/later operations**, not calls
   between cloud's successor check, pure construction, reserve, take-ID and push.
   The exact native `as_guest`, actor setup, lifecycle/epoch lookup and allocation
   boundaries remain protected. Advancing the global counter before a later
   cloud check is not the same as invalidating a previously reserved cloud ID.
5. The world-set cache helper also conditionally edits saved material-block NBT;
   it is not mislabeled as a no-op. Its exact bytes and directly touched states
   are bound. It does not interpose in the callback-free cloud publication tail.
6. The cloud consumer itself is exact c977, retaining guest and guestless native
   Player attribution, old insert/resolver inversion, explicit effect budget,
   native color calculation and the post-publication mixed-action accounting.
   Its snapshot does not prove an ABA history or authorize arbitrary direct
   calls. The fixed actual native caller is part of the reviewed contract.

The source review does not certify arbitrary future Mod callbacks or the entire
behavior of machine, book and combat dependencies. Their broad gameplay policy
is not a new proof goal. The directly selected bytes, native caller/restore
boundary and callback-free publication tail are what this guard constrains.
Unrelated composition can change without this guard certifying it; the other
guards and actual build remain required.

## Pure checks and history

Thirty-five pure Python tests passed: twenty-seven new checker regressions and
the eight existing cloud plan tests adapted to a complete synthetic contract.
The old test that used an arbitrary `use_tool` hook as an unrelated hook now
uses `unrelated_draw`: `use_tool` is deliberately protected for the new native
item connection, and unknown hooks there reject in the new tests.

The first development run was 16/20: the metadata generator retained the old
`source` field when replacing a candidate body. The positive synthetic tests
correctly rejected that inconsistent record. The generator was corrected to
remove the original body before storing its digest; the final 35 pass. No Rust
or native job was involved, and no failed test was reclassified as a pass.

Pure tests use explicitly synthetic IR/source text to exercise checker behavior.
Separately, comparison against the recovered actual plan confirmed that only
the five known c977 modules differ: descriptor, insertion, potion facts,
splashing source and cloud source/placement. Existing entry identities and
all selected hook/state records are actual-plan-derived. The production
pending-observation rejection was also exercised against that old real plan.
