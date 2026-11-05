# Worldbound explosion survival

Related to high-priority #379. Stacks on the lifetime/environment portion in #817.

## Implemented

Validated Worldbound drops survive native explosion destruction while retaining the ordinary explosion exposure calculation, knockback callback and movement. This does not remove them from the entity walk, change their item ID, impersonate a Nether Star or revive a deleted entity. Other items, mobs, players, guests, vehicles, decorations, loot timing and native callback order retain the original path.

The adapter is entered only for an authoritative game with at least one validated Worldbound drop and a known damage type outside the native `minecraft:bypasses_invulnerability` tag. Otherwise the original function runs untouched. The existing Worldbound stack/definition/trait/tag gates are reused, including broken tools and armor.

## Why a bounded native adapter

The native entity walk uses opaque macros. The supported mixin boundary can preserve that body or cancel at its head, but cannot safely select the item assignment inside it. The helper therefore copies the pinned native entity-damage function and changes only the item-destruction decision. It retains the native MIT copyright and permission notice.

Brief static review compared the normalized copied function with the original: the only behavioral insertion is the Worldbound check in the item branch. No explosion is rerun and no dropped-item list is removed/reinserted. Future edits or injections into the original function body must also be reconciled with this adapter. Hooks in methods it calls, including player explosion damage and native explosion knockback callbacks, still execute.

## Limits

Lightning remains a separate unprotected path. All other #817 limits remain: no separate custom-entity identity, exact age/render parity, full generic invulnerability system, acquisition or rare-name display. This uses the native damage registry/tag view, not an independent Forge damage-tag interpreter. In-world exposure/push, chained blasts, guest order, save/reload and LAN behavior remain unverified.

No tests, builds, planner, CI inspection, version checks or merge were performed. Review was limited to source comparison, Rust formatting and whitespace inspection.
