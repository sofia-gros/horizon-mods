# Tool Forge acquisition

The source has two alternative station-upgrade recipes: three Seared or Scorched
masonry blocks across the top, four matching metal blocks around a Tinker Station,
producing the corresponding Anvil named **Tool Forge**. These crafting routes were
absent from native recipe resources.

This slice adds 24 ordinary native recipes: eleven existing metal variants and one
mixed-alloy fallback for each masonry family. It reuses the existing Anvil items,
models, rock-independent metal/texture components and stock crafting transactions.
The source legacy display-name JSON is adapted to the native translatable
`minecraft:custom_name` component, without copying arbitrary source NBT. No new
block registration, capability, workstation capacity or runtime hook is added.

Supported metal blocks are Slimesteel, Amethyst Bronze, Rose Gold, Pig Iron,
Cinderslime, Queen's Slime, Manyullyn, Hepatizon, Knightmetal, Knightslime and
Netherite. Mixed supported ordinary blocks use the existing default Anvil
appearance. The existing native variants and source `match_all` texture fallback
are retained. The separate [material Tool Forge routes](material-tool-forge-acquisition.md)
cover six existing material-bearing compatibility blocks with an exact four-block
guard. Other material mappings are not included.

Two source discovery advancements retain Anvil Metal acquisition OR any matching
native recipe-unlocked criterion and reward the twelve corresponding variants.
The existing world-aware source-tag advancement bridge supplies the acquisition
condition. The similarly named broad-tool collection challenge is unchanged.

## Boundaries and evidence

Masonry inputs use the live source Seared/Scorched Blocks tags. Metal inputs remain
bounded to the existing native block IDs; arbitrary Anvil Metal tag replacements,
new alloys and component-sensitive retexturing are not implemented. Input station
metadata is consumed under the existing native shaped crafting semantics, not
transferred onto the new Anvil. Existing Anvil naming/placement/drop behavior is
unchanged and is not newly verified here.

Exact source recipes and discovery JSON, URLs and hashes are in
`tests/fixtures/tool_forge_acquisition/`. The generator preserves native Anvil
outputs and adds only the Tool Forge name. New cases cover source identity,
patterns/costs, exact components, the fixed supported set, mixed fallback,
discovery and packaging. Two older dynamically formatted Anvil test expectations
are also updated to the live masonry tags introduced by the preceding change.

Only brief source/native-template/diff review and whitespace review were done.
Tests were added but not run. No builds, CI inspection, version checks or merges.
Actual crafting, name rendering, recipe-book placement, reload, save/reopen and
LAN remain unverified. This is partial acquisition coverage for issue #379.
