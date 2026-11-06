# Unapplied official BookScreen storage prototype

This is a proposed patch **stored only in horizon-mods**. It has not been
applied to any official checkout, installed by PlutoLib, or enabled in TConstruct.
The live LAN request/response path, cover, input, drawing and packet observation
remain unchanged. No user Horizon fork was accessed. This is not a merge-ready
upstream patch or a completed native screen-lifetime service.

## Actual host structure, not a replacement screen model

At official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`, Game owns
`Option<books::BookScreen>`. The original BookScreen enum, native Text pages,
edit boxes and lectern fields remain byte-for-byte unchanged in this proposal.
The field becomes `RetainedBook<books::BookScreen>`, initialized with its own
Default. No fake Game, renderer or native screen enum is substituted in the patch.

The proposed wrapper provides the existing Option-shaped reader methods,
native mutable projection, take, explicit assignment and mem::replace behavior.
Direct read patterns are adapted to `as_ref()`. The patch routes six direct
assignments, four take sites and one mem::replace site through the storage
boundary; 25 existing `as_mut()` calls remain and advance revision before the
borrow. Eight production player assignments, three actual remote take branches
and the connection install point invalidate attachment identity. Multiple
transitions in one operation can advance identity more than once; IDs are not
an event count. Server guest/fake-player and test-only player swaps are listed
separately, not misrepresented as window-client lifecycle proof.

The original dialog overlay module is unchanged. Showing or dismissing an
overlay does not mutate the slot. Actual ContainerClose disposal and canceled
command restoration are routed separately. Restoration always mints a fresh
opening even for identical contents. Take/replace return the original value
without cloning it. Assignment retains ordinary assignment's destructor order;
identity is revoked before a potentially panicking destructor. Exhaustion
removes trackable identity rather than blocking ordinary local contents or edits.
No cover sentinel, page bounds, audio, data format or input-admission policy is
implemented in the wrapper.

## Exact limits of this instrumentation

The returned stamp is **observation, not an owner handle or authority**. There
is deliberately no public/installed owned-open operation, reply callback or
conditional content-update capability. The stronger owned-opening/reference
model remains in `../retained_model.rs`; this storage prototype does not replace
that model's missing native connection.

`as_mut()` conservatively increments revision even if no write follows. It
returns the mutable native value for the pinned existing call sites. Arbitrary
trusted Rust could replace the whole enum through that reference, bypassing an
opening-generation distinction. The reviewed current call sites mutate fields;
a new variant-replacement path must be rejected/reviewed. This is not a stable
public mutation API, an exact change notification system, or a sandbox for
arbitrary injected Rust. A future public boundary needs restricted projection or
explicit owner-checked updates, not publication of this raw native borrow.

Attachment instrumentation is tied to the listed source operations. Some
native assignments may clear an already empty player; conservative extra
revocation is possible. This does not prove the exact UI availability semantics
required for feature-equivalent LAN recovery. The patch does not serialize
identities, move an old reply between clients, modify observers or disable the
old response handler. Applying the field-type proposal would also require
reviewing consumer Mixins that pattern-match or assign Game.book directly;
this prototype is not a compatibility claim for the full current MOD bundle.

## Reproducible source proof

`official-sources.json` pins 19 relevant official files and inventories 79 direct
`self.book` accesses plus player/connection mutation sites. The generator
re-enumerates those direct spellings across official `game.rs` and its Rust
subtree before reading the pinned sources. It fails on changed hashes, new
listed paths/sites, wrong rewrite counts or remaining raw Option accesses.
This is a source inventory, not general Rust alias/dataflow analysis.

`generate.py` has **no apply, build, fetch or official-source write mode**.
It transforms strings in memory, writes an ordinary unified diff only with
`--write`, independently applies the diff to the original strings in memory,
compares every resulting byte, and rechecks official input hashes. The output
has 11 changed host files: ten existing files and one new generic storage module.
`generation.json` records original/output hashes and every rewrite count.

From an explicitly approved exact official source tree:

    PYTHONDONTWRITEBYTECODE=1 python3 generate.py --official /path/to/official/source
    PYTHONDONTWRITEBYTECODE=1 PLUTO_OFFICIAL_SOURCE=/path/to/official/source python3 test_patch.py

Seven Python source/hunk regressions pass, including drift rejection. The first
negative hunk fixture accidentally replaced a nonexistent removed-line string;
it was corrected to change actual context and assert that the fixture differs.
That was a test-fixture correction, not a successful native apply or a host fix.

The separate `storage_fixture.rs` includes the **same proposed helper bytes**
as a module. Its seven tests exercise storage mechanics with inert enum/primitive
values and explicit destructor cases, not the real BookScreen or a real Client.
At exact source `43178aefdf9d35c46d1264ebb1f3c679285e08d1`, standalone
compilation and all seven focused tests passed (zero filtered). The actual helper,
fixture, generator, patch and 19 official source pins were unchanged before and
after; minimum workspace free space was 10,891,452,416 bytes, above the
10 GiB + 128 MiB floor. `validation.json` records commands and artifact hashes.
This proves only the proposed generic storage mechanics. No actual Game or
BookScreen was compiled, no native patch was applied, and no screen lifecycle,
renderer or LAN behavior was executed. The patch remains excluded and unapplied.

## Still required before any activation

- Full official Game/BookScreen compile and borrow/type validation for the exact
  generated patch, plus complete package/Mixins composition and drift guards.
- A reviewed native owner acquisition and conditional retained-view update
  boundary, separate from input admission and ordinary observed stamps.
- Real window tests for replacement, overlay retention, disposal/restoration,
  same-key player reset, connection changes and late/duplicate replies.
- Actual legacy codec/native Text equivalence, bounded envelope admission in
  the real observer route, and consumer cover/page policy migration.

No approved full Client/renderer resource environment has been established.
The existing ordinary library proof and PR #945 navigation proof remain separate.

## Attribution

Patch context is from the official Horizon revision named above, copyright
2026 Ludovic Marie and the Horizon contributors, under the MIT License.
The complete notice is preserved in `HORIZON-LICENSE.txt`. Generator, storage
instrumentation and tests are authored here; no source is taken from the
prohibited user fork.
