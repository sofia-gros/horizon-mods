# Tinkers' Construct for Horizon

This work-in-progress port brings Tinkers' Construct's parts and tool stations to Horizon. It uses Horizon's existing Minecraft resources and recipes where available.

## Try it

Choose the [Windows guide](docs/windows-playtest.md) or [Linux guide](docs/linux.md) to prepare the mod and use a separate test world. Windows has not been tested on a PC, and a reported launch error is unresolved.

### Native bundle artifact

On a manually run **Mixins validation** workflow, download the `tconstruct-hnmod-<commit>` artifact from that run's **Artifacts** section and extract the downloaded Actions ZIP. It contains `tconstruct.hnmod`, a native Mixins source bundle pinned to the Horizon and Mixins revisions in its manifest. The workflow must complete successfully before its artifact is available.

For a desktop Horizon Launcher that supports native bundles, select your instance, open its **Mods** page and install `tconstruct.hnmod` as a native mod, then build the instance using the launcher's native build flow. See the [official native-mod guide](https://github.com/horizon-rust/horizon/blob/374b50d48f0968bd08f7026c7fabb117c8ecb8a6/docs/native-mods.md). Native mods run with the player's permissions.

**Resource preparation is still manual.** The bundle includes `mixins/tconstruct/data/tconstruct-support.zip`; installing the native bundle does not apply those assets or recipes automatically. On a desktop with Python 3, extract that ZIP from the `.hnmod` into an empty folder. It contains `tconstruct/` and `pluto-lib/`. With a prepared Horizon 26.3 resource folder and the native-built game executable, run the following from that folder; choose new, separate output paths for `OVERLAY` and `GAME_RESOURCES`:

```text
python3 tconstruct/tools/resources.py BASE_RESOURCES OVERLAY
NATIVE_GAME --tconstruct-prepare OVERLAY tconstruct/content GAME_RESOURCES
python3 tconstruct/tools/prepare_horizon_source_data.py BASE_RESOURCES GAME_RESOURCES/minecraft-original-pack
python3 tconstruct/tools/prepare_bundled_forge.py GAME_RESOURCES/forge-original-pack
NATIVE_GAME --resources GAME_RESOURCES
```

Replace `BASE_RESOURCES` with the prepared resource directory, `NATIVE_GAME` with the native-built game executable and the two output names with paths that do not already exist. This manual desktop route and the Actions artifact have not been validated as an installed launcher bundle. Smartphone installation, native building, resource preparation and gameplay are unverified; the bundle does not establish smartphone compatibility.

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

The early [Smeltery crafting route](docs/playing.md#early-smeltery-crafting) has a representative Linux game check from Grout through Controller casting, Rose Gold and a crafted Tinkers' Anvil, including LAN Melter input. The test used vanilla material and fuel fixtures; see the guide for its scope.

[Nether Cobalt progression](docs/playing.md#nether-cobalt) has a representative Linux check for natural ore generation, iron-tier mining and blasting, with the tools, furnace and fuel supplied as fixtures.

[Metal part casting](docs/playing.md#casting-metal-parts) has a representative Linux check from natural Cobalt ore through a Station-built tool, save/reload and LAN guest visibility.
