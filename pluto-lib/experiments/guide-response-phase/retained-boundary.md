# Retained guide lifetime: minimal host boundary and excluded reference model

## What is implemented here

`retained_model.rs` is a **standalone excluded Rust model**, not a host hook or
ordinary Pluto API. It reuses the actual `RequestTracker` and `client_payload`
source modules. There is no second registry, native sender, screen mutation,
renderer/input shortcut or modification of the installed package.

The model implements a proposed retained slot identity and revision contract,
an explicitly selected legacy-u64-prefix envelope profile, and a conditional
prepare/commit path. Its generic value is **not native BookScreen/Text**. The
test consumer chooses cover preservation and ordinary-page clamping; these
policies do not become library behavior. Compile/model results belong here,
not to the ordinary v14 runtime or an official Client/UI proof.

## Smallest stable official extension identified

The official host has a private `Option<BookScreen>`. `open_book` replaces it,
but several paths take, clear or restore it directly. A generic screen-change
observer cannot supply exact identity, and native level epoch does not identify
a replaced player. Merely exposing more mutable Game bindings is insufficient.

The smallest **semantic** extension consists of these operations, rather than
copying a full caller or exposing all Game state:

1. A retained-book slot with checked, process-nonreused **attachment** and
   **opening** identities, plus a checked content/position revision. A fresh
   connection or player attachment changes the former even for same-key respawn.
   Every replacement/restoration changes the latter even for identical bytes.
   Ordinary navigation/content mutation changes revision, not opening identity.
2. An explicit owned-open operation returns an opaque handle only after the
   final installed presentation and client context are established. A read-only
   snapshot of an arbitrary current book never grants ownership. Owner labels
   and echoed request numbers are neither authentication nor gameplay authority.
3. A synchronous retained-view inspection and conditional content-update
   operation require that handle plus a current revision. They distinguish
   **retained** from **input eligible**. An overlay may retain the view and permit
   a checked content refresh without receiving book input, changing the overlay,
   playing turn audio or reopening the screen.
4. All clear/take/replace/restore paths go through the slot boundary. There must
   be no unrestricted mutable dereference exposing the raw Option. Otherwise
   newly added direct writes can silently bypass generation/revision updates.
   A thin wrapper is preferable to a side counter that every caller remembers
   to increment. Exact implementation/ABI and upstream acceptance remain open.

These are **proposed upstream semantics**, not APIs present at the pinned host.
No upstream repository or official source was changed. Supporting Mixins owned
state on generic Game would make storage easier but would not itself establish
these lifecycle semantics. A mod-only implementation could still be possible,
but the present event probes do not prove exact retained lifetime; it must not
be labelled equivalent solely because a no-op plan accepts its hooks.

Counter exhaustion disables new asynchronous ownership/correlation. It must
not prevent ordinary native book opening or destroy a locally visible book.
No pointer-address identity, text marker or equal-content recovery is used.

## Path distinctions confirmed in official source

The exact source hashes and locations are in `retained-source.json`.

- `open_book` always replaces `book`, while close/save/RunCommand clear or take
  it. `save_book` takes before it checks whether the removed value was Edit.
- Canceling a command prompt restores `Some(*book)` directly, so an old token
  cannot be revived by reusing a saved object.
- Showing/clearing a dialog changes dialog state but retains the book. In
  contrast, ContainerClose while a dialog is open directly clears it. Do not
  invalidate every overlay or assert every ContainerClose removes a plain book.
- Page links/navigation mutate the retained view in place. A reply requested
  before navigation should use the page at reply/commit time, not force the page
  from the request back onto the user. Native Edit and lectern handling remain
  outside the proposed guide ownership capability.
- Same-key Respawn can reuse the world/map/level epoch. A separate attachment
  identity must invalidate ownership. Disconnect, reconfiguration and changing
  dimension also need exact revocation before a reply can commit.
- LAN guest/fake-player code takes and restores screen fields around server
  work. This service is limited to the actual window client, not those contexts.

The model represents these event outcomes explicitly. It does **not** prove
that the real host emits the events at the right time or that an adapter catches
all of them. The actual native connection is still missing.

## Pluto and consumer responsibilities

Pluto would own one registration/correlation service, borrowed event ordering,
bounded envelope admission, and the conditional capability over a proven host
handle. TConstruct retains book/item selection, title/root checks, legacy JSON
decoding, metadata policy, content generation and cover/spread/page-clamp rules.

The reference `Replies` model consumes a non-cloneable `OwnedOpening` produced
only by `install_owned`. The slot binds the owner at that operation; ordinary
installation/observation grants no handle. A stale handle fails after replacement,
even if another owner installs identical contents. This is modeled host ownership,
not a native capability at the current official cutoff. A
request maps a checked, process-nonreused **opaque wire u64** to the existing
private `RequestId`. That ID is not serialized or reinterpreted. The legacy
host already echoes the u64, so the proposed profile adds no version byte and
requires no server format change. `LegacyU64PrefixV1` is an out-of-band profile
name, not a negotiated protocol or an implemented new public envelope.

Length is checked against eight bytes plus the unchanged 262,144-byte body cap
before split/semantic decode. The body remains borrowed. Passing that body to
the unchanged RequestTracker preserves a valid 262,152-byte whole reply without
raising the read-only observer's ceiling or copying/truncating the packet.
Document/legacy decoder equivalence is still a separate unresolved requirement;
the model's tiny test decoder is not substituted for either native codec.

Prepare happens after original observation and the single registry snapshot's
read-only callbacks. It checks current attachment/opening, pending wire ID and
labels, then performs pure consumer proposal validation. It does not consume
pending state. Commit rechecks identity/revision and pending ID, consumes once,
and only then replaces the modeled value under one exclusive borrow. Rejected
content/size/labels and changed revision leave a valid pending request available;
actual lifecycle changes revoke it. An overlay does not itself revoke it.

The inert Pages/String tests do not exercise user-defined destructor effects.
For generic T, assignment drops the previous value, and its Drop can run code or
panic after the request was consumed. The model does not catch that panic or
promise rollback. A separate destructor-panic regression requires the pending
request to remain consumed and deliberately makes no rollback/value assertion.
The future native API must expose attempted/unknown outcomes where applicable
and prohibit automatic retry; the model's ordinary Result alone is not a safe
panic-recovery interface. No general mutable Game callback is proposed.

## Proof boundary and remaining work

Focused tests cover modeled overlay refresh/cover preservation, identical
reopen, take/restore, same-key attachment reset, navigation, changed revision,
newest/duplicate replies, malformed/mismatched replies, maximum legacy envelope,
separate clients, and exhaustion preserving local presentation. They execute
the actual shared correlation implementation but **not BookScreen, native
observer order, input/audio, renderer, network or lifecycle callbacks**.

Before activation, require actual official type/apply proof for the selected
host boundary, complete source/whole-plan drift guards, retained-content and
legacy-codec equivalence, and real Client tests for the lifecycle/overlay paths.
No approved complete Client/renderer resource tree has been established. The
old LAN path remains unchanged, including its known lifetime limitations and
its remaining observer-guard collision. PR #945's navigation overlay apply/type
check is separately queued; neither this model nor its tests complete that gate.

## Executed model result

Exact `31ba6150` compiled in 0.405 seconds; all 12 retained tests passed,
including destructor failure and explicit owned-open handle checks. Fourteen
compiled dependency tests were filtered out and are not added to that count.
Eight source/package/JSON-shape tests also pass. Before/after official-source,
reused-API and model hashes match; minimum free disk was 10,899,214,336 bytes,
above the unchanged 10 GiB +128 MiB guard. See `retained-validation.json`.
This closes only the excluded model proof, not the missing native/UI connection.
