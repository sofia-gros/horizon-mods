# Anvil masonry ingredients

The 36 existing ordinary, compatibility-material and mixed-alloy Anvil recipes
now use the source `tconstruct:seared_blocks` or `tconstruct:scorched_blocks`
item tag. Previously they used private copied `anvil_*` lists, so a world pack
could replace the source tag without changing these crafting inputs.

The new Seared Blocks default retains the source nested Seared Bricks tag.
Scorched Blocks and Seared Bricks already have the exact source defaults and are
unchanged. Default-world input membership remains seven Seared or five Scorched
blocks. Replacements of either the parent tag or the Seared Bricks child can
now affect ordinary world-local ingredient resolution. No hardcoded fallback
is added. Old private tags remain for compatibility. Modifier Worktables use the same
live Seared Blocks tag; see [their correction](worktable-masonry-live-tags.md).

All recipe IDs, patterns, costs, metal selectors, compatibility-material guards,
results, texture components and existing discovery remain unchanged. The variant
generator uses the same live masonry tags so regeneration does not restore the
copies. This does not implement generic retextured crafting, source Anvil Metal
tag parity, Tool Forge recipes or new metal variants.

## Evidence and deferred checks

`tests/fixtures/anvil_masonry_tags/` contains the two exact pinned source recipes
and three exact source tags with hashes and URLs. Native baseline fingerprints
cover every recipe field except the deliberately changed masonry ingredient.
Focused tests cover source identity, unchanged recipe fields, nested tag defaults,
empty/replaced tag projections, existing recipe IDs and packaged resources.
The existing survival-chain fixture now expects the live source tag.

Only brief source/diff review was performed. Tests were written but not run.
No builds, CI inspection, version checks or merge. Actual crafting, recipe-book
placement, pack replacement/reload, save/reopen and LAN remain unverified.

Part of issue #379; this is a bounded acquisition correction, not full Anvil
or resource-tag compatibility.
