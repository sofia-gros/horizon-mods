# Synchronous current-book capability: design only

This is a smaller alternative to a persistent guide session. No adapter, state,
registry or input hook is installed by this document. The inspected target is
Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` with Mixins `e8cd0aa3`.

## Proposed limited contract

A library-owned native adapter could lend a current-book capability only for
one synchronous invocation. Its public facts would reuse the existing bounded
`guide::document::Document` and report the current page. Its first operation
could compare an expected semantic snapshot against the current book and invoke
one native page turn. This is current-value compare-and-apply, not ownership.

- Initially support only the local client's non-lectern `BookScreen::View`.
  Editing/signing, lecterns, unsupported Text styles, invalid presentation page
  indices and documents beyond the established bounds fail explicitly.
- Preserve the one existing Text converter and metadata as uninterpreted data.
  Snapshot equality would cover supported span semantics and current page, not
  Text's private source-component cache, native object identity or a world token.
- An expected snapshot can be checked now even if it was obtained earlier, but
  identical-content reopenings must not be advertised as distinguishable. This
  contract is unsuitable for accepting delayed network replies.
- Revalidate immediately before the operation. No await, retained Game pointer,
  caller-supplied marker authority, global actor map or generation claim.
- Preserve the native page-turn route. `books.rs::turn_page` bounds movement and
  plays `minecraft:item.book.page_turn` even when an edge prevents movement.
  Directly writing the page field would silently change those semantics.
- Reject or separately review native hooks that can intervene between the
  comparison and native operation. Verify the post-operation facts before
  reporting the result, without claiming rollback of arbitrary foreign hooks.
- Keep open, replace, close, editing, signing, lectern actions, sending packets,
  stale-response matching and durable ownership outside this first capability.
  TConstruct retains its covers, two-page spreads, chapter history and geometry.

A public borrowed trait or one-shot callback can hide Game/BookScreen from the
consumer. It need not create a persistent registry. The native adapter's entry
still needs a precisely documented invocation boundary; a caller must not infer
that inspecting a stored book proves that the book currently owns window input.

## Input admission must remain native

For a future input adapter, prefer an invocation that the native book handlers
already received, rather than duplicating the entire window dispatch predicate:

- Keyboard: window routing calls `book_key`, which ignores releases, gives
  editing shortcuts their existing precedence, then calls `book_input`.
- Controller on-screen keyboard: `pad_menu.rs::pad_osk_output` independently
  selects chat, sign, then book. It sends text/backspace/enter, not just the
  physical PageUp/PageDown cases.
- IME: `ime.rs::ime_commit` filters control characters, runs native character
  callbacks and requires a focused text field before reaching `book_input`.
  A read-only-view-only capability should not reinterpret this as editing.
- Mouse: the window path sends only a pressed left button to `book_click`.
  Native click geometry and page-link processing are distinct from keyboard
  input. A common API must not install TConstruct's geometry as the native rule.

These are native book-handler invocations, not evidence of a permanent screen
owner. Direct calls by control/native code remain distinct from a physical user
input event. A callback that performs a page action must not then allow the same
native input to perform it again; an eventual command/result contract needs to
settle that consumption rule explicitly.

## Existing native lifecycle APIs are insufficient as ownership proof

`native_hooks.rs::observe_screen` returns immediately when the native Mods list
is empty. Its change detector compares only screen ID and menu, and runs at most
once per frame. Two different or reopened view books both use `book_view` and can
therefore produce no close/open notification. This cannot back a per-open token
for source-only adapters.

The generic native screen-input hooks deliberately exclude books and other
screens over gameplay via `screen_hooks_open`. Reusing those hooks would not
cover the book input routes above. `remote_loading()` is also not an all-loading
predicate: when a level load is active it returns true only for Reason::Other;
portal loading has a different presentation. It must not be used alone to prove
that a stored book owns input.

The larger owned-session design still needs the full reset matrix in
[guide screen lifecycle audit](guide-screen-lifecycle-audit.md). It remains
unimplemented. This smaller proposed capability removes private host access for
bounded operations only; it does not replace that lifecycle work.

## Proof required before implementation is considered ready

1. Finalize the precise synchronous entry and result/consumption contract,
   including unavailable versus unsupported versus stale expected content.
2. Plan the actual latest-official module/entry bindings without a Game state
   extension. Keep any registry shared and default-off if one becomes necessary.
3. Test current-value comparison and all native View/Edit/lectern distinctions
   against the real installed types. Do not substitute a look-alike BookScreen
   and describe that as native validation.
4. Guard the actual native turn operation and prove audio/no-movement behavior
   and no double input consumption. Package application/build is necessary but
   does not by itself establish these runtime outcomes.
5. Complete these proofs as a guide feature group before consumer migration.
