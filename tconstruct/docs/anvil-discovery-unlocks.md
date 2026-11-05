# Tinkers' and Scorched Anvil discovery

The source separates ordinary retextured Anvil recipes from material-bearing
Anvil recipes. Native discovery now preserves that distinction for both families:

- The existing ordinary goal uses the current-world `tconstruct:anvil_metal`
  inventory criterion OR any ordinary recipe-unlocked event. It rewards eleven
  fixed ordinary metal variants and the mixed-ordinary-alloy fallback.
- A separate material goal uses acquisition of `tconstruct:fake_storage_block`
  OR any matching compatibility recipe-unlocked event. It rewards the six
  existing material-bearing compatibility recipes.

The two sets are disjoint and together cover all eighteen native recipes per
Anvil. Each event uses one string-valued `recipes` condition. No visible
achievement, toast, item grant, crafting cost, output or material guard is changed.
Already-earned recipe knowledge is not revoked.

Ordinary variants are Amethyst Bronze, Cinderslime, Hepatizon, Knightmetal,
Knightslime, Manyullyn, Netherite, Pig Iron, Queen's Slime, Rose Gold and Slimesteel.
Compatibility variants are Bronze, Constantan, Electrum, Invar, Nicrosil and Pewter.
Their existing fake-block recipes require matching material metadata on all
three metal inputs. The source discovery criterion intentionally checks only
fake-block item identity, not its stored material. Discovery does not establish
that a material is available or that a particular fake block can be crafted.

Ordinary discovery continues through the bounded world-aware source tag bridge.
Missing/empty tags match nothing; source tag replacements can affect discovery
without expanding the fixed native crafting choices. Reload invalidates its
index but does not synthesize an inventory event or revoke earned progress.
The material goals need no custom tag or metadata predicate and do not add fake
blocks to the ordinary Anvil Metal tag.

Exact pinned source fixtures, hashes and attribution are retained in
`tests/fixtures/anvil_discovery_unlocks/` for ordinary goals and
`tests/fixtures/anvil_material_discovery/` for material goals. Source paths are
`src/generated/resources/data/tconstruct/advancements/recipes/decorations/tables/`
and `.../recipes/misc/tables/`, respectively. Native adaptations preserve each
source OR rule while mapping one recipe ID to its bounded native variants.

This is not generic retexturing, arbitrary material/tag compatibility or natural
material supply. Recipe-book filling may select an incompatible fake block;
the existing guard rejects that craft. Smart material-aware placement remains
unimplemented. No earned-progress migration or revocation is introduced.

Tests cover exact source mapping, disjoint/complete reward sets, unchanged native
recipe metadata and packaging. The inherited dynamically formatted masonry-tag
expectation is also corrected to the live source tag. Tests were updated but not
run. Only brief source/diff and whitespace review was performed. No builds,
CI inspection, version checks or merge. Inventory/recipe events, real recipe-book
placement, crafting, reload, persistence and LAN remain unverified.
