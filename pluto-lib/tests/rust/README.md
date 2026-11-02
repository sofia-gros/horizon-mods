# Native inventory fixture

`inventory_host.rs` and `inventory_demo.rs` compile against `pluto_inventory_api` and the pinned Horizon libraries. They pass borrowed real `ItemStack` values to the planner, check that different custom data prevents a merge, and exercise the demo consumer transaction. They do not start the game.

Build `pluto_inventory_api` and the matching Horizon `horizon_items` and `horizon_world` libraries first. Compile the fixture with `rustc --test`, their dependency directories as `-L dependency=...`, and their exact rlib paths as `--extern pluto_inventory_api=...`, `--extern horizon_items=...`, and `--extern horizon_world=...`. Run the resulting test binary. Use Horizon rlibs from the same build.

## Content declaration fixture

`item_declaration_host.rs` reads the generated demo using `PLUTO_DEMO_DIR`, packages its item declaration and checks it with the pinned Horizon content parser. Set `PLUTO_PROBE_HMOD` to a disposable output file; the test overwrites it. Compile it with `rustc --test`, the matching dependency directory and `--extern horizon_mods=...`, then run the test binary. This checks declaration loading, not gameplay.
