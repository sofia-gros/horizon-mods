# Candidate deletion-sync validation preparation

Status: source prepared only. Runtime remains exactly the five Rust/manifest
inputs in candidate commit `0f528de0332afc5d86910dd07b08dcebe98a04dd`.
Initial CI run 42196270581 accepted the actual plan, exclusive guard and apply, then failed the preparer file-set check before native compilation. Rust/model/codec tests remain unrun. See results/attempt1-42196270581.json.
The candidate is still excluded from the normal PlutoLib manifest/package.

This follow-up provides the guard previously listed as missing in the candidate
README. The first actual candidate plan passed this guard. Twelve guard tests and eight preparer checks are separate from
the **32 pending Rust tests**: 18 native codec cases, eight actual entry-gate
model cases and six owned-Tag adapter cases. All exact names are in
`fixture-inputs.json`, split into nonoverlapping groups. Run the complete fixture
once with no filter, require all 32 names and zero failures/ignored/filtered.

## Complete plan before apply

`check_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE` checks exact
candidate bytes, official source hashes, function/site fingerprints, bindings,
contracts, helper/module ownership and visibility. The plan must be schema 6.
The three selected targets are exclusive. The two entire guest/receive source
files are conservatively reserved against other injections, modules, bridges
and state constructors; this prevents another guest-local extension from
arming the internal gate. Candidate post-build mutation is not supported.

This intentionally does not promise compatibility with another mod targeting
those reserved files. The shared package must be reviewed separately before
production integration. Synthetic guard records do not establish actual
Mixins acceptance and cannot be used as the stage for the codec fixture.

## Real source and dependency identity

The CI owner must obtain the pinned official source and pinned Mixins, run the
actual candidate plan and mandatory guard, then run the actual apply into a
separate generated stage. Capture apply output/exit and seal all stage files
immediately. Only afterward call:

```
python3 prepare_fixture.py --official OFFICIAL_GIT_CHECKOUT \
  --stage ACTUAL_APPLIED_STAGE --plan CHECKED_PLAN.json --output NEW_FIXTURE
python3 prepare_fixture.py --output NEW_FIXTURE --resolve
python3 prepare_fixture.py --output NEW_FIXTURE --verify
```

Preparation does not invoke plan/apply or compile. `--resolve` separately uses
offline Cargo metadata to adapt only the fixture's copied lock, and verifies
every dependency against the official lock. Official cleanliness also rejects
untracked and ignored files, including untracked Cargo configuration. Keep all
build/fetch outputs outside that checkout. The four direct native crates must
each have one actual official local package identity. A cold CI owner seeds
the official locked dependency cache first, never an inferred substitute graph.

The preparer verifies that the stage marker equals the checked plan, the
complete stage file-set matches the native plan inputs plus the stage marker and
exact generated `native-post-build.json` bytes `[]\n`, and
only `client_lib.rs`, `item_sync.rs` and `lan/guest.rs` changed. The single
export-only `.horizon-revision` input must equal the exact official revision
plus one newline in both the plan hash and stage bytes. It is not read from the
Git checkout, where it does not exist. No other missing input is excused.
A marker alone
does not prove who produced a stage; the CI must retain the actual apply log and
continuous input/output seals. No copied semantic mock of the native item or
protocol codec is used.

Actual applied regions:
- `item_sync`'s entire pre-test module, including both receive boundaries.
- `guest`'s native network conversion/support functions, with the applied HEAD
  and the actual appended gate module.
- The actual appended owned-Tag module.

The unchanged native versions of the same conversion regions are compiled as
separate differential oracles. Native `DYES` is an exact declaration fragment.
The tests supply small controlled native `Builtin`/`Dynamic` registry tables;
they do not execute registry negotiation or load a complete server data pack.
Existing `horizon-items` and `horizon-client-core` reexports provide the real
item/data/firework/primitive NBT types. Generated imports are wiring, not mocked
behavior. Extraction boundaries and every resulting byte hash are recorded.

## Proof boundaries

The 18 codec cases cover canonical root/charged/bundle/container paths, mixed
nested call order/count, None, no-marker equivalence, contradictory/duplicate
removal, independent custom names, native raw Wire baseline/count-only echo,
canonical edits, native callback panic/unwind and concurrent thread isolation.
Unknown nested custom-data NBT is checked using the actual codec; no claim is
made for every unknown protocol component or native lossy key normalization.

Raw echo is constructed with the actual `ItemStack::wire_patch`, `WirePatch`
and `DelimitedSlot` codec. Actual `Game::to_network` is not linked/executed:
its pinned source establishes the raw-before-canonical order, while its
permission/actor/network operations stay outside this bounded fixture.
Similarly, a passed region fixture is not a complete client-library check or
gameplay result. Default-component ownership and actual displayed Game names
remain separate unresolved work.

CI must run on its own approved slot with normal filesystem floors, a bounded
timeout, actual compiler evidence, strict complete test-name checks, pre/post
source/lock/stage/fixture seals and no surviving process group. This source
checkpoint does not dispatch CI or authorize local native work.

## First-attempt preparer correction

Official Mixins `native.rs` 630–694 unconditionally emits `native-post-build.json`, even for an empty post-build list. The first fixture file-set expectation omitted it. The source-derived diagnosis is concrete, but the first remote stage's file listing was not recovered from that job's log. This correction permits only the exact empty generated file, rejects it as an original input, still rejects every unknown extra/missing file, and reports the exact missing/unexpected names on failure. Nonempty or differently serialized contents are rejected. No runtime helper, codec case or pristine oracle is changed. A corrected retry has not run.
