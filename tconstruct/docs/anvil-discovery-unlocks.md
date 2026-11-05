# Tinkers' and Scorched Anvil discovery

Each source Anvil has one retextured crafting recipe. The native port already
represents it as eighteen recipes: eleven fixed ordinary metal variants, six
material-bearing compatibility variants, and one mixed-ordinary-alloy fallback.
One hidden advancement for each Anvil now rewards those eighteen recipe IDs.
Both preserve the source `tconstruct:anvil_metal` inventory criterion, OR the
unlocked event for any of that Anvil's native recipes. Every event uses its
own string-valued `recipes` condition rather than an unsupported array.

The ordinary variants are Amethyst Bronze, Cinderslime, Hepatizon, Knightmetal,
Knightslime, Manyullyn, Netherite, Pig Iron, Queen's Slime, Rose Gold and
Slimesteel. Compatibility variants are Bronze, Constantan, Electrum, Invar, Nicrosil and Pewter. Their
existing fake-storage-block recipes still require matching material metadata
on all three metal inputs. Rewarding a recipe only discovers it; it neither
supplies ingredients nor relaxes the material guard. The two mixed fallbacks
remain named `zz_{tinkers_anvil,scorched_anvil}_mixed_alloys`.

This extends the bounded current-world advancement tag bridge from PR #763
by `tconstruct:anvil_metal`. Candidate indexing and the final predicate use
the same resolved world tag snapshot. Missing, invalid or empty membership
matches nothing; stale native tags do not provide a fallback. The existing
snapshot reload invalidates the index, without synthesizing an inventory
event or revoking earned progress. Tag-only changes are observed on the next
inventory change. Unrelated tags and non-item registries remain unchanged.

The discovery tag is the source tag, not a hardcoded list of the native recipe
inputs. A world datapack can therefore affect discovery membership without
expanding the fixed crafting choices. Compatibility placeholder items only
satisfy inventory discovery if the current world's tag actually contains them;
this patch does not silently add placeholders or arbitrary metadata predicates.

Source advancements are from Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, in
`src/generated/resources/data/tconstruct/advancements/recipes/decorations/tables/`.
Exact bytes, SHA-256 hashes and attribution are retained in
`tests/fixtures/anvil_discovery_unlocks/`. Native changes are legacy `tag` to
`items: "#tag"`, `recipe` to `recipes`, and the one-to-eighteen recipe-ID mapping.
No display or achievement toast is added.

Discovery does not change crafting guards or ingredient costs. This is not
arbitrary retexturing, full source metal/ingredient parity, or first-metal
acquisition. Pig Iron and Knightmetal recipes, models and mixed-fallback
extensions are described in [the native metal extension](anvil-native-metals.md).
Recipe-book automatic filling of compatibility metadata remains unverified.

Source/mapping/metadata/package regression coverage was added, and the shared
world-tag fixture includes the new tag. No tests or compilation were run.
Brief static review only: injection application, inventory/recipe events,
recipe-book UI, crafting, reload, persistence and LAN remain unverified. No CI,
version checks or merge was performed.
