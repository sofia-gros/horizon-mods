# Explicit request correlation

`horizon_client::pluto_inventory::guide::requests::RequestTracker` is a pure,
owned helper for asynchronous results whose `RequestId` is carried in-process.
It is part of the one existing shared API crate. It adds no native state, hooks,
registry, subscription, network send, screen update or activation.

## Contract

- `new(owner, channel)` validates two exact namespaced labels using the existing
  payload API's rules and 256-byte bound. A label is not an authenticated sender,
  a claimed channel or ownership of a screen. New trackers are inactive.
- `restart()` drops earlier work and allocates a fresh process-local scope from
  one checked atomic allocator. It does not open or identify a native resource.
- `issue()` supersedes the previous pending request. A token has private scope
  and sequence fields, can be copied/moved with a result and cannot activate a
  tracker. Independent trackers do not reuse scopes, including identical labels.
- `consume(id, owner, payload)` requires the exact pending token and labels and
  the existing 262,144-byte payload ceiling. It clears pending before returning
  the same borrowed Payload, with no byte copy. A duplicate/stale result cannot
  consume a newer pending result. Label, size and stale failures leave a valid
  pending request untouched.
- `discard(id)` drops only the exact pending result without granting an effect.
  Use it when a matching result arrives while the caller's required context is
  unavailable; do not automatically replay that result when context returns.
- `invalidate()` revokes pending work. Both counter exhaustion paths fail closed
  rather than wrap. Moving a tracker preserves its state; it is not Clone and
  has no serialization or persistence contract.

The caller must separately validate untrusted result content and obtain the
appropriate synchronous native capability before any effect. Successful
correlation is not proof that a book is still open, that input was admitted,
that an actor or connection is the same, or that a gameplay operation is allowed.
No native event automatically calls restart/invalidate. The library does not
infer ownership from a title, marker, pointer, entity number or matching bytes.

There is deliberately no token encoding, envelope, packet decoder or old-guide
protocol compatibility claim. Consumers needing external request/reply messages
must agree and validate their actual protocol separately; this API does not
silently reinterpret existing TConstruct bytes. TConstruct still owns all
content, page layout, metadata, item and recipe decisions.

## Relationship to the lifetime experiment

The renderer-owned state and nine invalidation hooks in
`experiments/guide-lifetime` remain excluded from ordinary packaging. That frozen
experiment is evidence about its exact candidate, not a consumer of this new
API and not another normal registry. Any future native adoption must use one
reviewed state/identity service rather than install competing implementations.

Normal PlutoLib includes only this explicit correlation value. The current-book
service remains synchronous, and the existing payload observer remains read-only
and observer-ordered. No unverified activation, UI update or transmission is
turned on by adding the API.

## Verification scope

Nine tests execute the actual new API and five existing payload tests still
pass. They cover inactive construction, label/size validation, nonreused scopes,
latest-request and one-time consumption, unchanged borrowed bytes, explicit
discard/invalidation, independent trackers, movement and exhaustion.

A fixture passed against the actual official CustomPayload type and the
unchanged native packet projection at source `26585d1e`, official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`. Source/lock/path identity checks also
passed, with one actual packet test and no disk stop. It explicitly carries IDs in-process,
checks old/new/duplicate results, unchanged packet contents and borrowed byte
identity. It does not invent an envelope, connect to a server or exercise the
native observer's Game callback phase. Its exact commands, hashes and result are recorded in
`guide-request-correlation-validation.json`. The complete ordinary package proof also passed at
`921bb12dc23caea0c970c5bcc39b72921e98981c`: actual plan, all eleven mandatory
guard groups, apply and client lib/bin typecheck (71 seconds). The package
archive is `c265b47f22e0abd8e8603c7ac4ea38a6c88dccb9b0c1f681b34856e2fa967874`,
with 5,043 sealed generated files. The 187-test Python suite passed, including
its invocation of 115 passing API tests. The isolated lifetime experiment is
absent from both archive and manifest; existing native modules, hooks and states
are unchanged from the previous ordinary package. This closes only the normal
correlation API composition gate, not the separate native lifetime guarantees.

Real screen lifecycle, reopening, respawn, delayed network delivery, input and
audio remain unproved. No approved full Client+renderer fixture/resources are
available; the role or rendering gates are not weakened for a positive result.
See `guide-lifetime-feasibility.md` for the outstanding native guarantees.
