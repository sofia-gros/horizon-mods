# Item-name consumer connection: remaining native boundaries

Source-only follow-up to the [passing item-display fixture](item-display-language-result-20261106.md), inspected on 2026-11-06. No production source, default-component hook, registry or ItemStack was changed. No additional Rust, metadata, native-plan, Game or network execution was performed for this follow-up.

Official source is Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`. Links below are immutable. The earlier 22-test result remains limited to its exact fixture.

## Existing standard-name connection needs no new registry

The real [Names::display_item](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/creative.rs#L108-L158) already consults the selected language's `item.namespace.path`, then `block.namespace.path`, then its English/path fallback. The fixture proves this lookup for bare stacks with author-supplied standard keys and language data.

The tested explicit `minecraft:item_name` component is a separate path: native ItemStack and styled-name functions re-resolve its retained translatable Tag. Neither finding requires another alias table or a global dictionary owned by PlutoLib. Installed module composition and the actual Game screen remain separate gates.

## A declared default is not an explicit stack patch

[Content Plan::items](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/mod_content.rs#L1375-L1412) writes declaration overrides to the item-default resource table. [item_defaults::read/shared](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-world/src/item_defaults.rs#L1-L120) supplies that table to native readers and invokes the default-components hook. This does not itself construct a stack component patch.

Concrete inspected paths:

- [ItemStack::new](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-items/src/item_data.rs#L258-L267) initializes `extra: None`.
- [item_from_nbt](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-items/src/item_nbt.rs#L26-L44) decodes the supplied components; it has no defaults-table argument.
- [crafting::Template::parse/create](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/crafting.rs#L270-L315) decodes the recipe's explicit components and clones them into its result. It does not consult the item-default map there.
- [ItemInfo::custom_components population](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-items/src/items.rs#L890-L906) preserves non-`minecraft:` values only. The `components` list retains key presence, not the value of a vanilla item-name default.
- [Game::native_component](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/native_components.rs#L23-L46) accepts registered custom component types and falls back to that custom map. It is not an existing general default-aware `minecraft:item_name` accessor.

These observations identify concrete missing connections; they are not a complete audit of every item-creation route. In particular, adding a translated name through the default-components hook alone cannot be presented as proof that every local recipe or bare-stack tooltip receives it.

## Removal semantics block a naive default-name fallback

A fallback must distinguish an absent patch from an explicit removal. The inspected network path does not expose that full distinction in ordinary ItemStack name fields:

- [item_sync::from_network](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/item_sync.rs#L13-L34) constructs the ItemStack from the decoded patch and may retain the original opaque Wire patch.
- [components_tag removal loop](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/item_sync.rs#L374-L392) projects removals into semantic `other` fields only for `minecraft:max_stack_size` and registered custom types. It does not project a standard item-name removal there. The custom registry rejects the Minecraft namespace ([custom.rs](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-protocol/src/item/custom.rs#L38-L53)).
- [ItemStack::item_name](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-items/src/item_data.rs#L356-L363) and [styled_name](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/item_tooltip.rs#L1099-L1112) read the explicit stored name, not a defaults/removal state.

Consequently, simply returning the default when those accessors yield None could revive a removed remote name. Retained Wire may contain the information, but its presence is not a supported semantic removal query or permission to reinterpret/mutate it. No such fallback is enabled here. This is a source-derived risk of the proposed extension, not a new claim that the unchanged standard-key behavior failed the passing fixture.

## Minimum next proof before broadening the contract

1. Keep standard item keys and explicit translatable item-name patches as the currently proven bounded contracts.
2. If arbitrary declaration-default names are required, obtain one actual resource-snapshot-bound read path and prove absent / explicit value / explicit removal separately for local and remote stacks before designing an adapter. Preserve custom-name and special native tooltip precedence; do not pretranslate stored components.
3. Test the concrete recipe-result constructor and default-table reader together, rather than inferring propagation from independent successful reads. Include reload and source ownership; avoid a second defaults cache.
4. Keep Game display, retained-page/recipe-view refresh, English-title protocol identity and pack/LAN overlays as their own consumer gates. Do not label a read-only name helper as completion of those behaviors.

No new implementation is proposed until the missing removal/default distinction has a safe actual caller contract. This keeps the two-layer API thin and avoids creating a public helper that appears to guarantee unsupported automatic materialization.

## Source identities

- `item_defaults.rs`: `a5a1c495cfecf422ae497e786c154b075360efc55aafd3ccb5e4ca22946b3338`
- `items.rs`: `ac65c572d00bea03ddd37899c6c40b5091b435a44fa0b5deab4fd3677248bc49`
- `crafting.rs`: `5b014282c68ad2befbfa59f9980fb4b659f258894da76cecb3b7e23bae2324f4`
- `game/native_components.rs`: `aa0d34af920423f45fa323bdeb7c33a5a73c453b07e4a359829cf1fa0949334b`
- `item_sync.rs`: `abfed8999de083b4dcc06935c468679f24903c31243c02056135de92375af912`
- `horizon-protocol/src/item/custom.rs`: `7286507e7353338c068643db676f065ae609129a2250027144868e0a341158c6`

Other exact source hashes remain in the frozen item-display input record. No user-fork source was inspected.
