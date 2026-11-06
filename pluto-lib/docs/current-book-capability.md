# Synchronous current-book inspection and page turn

This candidate removes direct BookScreen access for a narrow presentation
operation. It does not introduce a guide session, native state extension,
registry, input hook or automatic activation. TConstruct content/layout remains
unchanged; this is not its consumer migration.

The host-independent contract is `pluto_inventory::guide::current_book`.
The private native entry is
`game::books::pluto_current_book::with_current(game, callback)`. It lends an
immutable entry Snapshot and a borrowed PageTurn capability for one synchronous
callback. Neither exposes Game/BookScreen to the callback. The capability cannot
escape the borrow, and at most one native turn may run during that invocation.

## Scope and conditional action

- A Snapshot uses the existing bounded Document converter and current page.
  It preserves supported span/opaque metadata semantics, not private native
  Text source-component identity. Metadata is not interpreted as an owner ID.
- Only a local client's non-lectern read-only View is supported. The context
  gate conservatively excludes missing player/renderer, server/guest action,
  title/other screen, prompts, sign, dialog, pending entry/save/reload, integrated
  startup, active remote level load and reconfiguration. This is a limited
  stored-book capability, not proof that a physical input event was admitted.
- No book, edit/sign view, lectern, unsupported document and invalid/out-of-range
  page are distinct errors. A custom cover sentinel such as usize::MAX is not
  normalized into a page. Broader TConstruct presentation needs separate work.
- `turn(expected, direction)` compares the expected semantic document/page to
  both this invocation's entry and the current native book immediately before
  acting. Equality is a current-value precondition, not ownership, identity or
  stale-response protection. Identical reopenings are not distinguished.
- An accepted action calls native `turn_page` once. Its native bounds and sound
  route remain intact, including a page-turn sound when already at an edge.
  The capability is consumed even if the page stays unchanged. A second action
  fails before another native call.
- The adapter rechecks the observed result. Errors after native invocation are
  explicitly OutcomeUnavailable/OutcomeChanged, not claims of rollback or of
  no effect. Precondition failures invoke no native turn.

There are no open, replace, close, edit, sign, lectern, send or response-acceptance
operations. No click geometry, title prefix, source book identity, cover choice,
chapter history or two-page layout is built into this API. It has no input
subscription: a future consumer that uses it inside a native input handler must
explicitly prevent that input from applying the same turn again.

## Required source/composition guard

The supported package preflight must run
`tools/check_current_book_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE`
before apply. The guard verifies the installed adapter bytes/placement and exact
reviewed official source, and rejects every other hook on native `turn_page`,
including alternate target spellings identified by function fingerprint. This
prevents an injected action from changing the book between comparison and the
native turn. Foreign arbitrary code is not sandboxed by this guard.

The original TConstruct page-turn hook is incompatible until consumer recovery
removes/reviews it. Raw application that skips this preflight is unsupported.
No native body is copied or replaced. Source hashes must not be refreshed without
review of changed context/page behavior.

## Validation limits

Four pure contract tests pass for changed document/page, expected entry mismatch,
native-style bounded page proposals, and rejection of invalid/cover sentinel
pages. Five guard/source/package tests pass. These tests do not execute Game,
page-turn audio or the borrowed native operation.

Three tests against the real private BookScreen are written in the installed
module, not run yet. They check native View projection and Edit/lectern/invalid
page/unsupported action rejection. They are in the gameplay crate test target,
not the small client-core/protocol fixture. The actual full library plan, mandatory guards, application and client lib/bin
typecheck subsequently passed at central cutoff
`c55f4985001999b7ea9aa34f7470e8725a5684af`. The grouped gate also passed 130 Python
and 93 API tests. Its archive is
`47dcc0d20c79beb6a592db4385b729e1019e87de0c752cac4430a658ea4e4f61`, with 5,039
sealed stage files; the native check took 84 seconds. This is the current-book
candidate's own installed composition proof, not an earlier Text/payload proof.

The three private projection tests and real native turn/no-double-operation/
audio/input proof remain unrun. The official headless Game fixture pattern uses
prepared resources and IntegratedServer role; it does not satisfy this API's
client/renderer gate unchanged. Do not bypass that gate or describe a modeled
state-machine test as native client operation evidence.

See [the design and input audit](current-book-capability-design.md) for the
separate durable-session and input ownership gaps. This limited API does not
resolve them and is not yet claimed ready for consumer adoption.
