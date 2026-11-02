# Part Chest and Tinker's Chest

This feature is in progress. It targets the original TConstruct revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` and is not yet implemented.

The original Part Chest accepts only tagged parts, keeps at most eight identical item-and-component variants per slot, forbids duplicate variants in separate slots, and has up to 256 slots with a growing visible inventory. The Tinker's Chest has 64 slots of up to 16 items each. Their original shaped recipes use a wooden chest, pattern and planks; the Part Chest uses wooden rods and the Tinker's Chest uses lapis.

The implementation scope is registration, source recipes and appearance, inventory rules and scrollable UI, persistent storage and destruction behavior, and use as an adjacent station inventory, including client/server and LAN transactions. Existing adjacent vanilla chest behavior must remain intact. Exact insertion, removal, shift-click, filtering, and source chest interactions will be checked against the source before implementation.

Representative acceptance will cover natural crafting and placement, limits and component identity, rejected items and full inventories, manual and shift-click transfers from adjacent Part Builder and Station, save/reload, destruction and recovery, LAN host and guest access, and an ordinary chest regression. No new heavy build or game test has been run for this unit.

The shared source-specific slot limits, acceptance decisions, and growing Part Chest display size are modeled and lightly tested. They are not yet connected to a placed block, menu, transfers, or saving.

The existing machine placement and block-entity serialization hooks now recognize the two intended chest IDs and their slot counts; no chest block is registered yet. The generic `BlockEntity::for_block` hook was considered, but its `Self` return prevents cancellation under the current Mixins compiler, so this implementation uses the existing concrete placement and save boundaries. Placement, persistence and interactions still require game validation.

The original chest blockstates, models and block textures are copied from the pinned source under its MIT license (`third_party/tconstruct/LICENSE`). They are not yet connected to registered blocks or tested in the game.

The shared insertion primitives now retain complete item components, use the source chest's per-slot limit, reject Part Chest duplicate variants and non-parts, and return any overflow. Their direct-slot and shift-move behavior has lightweight coverage; actual menu and server call sites are not connected yet.

Both block and item declarations and their original shaped recipe ingredients are present, along with source model files and block drops. The menus and transaction hooks are not connected; the draft must not be treated as usable until those paths are complete and verified. The Tinker's Chest uses the source legacy block-entity ID `tconstruct:modifier_chest` on save.

The chest-screen cell geometry follows the source's eight columns, four rows and 12-pixel scrollbar. A common position/hit calculation covers the final Part Chest slot, but the screen and scrolling events are not connected yet.

A first local chest screen connection uses the source blank background, original scrolling grid, source item tags, and the shared pickup/shift-move rules. Unsupported gestures are currently blocked; the guest and adjacent workstation paths remain unconnected. This is a draft implementation, not yet transformed, built or tested in the game.

A draft LAN direct-chest path projects the visible 32 slots into the existing 54-slot transport and validates normal/shift clicks on the host, including full component identity. It uses window-scoped page state and rejects virtual slots and unsupported gestures. Client/server integration has only a limited Mixins plan so far; no transform, type check or game test has been run. Adjacent-workstation access remains unconnected.

The pinned source calls `addChestSideInventory()` for the Part Builder and Crafting Station. Its Tinker Station does not call that side-inventory path; do not silently describe a new Tinker Station side panel as original behavior. The shared Part Builder result transaction is being extended to respect chest limits without changing ordinary chest rules.

A draft Part Builder side-inventory connection now uses a six-column paged view and the same chest insertion limits for local and LAN normal/shift transactions and built-part quick moves. The code has passed a limited Mixins plan and pure projection/rule tests only; it has not been transformed, type checked or game tested. Crafting Station side access, the remaining click modes, and complete recovery/visual behavior are still being implemented.
