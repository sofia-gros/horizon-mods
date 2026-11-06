# Guide LAN connection: preserved behavior and remaining proof gates

Read-only audit at official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. The corrected Font fixture source is
`59c0651b324e5cd1f4445cf9c15e8bb0b4a086a9` (published equivalent
`c9bc56eeea60a6caf88de0b8b1055cc33a8155e0`). No packet, screen, lifetime or
native host change is installed by this audit.

## What the unchanged consumer actually does

- `source-materials-book.rs::open` increments the existing ChunkMap sequence with
  wrapping arithmetic, resets navigation history, opens a non-lectern view at the
  cover sentinel, and for a remote open sends one hand byte plus eight big-endian
  sequence bytes on `tconstruct:materials_book`.
- The host request handler accepts either that nine-byte request or its legacy
  one-byte form. It verifies the connected guest's indicated selected/offhand
  stack is one of the six guide items. The modern reply adds an eight-byte
  sequence prefix; the legacy reply is the body alone. This is an observed
  compatibility branch, not a negotiated version protocol.
- The current client requires the eight-byte prefix, matching sequence and matching
  guide identity from the title/root metadata. It updates an existing retained
  non-lectern view even beneath an overlay. Cover `usize::MAX` is retained; a normal
  page is clamped to the decoded last page. A matching reply can currently apply
  repeatedly because there is no consumed pending request.
- The body ceiling is 262,144 bytes; the modern whole envelope can therefore be
  262,152 bytes. Decode additionally admits at most 512 pages, 32 spans per page,
  4,096 UTF-8 bytes per text and selected metadata up to 8,192 bytes. Encode does
  not enforce every decode constraint. The legacy decoder's style/default/filter
  policy is not Pluto Document's strict schema.

Those source files are unchanged by the Font migration. Keeping them unchanged
preserves the existing feature; it does not establish lifetime safety.

## Why replacing the response hook is not just a callback move

Official `Game::handle_packet` calls `native_client_packet` before its packet
match. The current cancellable TConstruct HEAD consumes its channel before that
call. Thus it skips native packet observation. Pluto's mandatory complete-plan
guard correctly rejects this combination; the guard must not be relaxed merely
to accept the old HEAD.

The existing Pluto RETURN observer is read-only: it borrows channel/data, takes
one subscriber snapshot, drops the registry lock and notifies matching entries.
Its 262,144-byte whole-payload ceiling is deliberately smaller than the maximum
legacy reply envelope. Registering at that ceiling would lose valid edge-sized
replies before a consumer saw them. Raising it globally or truncating the prefix
would silently change the reviewed contract.

A previously accepted mutable RETURN binding proves only that the official
planner can express that binding. It proves neither a retained opening identity
nor authority to replace its pages. A new mutable subscriber at the existing hook
would require separate API/source/whole-plan review, not a second observer or a
bypass of the present exclusive-hook guard.

## Required boundaries, kept separate

| Boundary | Existing support | Still required for safe LAN connection |
|---|---|---|
| Pure reply correlation | Checked RequestTracker with borrowed untrusted bytes | Bind a pending request to an actual owned retained opening/attachment; do not infer identity from matching content, pointer, actor key or marker |
| Current-book operations | Synchronous inspect/conditional page turn under current input admission | Retained presentation refresh under overlays; current input eligibility cannot replace retained ownership |
| Native book storage | Official Option<BookScreen>; excluded wrapper prototype with helper tests | Actual Game/BookScreen type proof and explicit owned-open acquisition; a storage stamp by itself is not an owner capability |
| Payload phase | One reviewed post-native read-only observer | One coherent dispatch snapshot and bounded mutation capability, with revalidation after callbacks and immediately before commit |
| Envelope/body | Existing strict portable Document plus unchanged legacy wire code | Explicit compatible whole-envelope admission and consumer-owned legacy decoding; no forced switch to a different document schema |
| Commit outcome | Pure model consumes an exact pending token before a proposed effect | Actual native conditional update/drop ordering; errors after an attempted effect cannot imply rollback or a safe retry |

An excluded eleven-file native storage patch already records every reviewed
creation/replacement/take/restore boundary, including cancelled-command restore
and guest save/restore. It remains unapplied research, not an ordinary Pluto
adapter. Its pristine and v15-generated-source compositions are distinct. Future
actual Game typechecking must preserve their separate source seals and must not
weaken the ordinary package's pristine-source guards. Neither successful typing
nor helper-model tests alone prove live Client/window behavior.

## Acceptance cases for the eventual connection

1. Close and reopen identical content: the old reply cannot update the new opening.
2. Reset/reconnect with the same actor/world keys: old attachment/request identity
   cannot be reused. Pointer or tuple equality is insufficient.
3. Retain a guide under an overlay: a valid current reply refreshes its pages;
   closing the overlay does not restore stale pages. Input gating is not used as
   a substitute for this retained-slot contract.
4. Preserve the cover sentinel and ordinary page clamp for valid current replies.
5. Wrong channel/header/title, malformed body and oversized envelope do not consume
   the current pending request under the stated validation policy.
6. Native/registered callbacks cannot change the opening between validation and
   commit without a final identity/revision rejection. Request-time revision
   equality is not required merely because pagination or a conservative mutable
   borrow advanced a revision before the reply arrived.
7. Duplicate matching replies are consumed at most once. This is an intentional
   correction to legacy repeated application, not a claim of identical behavior.
8. A panic/error after effect attempt is outcome-unknown; it is not silently retried.

These cases are requirements, not newly passed tests. No existing LAN path is
switched off while the replacement remains unproved.

## Automated verification without a manual user step

A dedicated, exact-WIP-branch push-trigger CI route has been approved for the
corrected twelve-case Font/Text/CPU-atlas fixture, with read-only repository
permissions and fixed official source/lock inputs. The CI owner is preparing it;
this document does not claim it has run. Its cold runner must fetch the official
locked dependency set before offline fixture preparation, enforce the disk floor,
verify exact source/API identities and record every named result. It does not
require a Game, full asset tree, renderer/window or Mixins application.

That route addresses the current local-capacity blocker only for the named Font
fixture. A separate future automatic Game type experiment is technically a
different scope. Live screen lifetime/LAN tests additionally need a proven actual
Client fixture and approved resources; no such complete setup has been verified.
A headless/server fixture or renderer=None must not be presented as a positive
window/input test. No manual user action is being substituted for these missing
proofs, and no PR state, permissions or billing settings are changed here.
