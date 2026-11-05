# Worktable masonry ingredients

All 13 existing Modifier Worktable recipes now use the source
`tconstruct:seared_blocks` item tag rather than the private copied
`tconstruct:anvil_seared_blocks` list. The source acquisition advancement already
uses Seared Blocks, so discovery and crafting now consult the same source tag.

This reuses the nested native Seared Blocks tag added for Anvil acquisition.
Default-world inputs remain the same seven blocks. World tag replacements,
including an empty parent tag or changed Seared Bricks child, govern ingredient
resolution without a literal fallback. The old private tag is retained.

The pattern remains three masonry blocks over four matching stone blocks.
Every native recipe ID, fixed rock selector, output item/count, rock identity,
texture component, model and discovery criterion is unchanged. The generator
uses the same source masonry tag. Existing rock variants remain bounded to the
13 supported source defaults: this does not implement arbitrary workstation
rock tags, generic retexturing, additional variants, or Worktable operations.

## Evidence and limits

`tests/fixtures/worktable_masonry_tags/` contains the exact pinned source recipe,
its URL/hash and fingerprints of all prior native recipe fields except the
deliberately changed masonry input. New regression cases cover these boundaries,
the source tag/generator and existing discovery. Existing resource and discovery
tests now expect the live tag; existing package coverage still includes all 13
recipes. Shared Anvil cases cover the nested tag defaults and replacements.

Only brief source/diff and whitespace review was performed. Tests were written
but not run. No builds, CI inspection, version checks or merge. Actual crafting,
recipe placement, pack replacement/reload, save/reopen and LAN remain unverified.

Part of issue #379; not full Worktable acquisition or gameplay parity.
