# Guide LAN compatibility review before actual Game typing

This is a source-only review. No local source, generated stage, normal package,
LAN route or production guard was changed. No new test, Cargo metadata, copy,
host check, UI or packet execution was performed for this review. The proposed
storage patch remains excluded and unapplied. Actual Game typechecking is
waiting for capacity and the coordinator's serial slot.

Reviewed official Horizon: 624c4f6265c02296fbe8a80f029dd0d987fb94c0.
Reviewed Mixins: e8cd0aa38cffef49cb2e088ef716baa107148c02.
The four TConstruct files below are byte-identical at ordinary MOD runtime
b19b35a10b96e398ca8dabcb46b200d88950edaf and
f5a83539525c26fd2142e26e43789a3dd0107033.

## Preservation and intentional corrections are different

Feature-equivalent recovery means preserving a valid reply's effect on its
current owned guide, including retained overlay behavior and consumer page
policy. It cannot mean reproducing every instruction of the old response hook:

- The old materials-book response HEAD returns Break for every packet on its
  channel, including rejected replies. This bypasses the official
  native_client_packet call at the start of handle_packet. A safe connection
  must restore original observation and native handling, not preserve that
  bypass or replay the observer later.
- The old decoder checks the current sequence and book label but never consumes
  the sequence. A matching duplicate may update pages repeatedly. A proposed
  at-most-once reply capability deliberately rejects duplicate commits.
- Equal titles/root markers and a wrapping ChunkMap sequence do not establish
  a particular opening, player attachment or connection. Rejecting replies
  after an identical reopen, restoration or same-key attachment reset is a
  deliberate lifetime fix, not removal of valid current-opening LAN updates.

These are requirements for a future implementation, not changes activated here.
No mutable subscriber or replacement response handler has been installed.

## Wire and acceptance contract to retain

The current opener increments the sequence, clears guide history, installs
the local generated book at the cover, then sends a request if remote.
The request is exactly one hand byte (0 or 1) plus a big-endian u64. The server
reads the guest's current selected main-hand or offhand item and only serves
one of the six recognized guide items.

The server also retains a one-byte legacy request branch, which returns an
unprefixed body. The current client sends nine bytes and its decode_reply
requires an eight-byte echoed prefix. This is not peer-version negotiation or
proof of compatibility with arbitrary older clients/servers. A new in-process
request token may use the existing opaque echoed u64 only with the same wire
encoding and a proven mapping to the owned opening. Private RequestId internals
must not be serialized.

The response body is limited to 262,144 bytes. The eight-byte prefix makes a
valid complete reply up to 262,152 bytes. The ordinary read-only observer's
262,144-byte whole-payload ceiling cannot simply become the reply body's gate.
No truncation, lower body limit or silent increase of every observer's ceiling
is a compatible fix. Native delivery of the complete maximum packet remains
unexecuted in this research.

The existing decode_reply first requires a recognized current book label,
matching sequence, admitted body and matching decoded book label. Its decoder
requires 1..512 pages, at most 32 wire spans per page and at most 4,096 UTF-8
bytes in each text field. Insertion metadata is retained only within 8,192
bytes and for the consumer's known prefixes/root/title marker. Page links must
be positive and within the decoded page count. Invalid links or unsupported
metadata are omitted; malformed required text rejects the body.

The encoder and decoder are intentionally not assumed symmetric. The encoder
checks page count, root marker and serialized body bytes, but does not enforce
all decoder span/text/metadata limits. Thus successful encoding alone is not
proof that the old client accepts the result. Nor is converting to the stricter
bounded Pluto Document automatically equivalent: canonical encoding size,
metadata handling, optional style values and unsupported styles differ.
Actual Rust codec and native Text fixtures are still required. The source
review does not silently broaden or tighten the existing accepted subset.

## Retained refresh versus input, opening and revision

The old response does not require the book to receive input or be visible
above a dialog. It updates an existing non-lectern View; native dialog opening
and clearing can retain that book. A future capability must admit a valid
retained refresh without reopening the book, dismissing the overlay, navigating,
playing page-turn audio or granting input authority.

ContainerClose is distinct: the native dialog branch clears the book, while a
plain retained non-lectern book is not always removed. A canceled command prompt
restores its saved book directly. The proposed slot intentionally gives such a
restoration a fresh opening, even when the same value or identical pages return.
It must not revive pending replies for the earlier opening.

The proposed storage helper conservatively advances revision on every as_mut
borrow, including a rejected or no-op access. Therefore a request must not
require equality with a revision captured when it was sent. The excluded
Replies model instead binds the opening/attachment and samples the current
revision at synchronous reply preparation. Commit rechecks that revision and
the pending request after validation. This permits ordinary navigation between
send and receive, while rejecting a changed presentation between prepare and
commit. That model has not yet been connected to native storage.

The old response preserves usize::MAX as the cover; otherwise it clamps the
current page to the last decoded page. It does not restore the page at send
time. These cover/spread and history choices remain TConstruct policy, not
generic Pluto behavior. A retained refresh must not reuse the input-only
current-book page-turn gate unchanged.

## What actual Game typing can and cannot establish

The planned pristine host check can verify the real Game field, BookScreen,
Text/EditBox types, all production borrow sites and thin client binary against
the eleven-file proposal. It can expose missing Option operations or native
borrow/type incompatibilities that the seven generic helper tests cannot.

A successful check alone cannot prove complete mutation interception, ownership,
native lifecycle order, packet delivery, retained overlay behavior, actual audio
or renderer effects. The raw native mutable projection can still be misused by
arbitrary trusted Rust to replace the entire enum. Observation stamps are not
owner handles. Existing Mixins and future mutation routes require separate
whole-composition review. No official patch or stable public ownership API is
claimed here.

The ordinary v15 dependency graph differs from pristine through two bundled
API crates and path dependencies. The pending metadata/capacity gate must
assess actual package identities, dependency edges, features and output budget;
matching a logical path is not a cache-reuse guarantee. Ordinary guards stay
pinned to their original host sources and must not be weakened to certify this
experiment.

## Evidence

All paths below are relative to horizon-mods:

- tconstruct/mixins/tconstruct/source-materials-book-response.rs:
  5216f1404439880c54db043e7c463ea54ce224264909198d3ab42e8301a4b647
- tconstruct/mixins/tconstruct/source-materials-book-request.rs:
  f9117c9290cdab963b938fea30d445b49fbe22b4d5d9e2690f8cf3713bee073f
- tconstruct/mixins/tconstruct/source-materials-book-wire.rs:
  960087432e9f9c071797f67b1d411c781dcc0cdafeb7dce57a7ebfbe8b9032d7
- tconstruct/mixins/tconstruct/source-materials-book.rs:
  3e31953570b3bd42d1e07501b4bb081d4325321fc1190fccc7e5095c86ea7c96

Official source hashes and exact mutation inventory are already recorded in
official-sources.json. Relevant native paths are game/books.rs (open/close,
save/take and lectern paths), game/dialogs.rs (retained overlays),
game/remote_chat.rs (direct canceled-command restoration), and game/remote.rs
(packet observer and ContainerClose). The previous model and helper results
remain in their own validation records; this review adds no executed test count.
