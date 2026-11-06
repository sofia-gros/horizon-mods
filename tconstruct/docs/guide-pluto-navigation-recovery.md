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

Actual read-only official planning, exact commands and terminal results are
recorded separately after the source is frozen. A full consumer plan may stop
on another existing compatibility error before reaching any guard; no hook or
feature is silently disabled to force a pass.
