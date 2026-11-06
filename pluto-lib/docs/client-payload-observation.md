# Read-only client play-payload observations

This is a guide-support boundary, not a guide protocol or request/reply service.
For the additive event-style entry and explicit startup example, see
[guide API usage](guide-api-usage.md). The legacy functions remain supported.
The single `pluto_inventory::client_payload` registry accepts an explicit
subscriber ID, exact namespaced channel, maximum observed byte count and function
pointer. The consumer owns payload interpretation. Registration grants no
ownership of that channel or authority over a server, actor, world or screen.

Subscribers receive borrowed channel/byte slices synchronously, in lexical
subscriber-ID order. The registry lock is released before callbacks; registration
changes apply to the next dispatch snapshot. Duplicate IDs are rejected rather
than replaced. Names/channels are bounded to 256 bytes. A consumer can select
0 through 262,144 bytes; zero observes only empty payloads. Larger or unmatched
payloads receive no notification but continue through the native handler. This
ceiling is not a protocol admission rule. No payload is cloned, and an empty
registry or unmatched channel allocates no callback storage.

Bytes remain untrusted remote input. The API exposes neither a Game reference
nor send, cancellation, gameplay action, screen replacement or request-matching
operations. It does not supply a connection generation, authenticated sender,
owner token or stale-reply guarantee. A consumer must not treat a payload's own
claims as any of these. Callbacks are trusted in-process code, not a sandbox;
they must return promptly without panicking. The adapter does not catch callback
panics or schedule asynchronous work.

## Native order and scope

The client-only adapter uses a RETURN on the existing official
`Game::native_client_packet`. Original native mod observation completes first.
The new projection then observes only the `CustomPayload` variant, before the
ordinary `handle_packet` match handles it. It never calls the observer again,
cancels the packet, substitutes its bytes or prevents native channel handlers.
This covers only packets that reach this play-phase observer. The official
network-thread/configuration exclusions remain unchanged, including brand,
keep-alive, resource-pack, cookie and transfer traffic. Other observed callers
of this native hook currently handle chunk/forget packets and do not produce
custom-payload notifications.

The native projection is separate from the phase hook so an actual protocol-type
fixture can test it without constructing a window or mocking Game. Such a
projection test does not prove packet routing, screen behavior or LAN delivery.

## Required installation guard

Run `tools/check_client_payload_plan.py COMPLETE_PLAN.json
PRISTINE_OFFICIAL_WORKSPACE` before application, through the supported common
preflight integration. It checks exact official source hashes, native observer
fingerprint, resolved source node, helper bytes and bindings. Alternate target
spellings cannot evade the same-function checks. The native observer target is reserved
for this reviewed RETURN; any other hook there requires a new composition
review. Caller HEAD/RETURN hooks may coexist only when they are noncancellable;
interior hooks and cancellable caller HEADs fail closed. In particular, the old
TConstruct guide-response HEAD on `handle_packet` is incompatible because it
can bypass the official observer. It must not be installed alongside this API
until the consumer is migrated coherently.

Raw launcher application without these complete-plan checks is unsupported.
The guard addresses generated hook ordering, not arbitrary native code that
manually calls methods or invokes the public adapter entry out of phase.
Official source changes require a reviewed adapter/guard update, not a silent
hash refresh. The official target remains Horizon `624c4f62` and Mixins `e8cd0aa3`.

## Validation status

Five pure registry tests pass: exact matching and bounds, deterministic order,
unchanged borrowed bytes, unlocked callback removal and poisoned-registry
failure. Seven source/packaging/guard tests pass, including rejection of old caller
cancellation, foreign observer hooks and source/helper/binding drift.

The actual official Mixins planner resolves the production RETURN helper and
bindings, and its source/order guard passes (one-injection probe, no apply).
One fixture test against the real official protocol and packaged API also passes:
custom versus other packet/channel projection, exact borrowed bytes, unchanged
native packet and subscriber removal. The fixture rechecks clean official source,
lock identities and local dependency paths; copied helper/API bytes match the
production source. Installed full-package hook validation remains pending.
Earlier Text tests and v6 package results do not cover this follow-up.
There is no native screen/session carrier, server guest handler or outbound
transport in this group.

The payload group through `c3f07e2c` subsequently passed full actual library
plan/preflight/apply and client lib/bin typechecking in central cutoff
`fb96dd82` (106 Python and 73 API tests). An initial build was disk-stopped;
the same sealed 5,036-file input passed on its cached retry with no source change.
This establishes installed composition, not live packet/LAN or guide session
behavior, and does not cover the later current-book candidate.
