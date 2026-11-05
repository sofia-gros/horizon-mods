# Boundless acquisition (#379)

Adds the canonical single-level Boundless ability to the existing source-driven
Station planner, extraction list, ability budget and item/Station text.

The normal recipe requires five separate input stacks: four Obsidian Panes,
one Book and Quill, four more Obsidian Panes, two Ichor Slime Crystals and two
more Ichor Slime Crystals. This is eight panes and four crystals total, not one
of each. Five ingredients require the Anvil input capacity. One Boundless
modifier crystal can instead be used in either supported workstation. Applying
it consumes one ability slot and cannot purchase a second level. Source material
traits remain independent of that single purchased level.

Only supported shield items admitted by the current world's
`modifiable/shields` tag qualify. The exact source recipe shape, amounts, slot
cost and canonical runtime modifier definition are required. A malformed or
out-of-range saved Boundless value is never silently replaced. Counted-input
handling is scoped to this canonical Boundless recipe; other recipes keep their
existing matching logic.

Extraction uses the existing wet-sponge plus Sky Slime Crystal flow, returns
one Boundless modifier crystal, and requires both the live shields tag and the
canonical Boundless salvage recipe. Removing the saved level frees its ability
slot through the existing rebuild accounting. Material swaps include this
ability's cost in their capacity validation.

Stacked on the local Boundless effect branch `f2382cef`; the runtime dependency
is native `5e47547e2`. Both remain unpublished. Acquisition does not create the
missing native hook or waive its integration requirement. No provenance-blocked
fluid code is included.

Sources: TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` (MIT), ordinary and
salvage `tools/modifiers/.../boundless.json` recipes and its modifier definition.
The retained source archives are unchanged. Brief static review and whitespace
checks only. Regression cases were added but not run. No tests, builds, planner
transforms, CI, version checks, merges, GUI use, saving or LAN validation.
