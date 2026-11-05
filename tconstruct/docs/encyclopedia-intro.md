# Encyclopedia guide

The pinned TConstruct recipe casts a book with 90 mB molten gold over 57 ticks. The original recipe is already in the bundled source pack. The book registration, source artwork, and two introductory pages are retained.

## Partial material chapters

The reader now includes General, Ranged, Ammo, and Armor material chapters from the existing source index. Their names differ from the progression books, so an exact mapping selects the existing stat, trait, flavor, part, and sidebar renderer. Each chapter has a partial-guide heading and a mandatory notice. The introductory notice lists what is available and what remains absent.

Selection uses current resolved material definitions and stats, hides hidden materials, and requires at least one supported stat group. Unlike the progression books, these chapters allow the source default tier range of 0 through 32767, including distant materials and ammunition without the basic-ammo tag. Sorting retains the stat-group, tier, source sort-order, and identifier ordering. Encyclopedia Armor uses the source cuirass, cuirass-plus-maille, maille, plating, then shield-core group order. Empty chapters are omitted.

The supported chapter descriptor must have `data: no-load` and exactly the matching `type` plus `detailed: true` in `extraData/tconstruct:material_tier`. Extra tier, tag, predicate, name, or suffix options, transformer arrays, mismatched types, and unknown chapters are omitted rather than interpreted incorrectly. The multi-type Slimesuit chapter is not enabled. Encyclopedia Armor uses only the original eight plating, maille, cuirass, and shield-core stat types; Slimesuit stats are excluded from selection, panels, and composite recipe filters.

The four canonical renderer keys are used consistently for material selection, flavor, visible stats, sidebar categories and recipe filtering, and Armor's multi-panel pagination. Supported stat details and traits retain existing tooltips. Sidebar inputs, casting fluids, composites, and tool previews still use the existing bounded projections; they are not a complete recipe browser or proof of survival acquisition or working trait effects.

Source basis: [TinkerBook material registrations](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/client/book/TinkerBook.java), the bundled original `assets/tconstruct/book/encyclopedia/index.json` and [TierRangeMaterialSectionTransformer](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/client/book/sectiontransformer/materials/TierRangeMaterialSectionTransformer.java).

## Limits and verification

Tools, armor equipment, Slimesuit materials, modifiers, and fluid-effect chapters remain absent. This does not implement new material mechanics, equipment, or recipes. Detailed source-page parity, actual reading/rendering, book acquisition, language changes, host integration, resource reloads, guest synchronization, and total generated book/wire size remain unverified. Existing page-count and wire-byte limits are unchanged.

Regression cases were added for exact chapter identities, default tier bounds, source descriptors, mandatory notices, canonical renderer routing, and an Encyclopedia material-panel reply. Tests, builds, CI, and version checks were not run for this change at the user's request. Review was static only.
