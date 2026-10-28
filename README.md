# Tinkers' Construct for Horizon — Mixins migration

> Draft preview on the unmerged `codex/tconstruct-stations` branch. This README
> does not describe the current `main` branch.

This is an early native source-mixin port of selected Tinkers' Construct
mechanics. It is not a complete port. It builds a separate Horizon executable;
it does not use the former fork's Wasm host API. The native code runs with the
user's privileges. The user's adoption decision is not a claim that current
Horizon documentation describes Mixins as its official exclusive mod system.

The proposed first completion unit is the original Part Builder: recipe-driven
selection, preview, exact material consumption and leftovers, result handling,
presentation, persistence and multiplayer behavior. The Tinker Station and
fixed pickaxe are incomplete previews for a later completion unit. The current
branch does not yet meet Part Builder acceptance; see the
[source-to-port record](docs/original-parity.md) and [roadmap](docs/roadmap.md).

## What works today

In single-player worlds, craft a Pattern and the Part Builder and Tinker
Station at a crafting table, place both tables, and open them by interacting.
In the Part Builder, insert a Pattern and material, then select an illustrated
part and take it from the result slot. Flint or oak planks make a pick head
for two materials, or a tool handle or binding for one. These parts use the original shared item IDs and carry their tested
material ID in custom data. A Pattern is consumed for each part. In the Tinker Station, put the head, handle
and binding in the three input positions and take a flint/wood pickaxe. Its tested
`tic_materials` list survives save and reload, but derived statistics, traits
and modifiers are not fully ported. Put a worn pickaxe and flint in the inputs and take a repaired copy. Repair retains
the tool's existing name and custom data.

Station inputs survive closing and saving the world. Breaking a table returns
its inputs. The pickaxe wears while mining or hitting a living mob, stays when broken, and
mines slowly. The fixed flint/wood parts and tool use original Tinkers textures. In GUI slots, the tested flint and wood parts select their original material sprites; a command-supplied oxidized copper head was also checked, but is not yet craftable.
Two players can use the same table; the tested competing result pickups consume
inputs once. See the [Part Builder](docs/part-builder.png) and
[Tinker Station](docs/tinker-station.png) screenshots. The [material display diagnostic](docs/source-part-visuals.png) includes a command-supplied copper variant.

This is a narrow prototype, not complete Tinkers' Construct. Assembly supports only the fixed flint-head/wood-handle/wood-binding combination;
the other craftable parts cannot yet be assembled into tools. Other materials,
tools and traits are not supported. The Part Builder supports selection and a separate result slot for only three
fixed prototype parts; original recipe lists, material variants and detailed tooltips
are incomplete. The Tinker Station uses separate assembly and repair
result icons, and does not yet provide the original naming or modifier UI.
Only oak planks are accepted for wood parts; the original accepts more
planklike materials. Shift-click, drag and double-click collection in these
screens are disabled. Broken-tool visuals and full broken behavior remain
incomplete. Dedicated servers and arbitrary other item components are not
verified. A plain Lore entry was tested through repair, display and save/reload
in both modes, including with the tool material list and unrelated custom data; this does not establish
preservation of arbitrary other components or all styled text.

## Requirements and setup

Linux, Python 3.12+, Git, Rust and Cargo, and your own prepared Horizon 26.3
resources are required. The reference-data setup also uses locally supplied
Minecraft 1.20.1 and Forge 47.4.0 jars. `compatibility.json` pins the source
revisions. This project does not distribute Minecraft game files or resources.

```sh
git clone https://github.com/horizon-rust/horizon.git ../horizon-official
git -C ../horizon-official checkout b6c936f82b08798057161bdfa577e3aac53fb91e
git clone https://github.com/horizon-rust/horizon-mixins.git ../horizon-mixins
git -C ../horizon-mixins checkout 0500ff24aa5933fca95a877ef41125353c4b4fe3
python3 tools/stage.py --horizon ../horizon-official --mixins ../horizon-mixins
cargo fetch --locked --manifest-path target/modded/Cargo.toml
cargo build --locked --offline --manifest-path target/modded/Cargo.toml \
  -p horizon-client --bin horizon-client --bin horizon-ctl --no-default-features --profile fast \
  --target-dir "$PWD/target/build"
python3 -m unittest discover -s tests -v
python3 tools/resources.py /path/to/prepared/26.3 target/item-resources
./target/build/fast/horizon-client --tconstruct-prepare \
  target/item-resources content target/resources
python3 tools/source_data.py /path/to/minecraft-1.20.1-client.jar \
  target/resources/minecraft-original-pack
python3 tools/source_data.py --forge /path/to/forge-1.20.1-47.4.0-universal.jar \
  target/resources/forge-original-pack
./target/build/fast/horizon-client --resources "$PWD/target/resources"
```

`stage.py` verifies both checkout HEADs and exports their pinned commits; local working-tree changes are not included. It exports a separate source tree,
and runs the Mixins CLI. It refuses an existing export; remove only your own
generated `target/original` when deliberately regenerating. Resource preparation
creates a separate overlay with links to the supplied base resources; keep the
base available. Both preparation commands refuse an existing destination.
The 1.20.1 client jar and Forge 47.4.0 universal jar are supplied locally and
checked against pinned hashes; the extracted source data stay in your generated
resource directory. The [Forge universal jar](https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.0/forge-1.20.1-47.4.0-universal.jar) is available from the official Forge Maven repository. These files
prepare source data for the unfinished Part Builder and do not enable its
original recipes yet. `--tconstruct-prepare` uses a Mixins-injected entry point
and official Horizon's native data-table builder; it does not start a Wasm engine. Generated trees and worlds
are not distributed. The prototype uses original Tinkers textures; attribution and the MIT license
are included in the repository and the generated resource overlay under
`THIRD_PARTY_LICENSES/`.

To repeat the source-recipe ordering comparison, install a JDK and run:

```sh
python3 tools/verify_recipe_order.py /path/to/minecraft-1.20.1-client.jar \
  /path/to/forge-1.20.1-47.4.0-universal.jar /path/to/TinkersConstruct
```

The checkout needs the pinned TConstruct commit locally; the command reads the
commit object without changing the checkout. This compares the unconnected recipe
ordering prototype with Java for the fixed source data. Separate unit tests
cover its explicit rejection of unsupported tree bins.

For the automated single-player game smoke test, run the following under a working
X11/Vulkan display (software Vulkan and Xvfb also work):

```sh
python3 tools/game_smoke.py --game target/build/fast/horizon-client \
  --ctl target/build/fast/horizon-ctl --resources target/resources
# Add --in-frame to test the alternate single-player path.
# Add --split-sticks to test one stick in each of two slots.
```

The script creates a fresh world under `target/`, checks consumption and
mining wear, saves, and writes a screenshot there. The station, recipe,
spill and two-player tests are described in [validation](docs/validation.md).

See the [feature matrix](docs/feature-matrix.md) and [roadmap](docs/roadmap.md) for scope and validation status.

## Provenance and recovery

The prior `main` was preserved at remote branch `backup/pre-mixins-92f696b`,
commit `92f696b475799403120330f6ad9c45c423de4ab7`. To inspect or restore it
without rewriting history, fetch that branch and create a new branch from it:

```sh
git fetch origin backup/pre-mixins-92f696b
git switch -c restore-pre-mixins FETCH_HEAD
```

Tinkers' Construct data is referenced at the revision in `compatibility.json`;
its MIT license and notice are included in [third_party/tconstruct/LICENSE](third_party/tconstruct/LICENSE).
The old environment's uncommitted chest work was not transferred to this checkout.

The material-condition reference work also follows Mantle 1.11.115 and Forge
47.4.0. Their license notices are included at
[third_party/mantle/LICENSE](third_party/mantle/LICENSE) and
[third_party/forge/LICENSE](third_party/forge/LICENSE).
