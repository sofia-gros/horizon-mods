# Guide source boundary: latest official snapshot

This is a read-only comparison from official Horizon `624c4f62` to
`f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`. The coordinator's corresponding
latest Mixins cutoff is `5a64af938ec86b00d53d16a408aabc0038fb3e57`;
this document does not claim a Mixins application or compatibility check.

The existing `751999ea` isolated typing success and the reviewed, untriggered
`aa235fe7` owned-opening candidate remain **old-target evidence**. They are not
latest-official proof. No ordinary manifest, runtime, fixture, LAN behavior or
CI trigger was changed by this audit.

## Exact scope and useful result

The prototype has ten existing source targets and one added helper. We retrieved
only those ten existing files through the official GitHub connector, compared
their complete bytes to the pinned local official source, and verified their Git
blob SHA-1 and sizes against the nontruncated latest official tree. The proposed
`game/retained_book.rs` helper is absent from that tree. Per-file hashes, exact
URLs, 22 replacement-count checks and four function-body comparisons are in the
[machine-readable receipt](guide-latest-official-source-audit-20261106.json).

- `books.rs`, `ime.rs` and `saving.rs` are byte-identical. In particular,
  `BookScreen::View`, open/close, page turning, keyboard/click/draw and lectern
  behavior have no direct source change in `books.rs`.
- Seven existing targets changed. The old exact source guard must reject them;
  changing only the pin would not be a valid port.
- All 22 old literal and player/connection transition replacement counts still
  match within these ten files. No obvious storage-expression rewrite is forced
  by this bounded comparison. This is not a patch application or typecheck.
- `capture`, `reset_inventory_clicks`, `native_client_packet` and command-prompt
  restoration bodies are byte-identical. Native opening still has a meaningful
  input/mining/capture tail; installing an owner and then calling native open
  again would revoke that owner's opening and is not a valid adapter.

## Changed input and overlay boundary

The controller code now prioritizes prompts independently of the underlying
screen. `pad_in_menu` includes `prompt_open`; `pad_navigation_mode` gives prompts
focus even over pointer-driven book screens; `pad_all_targets` does not add
underlying menu targets when a prompt is open. New prompt-back and typing paths
also participate in controller routing. The prompt hash is not a durable book
identity or owner token.

Command-prompt drawing now calls `widgets::cover_frame()` before declaring its
widgets. The cancellation path that restores the saved book is unchanged, but
unchanged storage is not evidence of identical focus, overlay or input behavior.
Any future active connection needs new prompt/controller/cover cases rather than
reusing the old lifecycle claims.

## Packet phase and limited test visibility

The latest `handle_packet` still calls `native_client_packet` before its ordinary
packet match. Its four recognized custom-payload branches follow that call.
The existing Pluto RETURN observation point remains after native mod observation
but **before ordinary packet handling**, not after all native processing. It
does not grant mutation, response authority or retained-screen ownership.

The `native_machines` delta exposes its existing test module and `server_game`
factory as `pub(crate)`. That factory still requires prepared resources and is a
server fixture. This is not a newly proven real Client/window route.

## Required next steps and exclusions

The old direct-field inventory covered 19 files, not just the ten rewritten
ones. Of its other nine files, seven have different latest blob IDs and two are
identical. This audit does not renew that full inventory or rule out new callers.
Before proposing a latest patch, review those files and new direct-field/player/
connection paths, then regenerate a separate exact patch and validate it against
the new immutable target. Keep the old prototype, fixture and results unchanged.

Matching bodies do not prove unchanged callees, renderer/assets or scheduling.
No new Rust, metadata, plan, apply, Game typing, gameplay, language-reload or LAN
execution was performed. Existing LAN remains enabled and unchanged. Normal
Pluto composition, owned Game opening, delayed response commits and retained
translated redraw remain separate, unproved integration boundaries.
