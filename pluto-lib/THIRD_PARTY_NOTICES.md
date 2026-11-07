# Third-party notices

`mixins/pluto_native_creative/inventory-advancements.rs` adapts the inventory
advancement trigger from [Horizon commit 624c4f6265c02296fbe8a80f029dd0d987fb94c0](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/advancements.rs).
Copyright (c) 2026 Ludovic Marie and the Horizon contributors, MIT license.
The helper retains the complete license notice. Changes add a complete-component
inventory key and scoped metadata supplied through PlutoLib's generic API.


`mixins/pluto_native_creative/advancement-tag-index.rs` adapts the native
`Game::inventory_index` cache from the same official Horizon commit and file.
Copyright (c) 2026 Ludovic Marie and the Horizon contributors, MIT license.
The helper retains the complete license notice. Changes supply resolved owner
snapshots and retain world/snapshot identity alongside the native tree cache.
