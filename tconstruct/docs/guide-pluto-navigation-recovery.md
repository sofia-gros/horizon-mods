# Guide navigation and PlutoLib composition

This repair is based on `31cf76672d3a7b982aebab86a91a78440afd163e`, with official
Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. It does not read or depend on a Horizon
fork. It is a partial consumer repair, not completed TConstruct integration.

## Local navigation

PlutoLib's current-book API deliberately provides one ordinary native page turn,
with a checked expected document/page, native edge audio and no persistent
ownership claim. TConstruct has a different cover (`usize::MAX`), two-page spread
and return-history policy. Calling the ordinary API twice would not reproduce
that policy, would play two sounds and would not admit the cover.

The consumer therefore keeps this policy in `source-materials-book-navigation.rs`
and calls it directly from its existing marked, read-only, non-lectern guide
handlers. No hook replaces official `Game::turn_page`:

- ArrowLeft/ArrowRight and A/D retain existing behavior.
- PageUp/PageDown are handled before native input, preserving the spread behavior
  previously supplied by the global page-turn hook.
- The guide's existing mouse arrow hitboxes call the same local operation.
- Official `book_control("next"/"back")` also calls `turn_page`. A guide-only
  adapter retains spread behavior and the native JSON readout without reentry.
  This also prevents an ordinary native increment of the guide cover sentinel.
- Unmarked books, editing, lecterns, close/sign/take/query actions and all other
  keys retain their native paths. Each consumed turn clears guide link history
  and plays exactly one page-turn sound, including a reached edge.

The root marker remains only the existing local presentation discriminator. It
is not a session identifier or evidence that a reply may update a screen. No
input admission, lifetime or renderer guarantee is added by this refactor.
Native source inspection found direct turn callers in `book_input`, `book_click`
and `book_control`; this statement is pinned to the official revision above.

## Remote refresh remains unresolved, not removed

The existing LAN request/send/response paths are intentionally unchanged.
`materials-book-response` still cancels `handle_packet` before the official
`native_client_packet` observer. The mandatory Pluto payload composition guard
continues to reject that full combination. Passing a navigation-only overlay
must never be reported as a full TConstruct plan, build or release pass.

Equivalent recovery needs all of the following rather than a guard exception:

1. A reviewed post-native-observer mutation boundary, after exactly one official
   observation, with explicit native/Pluto subscriber ordering. The current
   payload API is read-only and offers no mutable Game or UI authority.
2. A proven client opening/request capability revoked on every relevant close,
   replacement, same-content reopen, same-key player/world reset and disconnect.
   The existing wrapping sequence plus marker/title match does not establish
   that ownership. Pluto's pure RequestTracker is correlation, not native life.
3. A synchronous conditional application that rechecks current role, visible
   context, opening capability and pending request before decoding/applying
   bounded pages. Stale, duplicate, hidden and exhausted requests need explicit
   semantics. Preserve the original host-authoritative page generation and
   client view clamp rather than replacing them with locally generated content.
4. Actual Client/renderer and delayed-packet tests for those transitions, plus
   official plan/apply/type checks and complete composition guards. The isolated
   lifetime experiment and modeled state tests are not this proof.

No queue, lifetime experiment, transport rewrite, automatic subscription or new
screen mutation capability is activated by this navigation repair.

## Verification boundaries

The production helper fixture uses a modeled Game, marker and audio counter,
with real serde_json for the control result. It checks navigation policy and
exactly-once helper invocation, not physical input, audio playback, native Font,
window ownership or live LAN behavior. The native input/renderer evidence
remains unrun. Source/manifest regressions require the unchanged Pluto guards to
accept the navigation boundary and reject the preserved remote conflict.

At source `1c47a378c64082668b4fbeb392dc3371f24f912f` (production helpers unchanged
from `e4fcdee7`), all eight modeled production-helper cases passed, with zero
failed/ignored cases. Twenty-seven source/manifest/guard cases passed, including
mandatory preflight routing and the unchanged remote-conflict rejection.

The actual official planner accepted a **navigation-only consumer overlay**
(two modules, three input/control hooks and one existing history state) together
with the complete verified v13 Pluto package. All eleven mandatory guards passed
on that complete selected plan. Terminal exit was zero, with minimum free disk
10,895,065,088 bytes and no floor stop. This is not a full TConstruct plan.
No generated host stage, apply, native typecheck or real Client was run for this
repair. The current official host stage was left untouched.

The full original consumer-plus-library plan attempt produced no diagnostic or
plan artifact after several minutes and was interrupted (execution terminal
exit 130). It remains **inconclusive**, not a demonstrated planner rejection or
pass. The preserved response-hook conflict is independently demonstrated by
source/guard regression; full integration still cannot be promoted. No hook or
feature was disabled to force the isolated pass. The first helper fixture also
had a missing modeled Font method; its compile failure was preserved, corrected
only in the fixture, and the eight cases rerun successfully.

See [the validation receipt](guide-pluto-navigation-validation.json) for exact
source/package identities, logs, command scopes and remaining exclusions.
