# Build and test Tinkers' Construct

Builds use the Horizon and Mixins revisions pinned in `compatibility.json`. Check upstream updates separately before changing the verified pair.

## Run the lightweight tests

Material conversion regressions live in `tests/test_material_conversions.py`.
Its shared assertions compare against pinned upstream recipes and tags, without
calling the production converter. Add ordinary alloy names to `ALLOYS`; keep
special recipe families explicit (currently netherite nuggets and Rose Gold).
Failures identify the material, recipe, tag or item through named subtests.
The consolidated suite retains 48 recipe cases, 24 tag comparisons, ten block
checks and 22 item-registration checks from the former twelve modules. The
unittest method count changes from 13 to 4; coverage is retained in subtests.
Run it alone with:

```sh
python3 -m unittest discover -s tconstruct/tests -p 'test_material_conversions.py' -v
```

Keep regression cases after a feature is finished. Remove superseded test files
only when their assertions are represented in the maintained feature suite.
This reduces repeated test code, not generated build or game-world storage.


From the repository root:

```sh
python3 -m unittest discover -s tests -v
python3 -m unittest discover -s tconstruct/tests -v
cargo test --locked --manifest-path tconstruct/tests/condition-snapshot/Cargo.toml
```

## Prepare a modded source tree

You need Python, Rust, Git, a checkout of the pinned Horizon revision, and a Horizon Mixins repository. From the repository root, replace the example paths with your local checkouts and installed bundle:

```sh
python3 tconstruct/tools/stage.py --horizon /path/to/horizon --mixins /path/to/horizon-mixins --pluto-bundle /path/to/installed.incoming/pluto_native_creative/mixins/pluto_native_creative
```

The development target requires the unified PlutoLib native bundle, built from PlutoLib `5520a221655e9ea5b55b9ce32a46bca511c3ae71`. The bundle contains Creative, item, Tool, transport, display and inventory APIs. It supplies one `pluto-api` facade and one `pluto-inventory-api` crate. Use the Mixins bundle directory extracted by the official launcher, or extract the `.hnmod` for staging with Horizon's native bundle reader:

```sh
python3 tconstruct/tools/extract_pluto.py --horizon /path/to/horizon --package /path/to/plutolib.hnmod --output /path/to/installed.incoming
```

The output directory must not already exist. Do not stage the old individual PlutoLib adapters alongside the unified bundle. Install PlutoLib separately from TConstruct; TConstruct still requires its own `.hnmod` and `.hmod`. See [PlutoLib native installation](../../pluto-lib/docs/native-installation.md). Combined installation and gameplay remain unverified.

The generated source is placed in `tconstruct/target/modded`. The resolved Mixins commit is printed and saved in `tconstruct/target/mixins-revision.txt`. The default is the pinned revision; `--mixins-revision FULL_COMMIT_SHA` selects an explicit revision for a separate compatibility test. Ensure the selected commit exists in the Mixins repository. Build the client with:

```sh
cargo check --locked --manifest-path tconstruct/target/modded/Cargo.toml -p horizon-client --bin horizon-client --no-default-features
```

Preparing game resources and running the game also require Horizon 26.3 resources. On Windows, the [playtest launcher](windows-playtest.md) handles this preparation for the playtest.

## Representative Anvil checks

The `tests/game/anvil_*.py` scripts exercise the real client using a dedicated test world under `tconstruct/target`. Supply `--game`, `--ctl`, `--resources`, and the same `--run-dir` inside that directory to each script. Run `anvil_acceptance.py` first to prepare the world, then `anvil_parts.py`, `anvil_compat.py`, `anvil_lan.py`, and `anvil_lan_upgrade.py`. These scripts place and break blocks in that test world; do not point them at a player world. Linux GUI tests need a display and Vulkan adapter. They do not replace Windows testing or a full survival run.

The development manifest targets Horizon `fd6cfda6bdef59a53aee830009f78c14832b0308` and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. A combined Linux build and representative checks have passed, with remaining limitations in [compatibility](compatibility.md).

## Station recipe lookup

Tool-building recipe candidates are indexed when the world recipe snapshot is
captured, after pack overrides are applied. Station previews and repairs use
that per-world index in the same recipe-ID order as the snapshot. Reloading
recipes rebuilds it, and clearing world state discards it. Input matching,
material validation, and the authoritative take still run against current
state. This reduces repeated parsing during station rendering; the reported
Windows FPS drop still needs an in-game before/after measurement.

### Block item assets

Each registered block that has an item needs an `assets/tconstruct/items/`
item definition as well as its item model. The block registration test follows
those references through local model parents and textures. The covered
Scorched Table item model is taken from the pinned Tinkers' Construct revision
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2` at
`src/main/resources/assets/tconstruct/models/block/foundry/table_covered.json`.
The resource-overlay test uses a minimal vanilla item fixture, including the
book template required for the Puny Smelting item.

## Source common configuration

The condition snapshot reads `config/tconstruct-common.toml` from the game
root (the standalone dedicated server uses its working directory, independent
of `--universe`). An absent file uses source defaults. The reader supports these
boolean condition keys:

- `[gameplay]`: `shouldSpawnWithTinkersBook`, `syncKnockbackResistance`
- `[recipes]`: `addGravelToFlintRecipe`, `cheaperNetheriteAlloy`,
  `witherBoneDrop`, `glassRecipeFix`
- `[compatability]`: `allowIngotlessAlloys` (the spelling matches upstream)
- `[debug]`: `forceIntegrationMaterials`
- `[loot]`: `slimy_loot`

Other settings do not disable these conditions, but their corresponding
gameplay behavior is not supplied by this reader. A non-boolean supported value
leaves that condition unknown without disabling unrelated conditions; malformed
TOML or read failures make the snapshot unavailable. The source
condition `slime_recipe_fix` intentionally follows `glassRecipeFix` in the
pinned upstream implementation; deprecated `wither_bone_conversion` is false.
This is a subset, not full Forge config compatibility. Config capture runs on
world setup and datapack reload. Game and LAN behavior still need verification.

### Forge recipe conditions

At recipe capture and datapack reload, Forge conditions are checked against the
current item, tag, config and available mod-presence snapshots before the
recipe enters the shared catalog. Accepted conditions are removed from the
captured recipe so older consumers do not reject them a second time without
that context. False, malformed or unresolved conditions are not loaded. This
includes `forge:item_exists` on upstream compatibility melting recipes; it
does not add the external mod or guarantee other unsupported recipe features.
In-game and LAN reload behavior remain unverified.

### Ore-rate settings

The Melter, Smeltery and Foundry use their own `recipes.ore_rates` settings
from the same world configuration snapshot. The Foundry has separate primary
and byproduct rates. Metal uses `nuggetsPerMetal` (1–45) divided by 9; gem
uses `shardsPerGem` (1–20) divided by 4, with integer truncation. Missing
settings use the pinned source defaults. An invalid or unavailable relevant
rate prevents that ore match rather than silently falling through to another
recipe; unrelated non-ore melting stays available. The snapshot is refreshed
with datapack capture/reload; automatic Forge file watching is not implemented.
The Forge setting `slimeRecipeFix` and other unsupported settings remain outside
this reader. Custom ore recipes whose primary amount truncates to zero still consume input after heating and may produce Foundry byproducts. Game, reload-during-heating and LAN behavior need verification.

Foundry output follows upstream TConstruct f4e77d43: only the primary output
must fit. Byproducts are filled in order up to the remaining capacity, and any
unaccepted remainder is discarded while the input is consumed. A zero primary
output passes the capacity check even when the tank is full, so all byproducts
can be lost. A blocked nonzero primary preserves the input and retries later.

The focused state, tank, fuel and melter harness passed 32 tests, including
blocked-primary reload/retry and partial/full capacity cases, linked against
Horizon item and world libraries. The generated-host `horizon-client --lib`
test binary was not built: both parallel and single-job attempts ended with
rustc killed by SIGKILL. Therefore the host integration tests remain unverified;
this harness does not replace them. Gameplay, LAN and reload while heating also
remain unverified.

### Rose Gold crafting conversions

The source Rose Gold block/ingot and ingot/nugget conversions are supplied as
native crafting recipes. The shaped conversions require the source Rose Gold
item in the center and eight members of its corresponding Forge tag; the
reverse shapeless recipes return nine ingots or nuggets. Native tag files
contain the pinned source members. Datapack additions are intended but host tag
merging has not been verified.
These conversions do not add an alloy production route: the ingot or block
must first be obtained by melting, alloying and casting. Source-data and early
reachability tests pass; crafting-grid behavior, recipe-book display, datapack
reload, save/reload, and LAN remain unverified in game.

### Venombone acquisition

Venombone is registered as a bone-template item in the native content pack and
appended to the manual resource overlay without renumbering existing entries.
Its texture and bone-parent model come from the pinned TConstruct revision.
The existing source casting recipe consumes a `forge:bones` input and 250 mB
of `tconstruct:venom`, cools for 48 ticks, and yields one Venombone. The existing
material recipe accepts one Venombone for one unit of the craftable material.
Recipe lookup still requires the current world's resolved item/fluid tags and
registered output; registration does not bypass these checks.

Package/resource tests cover registration and visuals, with a bounded catalog
regression in `tests/rust/soul_glass_acquisition.rs`. Actual world casting,
Part Builder use, Venom supply, reload, saves and LAN remain unverified. This
does not add Venombone to Creative tabs or implement its material traits.

### Water bottle table casting

The three fixed-source water-bottle Casting Table recipes now support the
result NBT `{Potion: "minecraft:water"}` for normal, splash and lingering
potions. They consume one matching empty bottle and 500 mB from the resolved
`mantle:water` fluid tag, then cool for one tick. The world recipe/tag snapshot
and existing casting transaction remain authoritative.

Only this exact result metadata is supported: other potion types, extra NBT,
result counts and NBT on other items remain rejected. A plain potion result
and a water-potion result are distinct during conflict detection. The native
potion component is created before fluid reservation and persists through the
existing casting state codec. This does not implement arbitrary result NBT,
Ancient Repair Kit casting, or additional potion effects.

The source fixtures, catalog checks and machine state roundtrips pass in the
bounded Rust harness. Actual host crafting, rendering, world reload and LAN
remain unverified.
