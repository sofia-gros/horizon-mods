# Guide session and reply lifetime: bounded feasibility audit

## Result

The current official pair supports the individual Mixins entry points needed
for a **client-only candidate**, but a safe end-to-end guide session has **not**
been implemented or established. No session carrier, input hook, request sender
or reply application is installed by this audit. The existing Document/Text,
read-only packet observation and synchronous current-book services remain
independent and unchanged.

Official source cutoff: Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`, Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. The companion
[proof record](guide-lifetime-feasibility-20261105.json) contains exact source
hashes, the mutation/reset inventory and the complete no-op probe manifest.
Only official Horizon/Mixins and horizon-mods were inspected.

## What official events do and do not identify

1. `observe_screen` returns when the trait-mod list is empty. Otherwise it
   observes at most once per frame and compares only screen ID/menu. Closing and
   reopening the same `book_view` between observations can be invisible. These
   events cannot establish exact per-opening ownership.
2. `Remote::native_epoch()` identifies a client level, not a book opening or a
   replacement player. Same-key Respawn keeps both the epoch and ChunkMap.
   Its allocator uses `fetch_add`; it is not a checked exhaustion mechanism for
   a separate strict nonreuse contract.
3. `native_client_level_unload` is mutable and is called during reconfiguration.
   A dimension change reports new-level load before old-level unload; a library
   must not mistake that order for an already usable newly opened guide.
4. `native_client_disconnecting` is read-only. The mutable `leave_server` entry
   exists, but can begin closing-save and return before discarding the world.
   Revoking there is safely conservative, not an exact native close event.
5. Native `Screen::Mod` has a real mod-owner association. Its same-owner open is
   a no-op and supplies no new opening generation. It also has different pause,
   rendering and input behavior. Encoding a nonce into the owner string is not
   a proven substitute for a generation API or a drop-in native book.

The old TConstruct wire format does echo its request sequence. That is useful
correlation within its current assumptions, but its counter is ChunkMap-local,
wraps on increment, and is not revoked by every native close/player reset.
Its root marker, title prefix and equal page bytes are content, not authority.
This audit does not change that protocol or claim the consumer is recovered.

## A possible capability, with an exact bounded validity contract

A future library service could own an ephemeral record on the current client
ChunkMap, with a **process-wide checked nonreused opening nonce** and a checked
request sequence. A level epoch is an additional consistency check, never the
sole identity. The record must contain only owned data safe to move/drop with
the map. No pointer-keyed side table, cached entity ID, inferred ownership from
text, or serialized/reloaded live handle is acceptable.

The proposed validity interval is intentionally narrower than the native book's
lifetime:

- Start only after an explicit library opening operation completed and the
  current client, level and supported read-only non-lectern presentation were
  revalidated. Observing a matching native book must never create a capability.
- Revoke before any native open, close, save/take, command-prompt restoration,
  connection entry/exit, player/dimension reset or level unload. Map replacement
  drops the record; a conservative map-clear revocation also covers empty clear.
- These entry invalidations can revoke even when a native operation returns
  without changing the book. This loss of availability is part of the proposed
  contract, not an undocumented claim of exact native close timing.
- Exclude server, guest and fake-player contexts entirely. Client execution role
  is fixed at construction; server parked-world restoration is outside scope.
- A retained book under an overlay is not proof of admitted input. Each operation
  must recheck the client/renderer/player, title/screen, dialog/prompt/save/reload,
  full active loading and reconfiguration conditions. Async replies additionally
  require the current Remote to have a level and a matching native epoch. Do not
  substitute `remote_loading()`, which filters active loading by reason. A reply
  arriving while unavailable is rejected rather than saved for later automatic replay.
- A reply must match the current opening nonce, current pending request,
  registered owner/channel, current level and supported presentation in one
  synchronous operation. Consume a request before invoking any effect so a
  duplicate cannot apply twice. Newer requests supersede older ones. Exhaustion
  must stop issuing identities, not wrap or reset into a previous identity.
- Tokens correlate this presentation with this request. They are not secrets,
  server authentication, or permission to make a gameplay change. Page layout,
  cover policy, book/item identity and whether navigation supersedes a content
  request remain explicit consumer choices, never TConstruct rules in PlutoLib.

This describes a candidate, not production code. No trace/model test is being
substituted for native open/close, input or delayed-network execution.

## Native mutation/reset coverage required

The full source/path matrix is in the proof record. Important bypasses of a
simple open/close pair are:

- RunCommand takes a book, calls close, then a canceled prompt restores it by
  direct assignment. Restricted library Text excludes that command action, but
  restoration must still revoke rather than resurrect any old capability.
- `save_book` takes the option before checking Edit. `ContainerClose` can clear
  a book directly while dismissing a dialog. A supported current book must be
  checked every time; absence alone must not later grant ownership again.
- Lectern network updates directly change pages and clear the book. Lectern and
  editable books are not admitted by this candidate.
- Guest/fake-player use takes and restores book fields. Role/actor gates exclude
  these paths, rather than pretending their book is the window's guide.
- Same-key Respawn calls `enter_remote_dimension` without replacing ChunkMap or
  the level epoch. Reconfiguration emits level unload before the next Login.
- Disconnect/transfer returns from `poll_remote`, so its remaining old queue is
  not processed as the next connection's queue. The adapter must retain this
  ordering and must not move borrowed packet data onto an unrelated callback.
- Native page links/turns mutate the retained view's page without a new opening.
  Mouse, keyboard, IME and gamepad OSK have different admission paths. This audit
  adds no generic input interception or promise to own those events.

Other trusted mods can write private native fields from injected code. This is
not a sandbox. A supported package would need exact source drift checks and a
whole-plan composition policy for all ownership-changing entries and the reply
route. Unreviewed replacement/interior/cancellable hooks that can skip an
invalidation or alter the checked presentation must fail closed. The existing
TConstruct response HEAD is also incompatible with the official observer-first
transport guarantee until consumer recovery removes/reviews it.

## Actual feasibility evidence

A read-only plan with no-op helpers completed with exit 0 in 11.09 seconds:

- Eight mutable Game HEAD bindings: `open_book`, `close_book`, `save_book`,
  `answer_command_prompt`, `join_server`, `leave_server`,
  `enter_remote_dimension`, `native_client_level_unload`.
- One HEAD binding at `ChunkMap::clear`.
- One explicitly initialized owned `Option<(u64, u64)>` state on ChunkMap's
  actual Default constructor.

The probe wrote only its plan/logs, not a transformed host. Minimum free disk
was 10,933,063,680 bytes, above the continuous 10 GiB +128 MiB floor. It proves
planner acceptance of those exact bindings and primitive state layout, **not**
a richer session record, installed composition, borrow/type correctness of a
real adapter or gameplay behavior.

The rejected direct Game-state probe remains separate:
`E_BRIDGE_TARGET: generic bridge owner types are unsupported`. It is not evidence
that every Game constructor is untransformable. No constructor body was copied.

## Required changes and stopping point

No official source change is yet proven necessary for the narrow candidate.
The remaining work would be a grouped PlutoLib service, not another consumer
patch: one owned presentation/request state contract, atomic open/revoke and
conditional reply capability, the existing bounded Document codec, exact native
invalidation adapters, and mandatory complete-plan guards. It must preserve
`native_client_packet` observation before any reply effect; the existing
read-only subscriber API has no Game capability and cannot silently gain one.
A generic versioned request/reply envelope would need sender/receiver agreement;
it cannot silently reinterpret the old TConstruct payload. TConstruct keeps
content, metadata schema, page layout and item/recipe rules.

Before installation, prove identical reopen, direct restoration, same-key
Respawn, dimension change, reconfiguration, early closing-save, reconnect,
duplicate/out-of-order replies, hidden replies, exhaustion, no-consumer behavior
and two simultaneous clients against the actual adapter. Also prove whole-package
apply/typecheck and guard rejection of conflicting hooks. A modeled counter
alone cannot close these requirements. No session implementation should be
published as safe merely because the no-op plan succeeds.

For a broader stable official API, the concrete improvements would be a real
per-presentation generation/owned handle, exact replacement/close notifications
(including identical reopen and direct restoration), a client player-reset
lifecycle event independent of level replacement, and conditional presentation
operations tied to those identities. Supporting owned state on generic Game in
Mixins would simplify storage but would not itself supply those semantics.
These are proposed upstream API improvements, not changes made by this audit.

Until that proof exists, the verified useful boundary remains bounded Text,
observer-ordered read-only payloads and the synchronous expected-current-book
page-turn capability. Durable guide response lifetime remains explicitly open.

## Isolated experiment follow-up

The requested bounded experiment is retained outside the installed native bundle
in `pluto-lib/experiments/guide-lifetime`. Eleven pure state tests and twelve
guard tests pass. A separate complete library-plus-experiment plan, all mandatory
guards, apply and client lib/bin typecheck passed at library `529ad722` plus
experiment `1d3eb02f` on the same official pair. This proves the actual owned
state and native adapter type composition, not live callback execution.

There is still no native activation, packet subscription/send or screen update
path. Real Client/window resources are unavailable; no role/renderer gate was
weakened to simulate success. Identical reopening, player reset and delayed
reply cases executed only against the candidate state logic, not a live UI or
network. Durable end-to-end ownership remains open. The experiment README and
validation record separate every passed, rejected and unrun stage.
