# Fake ingot and storage-block conversion

The existing source-faithful conversion planner had no runtime caller. It is now
connected through bounded, world-local native crafting recipes for the two exact
source forms: nine equal-material fake ingots become one fake storage block, and
one fake storage block becomes nine fake ingots. Results are fresh stacks carrying
only the source Material identity; source crafting does not copy names or other
input components.

After the world material/recipe/config snapshots are published, the projection
checks visible material definitions and the same source HasPart eligibility used
by existing fake-block support. The shared helper now accepts either fake output
item; its existing fake-storage caller is unchanged. Eligibility still includes
resolved material tags and visible base-material recipes, so a visible ordinary
ingot or nine-unit block recipe can suppress its fake equivalent.

Recipes are keyed by direction and exact material ID under a reserved prefix.
No process-global material cache or hardcoded list of six alloys is used. At most
512 visible material definitions are projected. Incomplete/invalid eligibility
clears the synthetic recipes instead of keeping a stale set. Exact native-format
world recipe replacements take precedence over the legacy source snapshot; any
unsupported source change or conditional wrapper disables that direction.

Each projected ingredient has an internal marker. The new read-only assembly
guard handles only marked shapes, checks their complete shape/material contract,
and calls the existing planner against the actual trimmed input. The planned
result must match the native result template exactly. Native shape matching,
remainders and consumption continue normally; no second transaction path is
introduced. Unmarked recipes, Anvil guards and other projections are untouched.

## Deferred work and verification

These synthetic recipes are not added to the native recipe-book index; source
discovery, displayed alternatives and automatic material-aware placement remain
separate. They are not generic crafting_shaped_materials support. Source form
changes, variants with `#`, arbitrary conditions and more than 512 visible
materials are conservatively unsupported. Existing saved Material decoding and
component-handling limits are inherited from the planner.

The one-input reverse is represented as native 1x1 shaped crafting, which matches
the same trimmed input as its source one-ingredient shapeless recipe. Runtime
recipes are installed only by the authoritative host, not a remote client.

Exact source fixtures/hashes, existing planner regressions, new geometry and
source-form rejection cases, and wiring checks are included. Only brief
source/native-boundary/diff and whitespace review was performed. Tests were
written but not run. No builds, CI inspection, version checks or merges. Native
injection/compilation, preview/take/shift/crafter, reload, saved recipes and LAN
remain unverified. This is partial acquisition work for issue #379.
