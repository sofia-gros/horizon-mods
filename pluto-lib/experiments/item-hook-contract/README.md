# Item behavior on newer Horizon: design probe

This is a design and a small decision test, not a working native bundle. The released experimental [item behavior guide](../../docs/item-behaviors.md) still describes the older verified host. The candidate here is Horizon `b2e71aca04c7f62f24eefd7dd0324bae9c04954f` and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`; it has not been built or run as a demo.

## One short use

The existing handler operates on a copy of the stack's mod-owned integer data. A Pass discards edits and continues standard behavior; Handled commits them and consumes the use; Cancel discards them and returns failure without a success swing. The official server-side `native_use_item` and `native_use_item_on` callbacks receive the stack and a `claimed` flag. `claimed = false` can preserve Pass; `claimed = true` consumes the use with a swing and can represent Handled. The flag cannot express the existing Cancel feedback. `decision.rs` returns an explicit error for that case: it is a contract probe, not an installed hook or a fallback that silently changes behavior. A native result/feedback hook or a narrowly scoped, verified injection is needed before the existing API can be migrated faithfully.

Block use includes the target position and face in `native_use_item_on`; `native_use_item` does not provide a target. The block receives its own use first. LAN guests can send separate use-on-block and item-use requests, so dispatch needs a defined once-per-action policy. Do not call the existing single handler once from each hook without resolving duplicate dispatch. Passing `claimed` also has to preserve claims made by other mods.

The proposed consumer context uses library-owned values: acting player identity when available, dimension and position, hand, and an optional block target with face. Do not expose `Game`, `ItemStack`, or host block types. `native_acting_player` can expose the swapped-in actor and position, but its implementation depends on acting-player state; availability for local host and all long-use callbacks must be checked. The use hook provides a stack snapshot; writing the selected hand requires a host adapter that identifies the actual hand, validates the same stack and updates it on the authoritative side. The existing adapter's `plutolib:data` compound preserves foreign custom data and can be reused. Do not commit staged data or world effects on Pass or Cancel. The current handler only stages data; a future world-action interface must also delay effects or restrict outcomes so it can keep that guarantee.

## Holding and releasing

`native_use_duration` provides a duration from the stack. `native_living_entity_use_item_start`, `_tick`, `_stop`, `native_on_use_tick`, `native_release_using`, and the server-side `_finish` cover different stages. Stop may skip release; a hand change may end a use without a stop callback. The start, tick, stop and release hooks run on both sides, and their own documentation says there is no helper that reliably distinguishes the authoritative world from the predicting client. Until an authority boundary is proven, these callbacks must not mutate saved data or world state. Claiming a normal use also returns before the host starts its mod-item long use, so short-use and hold dispatch cannot simply share the same claimed mapping.

## Sync and compatibility

The current transport bundle uses the host's standard custom-data path with narrow standard component supplements. Reuse it rather than installing the old item's cancellable host/client packet adapters alongside it. Verify the exact actor/hand write, foreign data preservation, save/reload and host/guest update before treating this as supported. Tinkers and other native hooks are unverified. The old `pluto_items` bundle and wand demo remain intact for the previously verified host.

A minimal next implementation should take the existing wand as its only consumer, cover server-side short use and tooltip, and retain all three outcomes. Add long use only after the authority and lifecycle gaps are resolved. The current prototype and its test do not register an item or exercise the game.
