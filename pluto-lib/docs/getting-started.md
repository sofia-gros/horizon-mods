# Make a first content mod

The demo creates a gem, a block and a Creative tab using PlutoLib's Rust API. Horizon registers the generated content; the Creative tab is added to a prepared resource file before launch. These are separate steps.

## Generate the demo

You need Rust and a prepared Horizon 26.3 resource directory for the [target revision](../README.md#compatibility). From `pluto-lib/`, choose a new output directory under `.work`:

```sh
cargo run --locked --example demo -- .work/demo /path/to/horizon-resources/creative-inventory.json
```

The output contains `horizon-mod.toml`, item and block declarations, models, names, and a modified `creative-inventory.json`. It references Minecraft textures already present in the game resources; it does not distribute them. The output directory must not already exist. The example uses the top-row position 5, which must be free of visible non-hotbar tabs.

## Run it

On a system that supports symbolic links, create an overlay without changing the original resource directory:

```sh
python3 tools/creative_overlay.py /path/to/horizon-resources .work/demo/creative-inventory.json .work/resources
```

Use the generated directory as a Horizon content mod (`--dev-mod <absolute path to .work/demo>`). The generated Creative JSON must also replace `creative-inventory.json` in a **separate prepared-resource overlay** passed to the client; placing that file in a content mod does not install the tab. Preserve the original resource directory. The exact client command depends on the Horizon build and resource paths; this integration and the resulting game screen are not yet verified for the pinned combination.

The expected result is a **Pluto Demo** Creative tab with a gem and a placeable block. The gem and block use vanilla template behavior and appearance, and are independent of Tinkers. The demo source is [examples/demo.rs](../examples/demo.rs).

## Use it in your mod

Create a `Registry` with your mod namespace, add declarations, and write them into a new directory as in the [overview](../README.md). Supply your own manifest and assets. `CreativeTab` takes a title, icon and ordered SNBT stacks; `add_creative_tabs` returns a changed copy of the inventory document. Register those item IDs separately. See [API notes](api.md) for collisions and constraints.
