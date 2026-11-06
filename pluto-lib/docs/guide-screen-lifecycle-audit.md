# Guide screen lifetime: unresolved native adapter

This source audit concerns official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`.
It introduces no session state, screen hooks or input interception. The portable
document/Text group can be installed independently.

The actual Game state probe fails with `E_BRIDGE_TARGET` because generic bridge
owners are unsupported. No large Game constructor is copied to work around it.
ChunkMap is a supported owned state carrier, but its lifetime is not identical
to the active window's book lifetime. A first-span marker, matching content,
entity number or a pointer is not sufficient to establish ownership.

## Observed paths

- `game/execution_role.rs` defines a fixed construction-time Client role.
  Client Game instances do not themselves simulate single-player worlds.
- `game/books.rs::open_book` replaces the native book, clears keys/right-click
  and releases capture. `close_book` clears it and restores capture. A library
  service would have to observe both, including identical-content reopenings.
- `book_follow_link` can take a view book for a command confirmation;
  `remote_chat.rs::answer_command_prompt` can restore it directly. Restricted
  Pluto documents exclude those command actions, but this is still a native
  replacement path that an ownership contract must explain.
- `save_book` takes editable books. `remote_lectern_closed` clears a lectern
  view directly. `remote.rs` also clears books on a server ContainerClose while
  a dialog is active. None is equivalent to every caller using close_book.
- `native_machines.rs::native_use_as_player` temporarily takes/restores the book
  while acting as a fake server-side player. `lan_actions.rs::as_guest` and
  `leave_guest` likewise take/restore the host's window book around guest use.
  A naive ChunkMap record changed by guest close hooks could invalidate the
  wrong window. Wider server/guest coverage is not supported by this design.
- Server `levels.rs::swap_level` swaps whole worlds and parked state. Its
  `enter_level` returns early for Client role, so a strictly client-only service
  could avoid that server lifecycle, but cannot claim server window support.
- Client connection/disconnection, world closing and a changed remote dimension
  replace World/ChunkMap. A stale handle must not revive in a later world.
- `remote.rs::enter_remote_dimension` can retain the World/ChunkMap for a
  same-key respawn while replacing player and loading state. Resetting only
  when ChunkMap is constructed therefore misses a session transition.
- Title/loading/dialog/pack/command overlays can hide or supersede input to a
  retained native book. `game.rs` draws books after ordinary screen content,
  suppresses them under a pack prompt, and dispatches book input separately.
  A stored book is not by itself proof that its window currently owns input.

A possible narrow adapter would be client-only, explicitly invalidate native
replacement and same-world respawn paths, and use a nonreused per-open session
nonce plus owned state and current-screen validation. A monotonic nonce must
fail closed on exhaustion; it is identity, not gameplay authority. That proposal
is not implemented or proven. In particular, exact reopening/overlay semantics,
source-drift checks, composition guards and stale response behavior need evidence
before a session registry or packet-driven screen update is installed.

## Follow-up feasibility result

The [expanded lifetime audit](guide-lifetime-feasibility.md) records the exact
official event/reset inventory and a successful read-only plan for nine
invalidation entry points plus one primitive owned state. This does not install
a carrier or prove native session/reply behavior. Durable ownership remains open.
