# Colored slime block acquisition

Sky, Ichor and Ender slime blocks are registered as block items, with the
source models, textures, map colors, friction and Ichor light level. They appear
between each matching ball and congealed block in the partial World Creative
tab. They are distinct from the fluids with the same identifiers.

The existing source Basin recipes now have registered outputs: consume one
matching congealed block and 1,250 mB of matching slime fluid. Cooling takes
107 ticks for Sky, 212 for Ichor and 119 for Ender. Current world tags and
registered-output checks still apply. Normal crafting compresses nine matching
Forge-tagged slimeballs into one block; unpacking returns nine matching balls.
Breaking the block drops itself, subject to explosion survival.

The three block-to-slimeball recipes have source recipe-book discovery: owning
the matching block OR an unlocked event for its recovery recipe unlocks that
recipe. The original direct-item predicates, OR requirements and hidden
advancement parent are preserved. Native recipe IDs and the host string-valued
`recipes` event condition are adapted. This does not add the separate tagged
compression unlocks or change the existing Congealed Slime recovery unlocks.
Source advancement bytes and SHA-256 hashes are retained in
`tests/fixtures/slime_block_recovery_unlocks/`.

This is registration and acquisition support, not complete slime behavior.
Horizon uses vanilla block identities for bouncing and sticky-piston behavior;
copying the vanilla slime content template does not implement the source
colored adhesion rules. Bounce, entity motion and the separate Sky/Ichor/Ender
piston rules remain unfinished. In-game rendering, casting, crafting, lighting,
recipe-book unlocks, save/reload and LAN remain unverified.

Source: Tinkers' Construct
[`f4e77d43`](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2).
Registration and properties come from `world/TinkerWorld.java` and
`shared/block/SlimeType.java`; recipes are in the existing pinned archive.
The models and sprites are from `src/main/resources/assets/tconstruct`, listed
in `upstream-colored-slime.sha256`. They retain the existing TConstruct license
in `third_party/tconstruct/LICENSE`; no vanilla textures are redistributed.

Focused data/package regression coverage is included for the new registrations,
amounts, colors and models. Test execution and full-host checks are deferred.
