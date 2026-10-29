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

The source recipes, material definitions, tags and tool definitions are
prepared as shared data. Shared selection and material-cost calculations are connected to normal GUI
crafting for supported ordinary parts and limited source recycling. Recycling selects uniformly among available alternate results when the selected source recipe identity is known. We will connect and verify each common behavior once,
then expand content through its source definitions and check separately only
where the original has distinct behavior.

The planned v0.5 is a playable preview that nearly reproduces the original mod
on its own; v1.0 is the complete port. Neither milestone has been reached.

## What works today

In single-player worlds, craft a Pattern and the Part Builder and Tinker
Station at a crafting table, place both tables, and open them by interacting.
In the Part Builder, insert a Pattern and material, select a source-recipe
pattern in the scrolling grid, and take an ordinary part from the result slot.
For example, an oak log produces a wood tool handle and returns three oak
planks; an oak plank can also produce a wood handle. Both paths and save/reload were tested in both game modes. All 29 original gold cast IDs are registered using their source models and sprites. A command-supplied Tool Handle Cast was reused without consumption while crafting a wood handle in a Linux test-generated build; normal casting acquisition, cast cost tooltip, other casts and Windows are not yet verified or implemented. The original sand and red-sand cast IDs are also registered. In a Linux test-generated build, a command-supplied blank sand cast was consumed to create an Adze Head sand cast through the source item-part recipe and survived save/reload. Red sand, other shapes and Windows remain unverified. Four original stone armor-plating dummy items now use the shared item-part recipe path. A test-generated Linux build crafted a stone helmet dummy from three calcite in the normal GUI and retained it after save/reload; the other three and the original dummy tooltip are unverified or incomplete. A left Shift-click on the result can repeat ordinary crafting into the player inventory when no adjacent block entity is present; the test also covers a full inventory and competing players. Part recipes
and materials are still incomplete, and unsupported contexts are rejected. In the Tinker Station, put the head, handle
and binding in the three input positions and take a flint/wood pickaxe. Its tested
`tic_materials` list survives save and reload, but derived statistics, traits
and modifiers are not fully ported. Put a worn pickaxe and flint in the inputs and take a repaired copy. Repair retains
the tool's existing name and custom data.

Station inputs survive closing and saving the world. Breaking a table returns
its inputs. The pickaxe wears while mining or hitting a living mob, stays when broken, and
mines slowly. The fixed flint/wood parts and tool use original Tinkers textures. In GUI slots, the tested flint and wood parts select their original material sprites; a command-supplied oxidized copper head was also checked, but is not yet craftable. Part tooltips display tested material names in English and Japanese; existing explicit names are preserved.
A tested vanilla diamond with custom data also keeps that data when two players
move it through a shared chest and save/reload the world. This check covers
custom data, not every item component.

Two players can use the same table; the tested competing result pickups consume
inputs once. In both game modes, a guest can reopen the table and see the
shared source-pattern selection after the Pattern is removed and restored. See the
[Part Builder source-recipe preview](docs/part-builder-source-preview.png) and
[stone-axe recycling](docs/recycling-preview.png), [flint-and-steel result](docs/recycling-flint-result.png), [crossbow recycling result](docs/recycling-crossbow-result.png), [Pattern identity example](docs/recycling-pattern-identity.png), [material cache example](docs/part-builder-material-cache.png), [oak-plank crafting](docs/oak-plank-builder.png), and [Tinker Station](docs/tinker-station.png) screenshots. The [material display diagnostic](docs/source-part-visuals.png) includes a command-supplied copper variant. The [registered part diagnostic](docs/source-parts-command.png) and [fake ingot diagnostic](docs/fake-ingot-command.png) use commands, including combinations that are not valid crafting recipes.

This is a narrow prototype, not complete Tinkers' Construct. Assembly supports only the fixed flint-head/wood-handle/wood-binding combination;
the other craftable parts cannot yet be assembled into tools. A command-supplied venom bottle can be drunk, giving Strength and poison and returning a glass bottle in the tested cases. A venom bottle and stone axe can also be used in the Part Builder: the tested undamaged axe returns three cobblestone and an empty glass bottle; durability affects the quantity. An undamaged flint-and-steel can return flint and an iron ingot along with the empty bottle when its source selection is current. A command-supplied undamaged crossbow can return three sticks and either two string or one iron ingot. The venom bottle's fluid-container capability, detailed tooltip and normal acquisition remain unimplemented. The 28 original part and repair-kit item IDs and the fake ingot ID are registered; command-supplied examples retain their material data, but most cannot yet be crafted or used as original parts. Vanilla armor durability values are available to the shared recycling path. A test-generated build recycled undamaged leather boots into four leather and damaged boots into two leather in the two game modes, with the empty bottle and save/reload; the official Mixins build and other armor remain unverified. Other materials,
tools and traits are not supported. The Part Builder has a source-recipe selection grid and ordinary pickup, but
recycling with unknown selection identity, modifiable and unsupported input items, all material and item registrations, original container remainders,
complete tooltips and side panels are unfinished. Dynamic part names are not
yet connected to hotbar labels or recipe search. A table retains its cached recipe across reload until its input changes; both game modes verified the old cost and a subsequent input change. For supported recycling, a selection retained across an input change is compared by its source recipe identity: an equal Pattern from a different recipe or reload generation remains an alternative, as in the original. Both game modes tested this path; unavailable selection origins are still rejected. The table also retains a cached material recipe until the material item or its data changes. Both game modes tested a material-value reload and an oak-log tag change, including invalidation on material reinsertion; other materials and tag interactions remain incomplete. The four source slimewood plank blocks are registered with their original textures; a command-supplied greenheart plank was placed, saved, mined and rejected as fuel in both modes. Their normal acquisition and explosion-drop random sequence remain incomplete or unverified. The Java-compatible random state is shared in
the process and advances only after a successful transaction, but initialization
is lazy, the JVM-wide seed uniquifier is not mirrored, and other original uses
of the shared RNG are not yet connected; exact
cross-process random sequences are not established. Random transactions on
non-Unix builds remain unsupported. The Tinker Station uses
separate assembly and repair result icons, and does not yet provide the original
naming or modifier UI. Shift-click of other slots, right Shift-click, drag and double-click collection remain disabled. Result Shift-click is rejected when an adjacent block entity may supply the original side inventory or a neighboring chunk is unloaded; the original side-inventory behavior is not implemented. Broken-tool visuals and full broken behavior remain
incomplete. Dedicated servers and arbitrary other item components are not
verified. A plain Lore entry was tested through repair, display and save/reload
in both modes, including with the tool material list and unrelated custom data; this does not establish
preservation of arbitrary other components or all styled text.

## Requirements and setup

[Windows PowerShell playtest instructions](docs/windows-playtest.md) (native Windows build and play are not yet verified).

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
prepare source data for the partially implemented Part Builder. `--tconstruct-prepare` uses a Mixins-injected entry point
and official Horizon's native data-table builder; it does not start a Wasm engine. Generated trees and worlds
are not distributed. The prototype uses original Tinkers textures; attribution and the MIT license
are included in the repository and the generated resource overlay under
`THIRD_PARTY_LICENSES/`.

To regenerate the original 1.20.1 item durability table (30 tier tools, 11 directly specified items and 25 armor items), use a JDK and the
pinned local client jar:

```sh
python3 tools/extract_original_item_durability.py /path/to/minecraft-1.20.1-client.jar /tmp/original-item-durability.rs
cmp /tmp/original-item-durability.rs mixins/tconstruct/source-original-item-durability.rs
```

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
