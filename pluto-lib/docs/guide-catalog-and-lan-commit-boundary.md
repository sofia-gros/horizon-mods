# Guide catalog snapshots and legacy reply commit boundary

Read-only follow-up, official Horizon `624c4f62`. No runtime changes or new test
result. Exact source hashes are in the adjacent JSON. The reviewed ownership CI
remains frozen at `aa235fe7`; this document is not part of its frozen CI inputs.
There is no ordinary Guide owner or mutable packet capability installed here.

## Native language identity that can be retained

Official `core/lang.rs` already has one `CURRENT: RwLock<Option<Arc<Language>>>`.
`current()` returns an owned clone of the effective native Arc, preferring the
thread-local test scope when present. `set(Language)` constructs a fresh Arc even
when the locale code is unchanged. Holding the original Arc strongly and comparing
it with a later native Arc using `Arc::ptr_eq` can identify the same live catalog
allocation. The retained reference prevents its allocation from being freed and
reused while that comparison is relevant. This is different from caching an
unowned address or inventing a second catalog/generation counter.

This is a **candidate input identity**, not a commit authority or monotonic event
generation. `current()` releases the global lock before returning; a concurrent
`set` may run immediately after the comparison. Native `scoped` restores an older
Arc on exit, so effective snapshot equality does not mean no intervening scoped
change occurred. `BIDIRECTIONAL` is a separate process-wide atomic, written before
CURRENT, and scoped lookup does not scope it. Retained catalog identity alone
therefore does not prove coherent visual/bidi state or a resource/font snapshot.

The existing Key resolves through native Text once. Repeated native global
lookups during a page rebuild could observe more than one catalog unless an
actual stable-resolution boundary is proved. A later shared API may expose the
native snapshot as a read-only input, but should not pretend it holds a global
read guard, freezes resources or authorizes installing pages. No new resolver,
CURRENT, public setter or callback registry is proposed in this pass.

Required later cases include same-code `set` with changed strings, retained old
Arc versus newly effective Arc, nested scoped restoration, a catalog change during
page preparation, and a change after comparison but before commit. They require
actual native tests and a concrete synchronization/admission design; they are not
covered by the ownership guard's unique borrow of the **book slot**.

## Precise packet phase, not an ambiguous “after native”

Official `Game::handle_packet` first calls `native_client_packet`, then enters its
ordinary packet match. Pluto's existing read-only RETURN adapter runs after the
native mod observation call, **before the handle_packet match**, not after all
packet handling. Its contract and source already make that distinction. A future
reply connection at that same phase must preserve native observation exactly once
and must not rely on effects from the later native match having already happened.

The current TConstruct cancellable HEAD intercepts its Guide channel before that
native observation, which is why the complete-plan guard rejects the combination.
This document does not weaken the guard or move that handler. A new permitted
capability would need one reviewed dispatch/observation order, final context
revalidation after any callbacks, and a native composition proof. An accepted
mutable Game binding by itself supplies none of those guarantees.

## Compatibility obligations that remain separate from the slot owner

- The legacy host accepts one-byte and nine-byte requests; the latter echoes an
  eight-byte sequence. The current client requires that prefix and matching book
  identity. Do not infer that its decoder accepts old prefix-free replies merely
  because the host can emit them to an old requester.
- The body permits 262,144 bytes, yielding a 262,152-byte modern envelope. The
  read-only observer rejects whole payloads larger than 262,144 bytes before a
  subscriber runs. Slicing off the header inside a callback cannot recover an
  event that never reached that callback. Neither its cap nor RequestTracker's
  payload admission changes in this work.
- Decoder defaults and filtering remain the legacy consumer's policy. Strict
  Document conversion is not an equivalent decoder. Do not drop existing valid
  pages, title/root metadata, page links or graphics to make admission succeed.
- Valid replies currently update the retained non-lectern View beneath overlays,
  preserve `usize::MAX` cover state, clamp ordinary page indexes and leave history
  untouched. Input focus is not the admission test. The proposed actual-type
  preparation preserves these storage choices but is not connected to a reply.
- The legacy sequence uses wrapping arithmetic and replies may be applied more
  than once. The checked in-process RequestTracker has no wire encoding. Mapping
  between them needs an explicit connection/opening/request contract, including
  sequence reuse/overflow policy; simply casting one token into the other does
  not prove correlation. At-most-once would be an intentional behavior change.
- Localized replies need a negotiated format or an explicit compatible content
  strategy. Generic Horizon mod-message support is not Guide-version negotiation.
  Existing peers and the existing LAN path remain unchanged while this is unproved.

The next isolated ownership gate can establish generic replacement mechanics and
actual BookScreen typing. It cannot establish an admitted opening, current actor
or packet attachment, catalog/reload consistency, pending-reply consumption,
ordinary Pluto/consumer composition or live Client/LAN behavior. These conditions
remain visible rather than being hidden behind a convenience event or silently
disabled legacy feature.
