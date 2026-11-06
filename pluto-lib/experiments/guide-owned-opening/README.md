# Owned retained opening: excluded source candidate

This is a **source-only, unexecuted** extension of the proposed native storage
wrapper. It is not an ordinary Pluto API, installed Mixins adapter, packet hook,
language listener or TConstruct change. Existing LAN and screen handlers remain
unchanged. The successful [751999ea typing result](../../docs/guide-storage-type-ci-result-20261106.md)
does not cover these new files.

## One boundary only

`owned.rs` is appended to the same private `game::retained_book` module as the
exact old helper. It reuses that store, `Stamp`, checked `NEXT` allocator and all
native invalidation paths. No registry, parallel catalog or identity counter is
added. Its non-Clone/non-Copy `OwnedOpening` has private fields and can only be
returned by `try_install_owned`, which **actually installs** the supplied value.
Reading an existing native book or its stamp cannot acquire it. This is a local
storage capability, not proof that a mod is authorized to open a Game screen.

Installation allocates identity before changing the old slot. An exhausted
allocator or unavailable attachment returns the original candidate unchanged.
On success the old value is returned, never destroyed within the installation.
Native set/replace, close/reopen, save/restore and attachment invalidation revoke
an earlier owner. A page turn only advances revision, so it does not revoke the
opening; preparation must use a fresh commit-time observation rather than the
revision remembered when a network request was sent.

`prepare_owned_replace(owner, expected)` validates owner, current observation and
revision capacity before returning a guard holding the unique mutable slot
borrow. Dropping that guard does nothing. The guard's one consuming `commit`
returns the old value after the new slot/revision is committed. It does not run a
callback, allocate another identity, drop the old value or silently roll back.
Release the enclosing Game/RefCell borrow before dropping the returned value.
A destructor may legitimately perform a later native change or panic: neither
permits automatic retry, and no write follows destruction inside this service.

Native code remains trusted and can call ordinary mutation methods or use its
private mutable value borrow. This is not a sandbox against arbitrary native
mods. Whole-plan/caller coverage and authorization of the eventual public opening
operation remain separate requirements.

## Actual native type candidate, not an executed adapter

`book-view.rs` is intended to be appended in that same private module for a later
isolated actual Game type gate. It imports the real `books::BookScreen` and native
`Text`, rejects Edit/lectern/empty replacements, and prepares a View using the
current legacy cover sentinel / numeric clamp. It preserves content below an
overlay by not substituting input eligibility for retained-view ownership. It
does not modify navigation history. Failure returns all candidate pages.

The resulting `PreparedView` holds the exclusive storage guard, allowing a future
adapter to consume its already-validated request immediately before committing.
There is no callback or implicit pending-token consumption in this source. Its
typecheck is **not covered** by the std-only fixture or the previous Game CI.

## Integration conditions, not implemented assurances

| Condition | Concrete obligation before activation |
| --- | --- |
| Opening | A permitted client-only Game operation must install the initial View and retain the returned owner; it cannot claim an existing title/content match. The role, real attachment and all restoration paths still need actual coverage. |
| Stable content identity | Bind the consumer's BookId and source anchors to that owned opening. Do not translate legacy title markers while they remain routing inputs. Existing BookId/anchor helper tests are not live binding proof. |
| Locale/reload | A prepared rebuild must bind both locale and the actual resource/catalog input generation. Same-code reload must invalidate old preparation. No proved catalog-generation API exists here; strings, `lang::code`, pointer guesses or a new counter are insufficient. |
| Pending reply | Keep the shared RequestTracker in the native owner context, not a second registry. Validate channel/envelope/decoder/book/request/localization, then run the approved observation phase and revalidate afterwards. Wrong/stale/malformed input must not consume the latest valid request. |
| Critical section | After all callbacks, obtain a fresh stamp, prepare the exclusive slot guard, consume the exact validated pending work, then commit once. Token-consumption failure drops the guard without changing the slot. Locale/actor inputs require their own proved stability across this section; the slot borrow alone does not freeze process-global language. |
| Revision | Request-time page revision is not the opening. Page turns while waiting are allowed; a mutation between final observation and preparation rejects the commit. Native replacement/reset invalidates the owner even with identical contents. |
| Legacy LAN | Keep one-byte/nine-byte request compatibility, existing retained-overlay updates, cover/clamp/history and decoder semantics. Existing max envelope is 262,152 bytes, above the read-only observer's 262,144 cap. This source neither raises that cap nor maps generic Horizon mod support to a new Guide protocol. |
| Duplicate replies | At-most-once consumption would intentionally change legacy repeated-application behavior. It requires a reviewed consumer/protocol connection, not an assertion that this storage guard already implements it. |
| Callback/Drop | Revalidate after observers; dispose returned old pages only after releasing mutable context. Any later panic/effect is outcome-unknown, not rollback. No automatic retry. |

No actual locale, actor, RequestTracker, payload observer or LAN state is part of
the std-only test fixture. The design specifically avoids claiming that a passed
slot model establishes those boundaries. Language rebuilds and network replies
may eventually share a retained owner, but use different content/locale/request
facts and must not overwrite newer work merely because the opening matches.

## Proposed acceptance sequence

1. Independently review this exact source. Run its integrity guard without Rust.
2. With a separately granted CI slot, compile `fixture.rs` once using `rustc --test`.
   Inventory is the original seven storage tests plus ten `ownership_tests`.
   Run only the ten exact new names (10 passed / 7 filtered) with source/binary
   seals and the normal disk guard. They exercise generic Rust values, not Game.
3. Separately append `owned.rs` and `book-view.rs` to the original generated
   helper in an isolated official experiment. Preserve all original ten caller
   rewrites and pristine checkout. Compare full metadata and typecheck actual
   client-lib/Game lib/bin, with updated exact output hashes. Do not reuse the
   previous successful 11-file checksum as if this extension were already tested.
4. Prove the permitted native opening, catalog invalidation and pending-context
   integration, then ordinary Pluto/consumer whole-plan composition. Finally run
   real Client/overlay/close/reopen/reload/delayed-reply acceptance. Until then,
   production remains unchanged and no screen/LAN completion is claimed.

### Concrete native opening seam still missing

At official `624c4f62`, `game/books.rs::open_book` (lines 262–267) first stores the
screen, then clears keys, clears the right-button state and calls `capture(false)`.
Its complete source SHA is recorded in `inputs.json`. Current TConstruct `open`
builds pages, increments the legacy request sequence, clears back-history, calls
that native open function, then sends the unchanged hand/sequence request when
remote. Replacing only the slot assignment would not preserve these semantics.
Calling native `open_book` after `try_install_owned` would itself create another
opening through native `set`, revoking the newly minted owner. No such two-step
adapter is proposed here. A later actual Game opening operation needs a single
installation point plus the native tail, exact composition guards and explicit
failure/side-effect ordering, before the consumer can retain this capability.

The prepared tests cover identical reopen, native restoration, cross-slot owner
rejection, revision-only navigation, attachment reset, cancelled preparation,
identity/revision exhaustion, destructor reentry and post-commit panic. They are
not executed merely by publication. The review/test status is recorded alongside
the exact source hashes in `inputs.json`.
