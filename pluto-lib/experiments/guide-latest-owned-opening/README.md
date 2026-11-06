# Latest official retained-book candidate: excluded source only

This is a separate latest-target candidate, not a change to the ordinary Pluto
manifest, the existing LAN handler, old `751999ea` typing proof, or reviewed
`aa235fe7` old-target CI. There is no workflow or trigger in this group.

Official Horizon is `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`; the corresponding
Mixins identity is `5a64af938ec86b00d53d16a408aabc0038fb3e57`. The proposed native
source extension is not a Mixins-only mod and has not been applied or typed on
this target. It conveys no approval to edit the official repository.

## Fixed implementation and small delta

The original storage helper, `owned.rs`, `book-view.rs`, their ten-test fixture
and original generator are reused byte-for-byte, not copied into a new registry.
The same 22 reviewed literal/transition rewrites transform ten existing files;
the single new native helper consists of the three exact original fragments.
`generation.json` records nineteen input hashes, eleven changed output hashes,
and exact inverse-restoration hashes. `prototype.patch` is an unapplied artifact.
It is derived only from official source, with the original MIT notice retained
at `../guide-response-phase/native-patch/HORIZON-LICENSE.txt`.

The known-source forward patch is independently parsed in memory and compared
with the transform output. Its inverse restores all nineteen exact latest input
files. This preserves the new controller/prompt/cover logic rather than copying
old native bodies over it. It does not prove live behavioral equivalence.

## New input behavior is retained, not treated as ownership

- Controller back now handles `prompt_open()` before the underlying screen.
- Controller navigation/focus considers prompts independently of book pointer
  mode. Command prompts call `widgets::cover_frame()` before declaring widgets.
- Control-socket book edit modifiers now read the raw held-key state.
- Native book content/open/close/page logic and the capture tail are unchanged
  at their direct source boundaries. Capture still clears container interaction
  and stops mining; it must not be skipped or called through a second opening.

An overlay is not a close. The existing `PreparedView` accepts an owned retained
read-only, non-lectern view without claiming that current input is admitted.
It preserves the cover sentinel and clamps ordinary page indices. Native close,
restore or identical reopen revokes the old owner; mutable page access advances
revision. `prepare_owned_replace` holds the slot exclusively until one commit;
old values are returned for destruction after that borrow ends. None of this
connects an owner to a Game actor, a request, a locale snapshot or a network peer.

## Deliberately incomplete lifecycle inventory gate

We fetched the nineteen existing inventory inputs, using unchanged official
bytes where the latest Git blob is identical. Their direct book/player/remote
sites were compared; the latest game subtree nevertheless contains sixteen new
files and 133 changed files in total. The rest has not been source-enumerated
locally. The stored inventory is therefore **provisional**, not a claim that no
new caller exists.

`generate.py --official <fresh exact official checkout>` first validates the
official remote, clean revision and fresh lock. It then reads **every** current
game Rust file and `game.rs`, including files outside the nineteen known inputs.
Any new/moved/changed direct book, player or connection site stops with exact
paths. It never silently excludes or converts an extra site. Only after this
gate may it compare the known-source delta with the frozen patch and receipt.
The returned `full_inventory_verified` is false in the saved partial receipt
and becomes true only after that complete-tree gate. It is still a spelling
inventory, not alias analysis or a sandbox against arbitrary native mods.

## Verification plan and current limits

The ten Python tests cover source identity, patch/inverse facts, extra site and
actor reset rejection, changed line rejection and independent hunk context
failure. They do not execute Rust or simulate a real input device.

Future CI must run the full latest inventory gate before preparing any copied
experiment. It must use the new official lock and fresh dependency metadata,
then compare normalized pristine/experimental package identities and require
actual client-library, Game-library and binary compiler invocations. No old
cache or compiler receipt is a substitute. The unchanged Rust ownership fixture
still has seventeen listed tests and ten selected ownership tests, with seven
filtered. It is not yet executed. The actual `BookScreen` adapter also remains
untyped on this latest target.

Ordinary package composition, real owned Game opening, prompt/controller/window
execution, language reload and retained redraw, delayed/duplicate LAN replies,
and authority after native callbacks all remain separate unproved gates. Existing
LAN functionality stays enabled and unchanged.
