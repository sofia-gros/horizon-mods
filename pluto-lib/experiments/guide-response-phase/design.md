# Feature-preserving guide reply connection: design and official constraints

## Status and scope

This is an excluded research checkpoint, not an installed reply service. The
TConstruct LAN request, host reply and client response paths are unchanged.
No mutable callback, page update, session activation, transport or relaxed
package guard is enabled. Navigation repair remains separate draft PR #945.
The ordinary PlutoLib runtime proof remains v14, source `b19b35a1`.

Official cutoffs are Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. Only official sources and
horizon-mods were inspected. The two no-op probes and exact source hashes are
beside this document; `validation.json` separates acceptance from rejection.

## Demonstrated extension point

At the official cutoff, `Game::handle_packet` calls
`native_client_packet(&packet)` before matching the packet. The latter calls
the original native observer once. Ordinary guide CustomPayload packets reach
this route. The two direct observer calls in `poll_remote` handle forgotten
chunks and decoded chunk packets, not guide CustomPayloads.

A standalone RETURN probe accepting `&mut Game` and the borrowed packet passes
the official planner. It proves only target/binding acceptance. A direct
AFTER_CALL probe at the caller's observer invocation fails with
`E_UNSUPPORTED_CONTROL_FLOW`. Do not work around that rejection by copying the
large caller, re-observing packets, or weakening the guard.

The candidate extension therefore belongs inside the **one existing Pluto-owned
RETURN**, after original native dispatch. A second RETURN is deliberately
incompatible with the current exclusive observer guard. Changing that one
helper's signature, source hash and mandatory complete-plan contract requires a
new coherent reviewed group; this checkpoint does not change them.

## Proposed operation and borrowing contract, not an implemented API

The proposed consumer-facing concept is a synchronous reply callback with
borrowed packet facts and an optional, non-cloneable presentation-update
capability. It does not expose mutable Game, a general closure over Game,
packet cancellation, arbitrary native method calls, transport or gameplay
authority. Final Rust names and signatures remain open until the content and
presentation contracts below are settled.

One invocation must follow this order:

1. Finish original native packet observation exactly once.
2. Snapshot eligible registrations once, before any subscriber. Keep one
   registry/service owner, deterministic ID order, duplicate-ID rejection and
   no locks held while calling consumers. Registration changes take effect on
   the next invocation, not between the read-only and effect phases.
3. Deliver existing read-only observations under their unchanged limits.
4. For an explicitly opted-in effect registration, validate the current
   client/actor/world, owned opening, pending request and retained presentation
   **after** those callbacks. Owner/channel are selectors backed by a proven
   opening record, not inferred from a channel string or text marker.
5. Decode/admit a bounded proposal without changing the game. Revalidate at
   the commit boundary. A matching, valid attempt consumes its pending request
   before any effect; subsequent calls return an explicit already-used result.
   Malformed, oversized, wrong-channel and mismatched proposals preserve pending
   state unless an independently detected lifecycle invalidation revokes it.
6. Apply only the admitted replacement to that presentation in the same borrow.
   Do not reopen, close, navigate, play a page-turn sound, send a packet or
   invoke another observer. Report attempted/outcome-unknown separately from a
   reversible precondition rejection. A later error is not a rollback promise.
7. Return normally so official packet handling still runs. The current guide
   channel has no special native handler and reaches the ordinary fallback;
   reserved native channels must retain their existing handling.

No opted-in consumer means no additional presentation snapshot, session
activation, body conversion or mutation. Borrowed packet/capability values cannot
be retained as an automatic later callback. This is trusted-mod composition,
not a sandbox against arbitrary injected Rust.

## Compatibility requirements discovered in the existing consumer

### Full legacy envelope size

`source-materials-book-wire.rs::encode` permits a body of 262,144 bytes.
`encode_reply` then prepends an eight-byte sequence, so a valid reply can be
**262,152 bytes**. Pluto's read-only `client_payload::MAX_BYTES` is 262,144 for
the whole observed payload.

A direct rewire through the old observation ceiling would lose the last eight
bytes' worth of valid payload sizes. Do not truncate, silently lower the
consumer's body limit, or silently broaden every read-only subscription. A
future effect registration needs an explicit bounded legacy-prefix/body
contract, with checked length arithmetic and admission before copy/decode.
Keep the existing read-only ceiling unchanged. The existing nine-byte request
(hand plus u64) and echoed eight-byte field can remain wire-compatible if a
nonreused opaque request identity is introduced; the host need not interpret
that field. This is a proposal, not a new transport format or serialization of
the current private RequestTracker.

### The two document formats are not interchangeable

The consumer uses an array body with `insertion`; Pluto Document uses a
versioned object with `metadata`, and its constructor checks the size of a new
canonical encoding. Legacy decoding is permissive about missing fields and
normalizes unsupported metadata/links and some style defaults. Pluto conversion
rejects unsupported styles instead of discarding them.

Consequently `Document::new(from_legacy_decode(...))` is not an established
equivalent path. A compact valid-shaped legacy JSON body can expand far beyond
the Document serialization limit when explicit null style fields are added.
The regression here is a source/JSON-shape model, **not execution of the Rust
codec or native Text**. It establishes a required compatibility case, not a
claim that all old server output hits that case. Actual legacy encoder/decoder
and native converter fixtures must prove the selected semantic/budget boundary
before activation. Reuse the existing span/style conversion where equivalent;
do not duplicate a component schema or silently change accepted content.

### Retained presentation is not input admission

The old response handler updates a retained read-only, non-lectern book even
under an overlay. The current-book page-turn API intentionally requires admitted
input context. The lifetime experiment drops hidden replies. Neither is an
equivalent LAN response adapter unchanged.

A safe equivalent service needs identity of the **retained** presentation
without claiming it currently receives input. It must prove retained-view
refresh under overlays and prevent refresh after replacement/restoration,
same-key player reset, reconnect or an identical reopen. Do not disable remote
refresh to avoid this proof, and do not weaken the existing page-turn input gate.

### Cover, spread and refresh policy stay in TConstruct

The consumer preserves `usize::MAX` for its cover and otherwise clamps the
page to the last new page. Pluto's ordinary current-book Snapshot rejects that
sentinel correctly. A replacement contract must not normalize the cover away,
admit arbitrary unchecked native page positions, or transfer Tinkers' cover,
spread/history or title/root schema into PlutoLib. A separately reviewed
presentation position policy is needed. Page-turn audio must not be emitted by
a content refresh. Local generated guides can also exceed the wire subset, so
a new owned-open API must not silently remove their existing local presentation.

## Opening/request ownership remains the blocker

The existing ChunkMap-local wrapping sequence, equal page bytes, title/root
marker, pointer or native level epoch cannot prove an opening. The inactive
lifetime experiment has checked nonreuse and explicit invalidation, but no
native activation/send/update route and no real Client/window execution.

An explicit opening operation would need to establish ownership only after the
native opening and final view validation, without another hook's identical
replacement being mistaken for the owned opening. New requests must supersede
older requests, checked counters must fail closed on exhaustion, and map
replacement, same-key player reset, direct command restoration, close/save,
connection changes and level reconfiguration must revoke correctly. Native
input/navigation may change the page without granting a new opening.

Full lifecycle path inventory is in
`../../docs/guide-lifetime-feasibility.md`; do not replace it with a marker or
polling shortcut. The mandatory whole-plan guard must reject unreviewed hooks
that bypass those boundaries. No full approved Client/renderer resource tree
or real-window fixture is available, and no renderer/role gate was bypassed.
The existing experiment's pure tests and separate type proof do not establish
live UI ownership.

## Required proof and next group

Before removing the old cancellable response HEAD, require a grouped service
with source/type/borrow proof, exact observer ordering and no-consumer behavior,
single registry snapshot/reentrancy tests, legacy body/envelope boundary tests,
cover/clamp/metadata equivalence, and lifecycle cases including identical reopen,
same-key reset, delayed/duplicate/out-of-order replies, overlay refresh, direct
restoration and two clients. Test native packet handling still occurs exactly
once. Re-run complete package guards, apply and lib/bin checks on the immutable
combined cutoff; isolated target acceptance is not enough.

PR #945 already removes the global turn_page collision and keeps LAN refresh.
Its focused helper and isolated actual plan/guards passed, but its native
apply/type check is still queued and real input/audio is unverified. The old
response HEAD remains an explicit full-consumer composition blocker. Nothing
in this checkpoint marks the consumer merge-ready or disables that feature.
