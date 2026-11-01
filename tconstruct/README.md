# Tinkers' Construct for Horizon

This work-in-progress port brings Tinkers' Construct's parts and tool stations to Horizon. It uses Horizon's existing Minecraft resources and recipes where available.

## Try it

Choose the [Windows guide](docs/windows-playtest.md) or [Linux guide](docs/linux.md) to prepare the mod and use a separate test world. Windows has not been tested on a PC, and a reported launch error is unresolved.

## Make a part

The Part Builder can turn materials and patterns into tool parts. The Tinker Station can assemble and repair supported tools. See [Making parts and tools](docs/playing.md) for an illustrated example. The [Anvil guide](docs/anvil-assembly.md) covers five-input assembly, repair and representative Linux LAN checks; Windows remains untested.

In Creative, the Tinkers blocks tab also offers registered building blocks. See [the block guide](docs/block-registration.md) for the tested selection and its limits. See [Connected glass](docs/connected-glass.md) for windows and panes.

Use a Melter and a casting setup to melt and cast supported materials. The [melting and casting guide](docs/melter-casting.md) shows the tested copper example.

A small Smeltery can also melt copper and amethyst into an alloy. See [Smeltery and alloys](docs/smeltery.md) for the tested setup. The [fluid container guide](docs/fluid-containers.md) explains bucket transfers.

The [Foundry guide](docs/foundry.md) shows a tested copper ore and byproduct setup. [Mining and durability traits](docs/mining-traits.md) describes representative material effects. [Harvest tier and effective blocks](docs/harvest-rules.md) describes tested drops, saved-tool limitations and LAN results. [Area mining](docs/area-mining.md) covers the tested Creative-obtained Sledge Hammer and Excavator. [Chain mining](docs/chain-mining.md) covers representative Vein Hammer and Broad Axe use. [Tool transformations](docs/tool-transform.md) describes tested tilling, pathing and stripping. [Crop harvesting](docs/crop-harvest.md) covers tested Kama and Scythe replanting and its remaining shears limitation.

[Material armor damage and repair](docs/armor-durability.md) covers tested defense, retained breakage and repair.

[Material projectile stats](docs/projectile-stats.md) describes tested Longbow and Crossbow arrow speed, accuracy and current limits.

[Reinforced](docs/playing.md#reinforced) has representative Linux casting, Station, Crystal and LAN checks; Glowing and its modifier-caused wear remain unimplemented. Diamond and Emerald upgrades have representative Linux Station, Anvil, Crystal, mining, armor and ranged checks; see [gem upgrades](docs/playing.md#gem-upgrades). Windows and other source-tagged ranged tools remain unverified or unsupported. [Haste and Modifier Crystal extraction](docs/playing.md#upgrade-a-tool) have representative Linux single-player and LAN tests. Quick Charge and Trueshot have representative Crossbow transaction and save tests on Linux, including LAN Trueshot and Quick Charge crystal reapplication. A Longbow retained Trueshot after reload and showed less lateral arrow spread at level II than level I in a small Linux sample. Trueshot Crystal reapplication with partial progress was checked at all three stations. The mod is still in development. The Smeltery and Foundry are not complete, and material traits and modifiers are not fully implemented. The tested build does not include sound.

## Develop

See [Build and test](docs/development.md) for source preparation, tests, and the Mixins build.

Sky Geodes provide [Skyslime Crystals](docs/playing.md#skyslime-crystals) in newly generated land areas. Natural generation, mining, Worktable use, reload and LAN visibility have representative Linux checks; crystal regrowth is not implemented.

Earthslime, Ichor and Enderslime Geodes have representative Linux natural generation, mining and crystal-use checks. Crystal regrowth and many material traits remain unimplemented.
