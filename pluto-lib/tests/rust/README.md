# Native inventory fixture

`inventory_host.rs` compiles against `pluto_inventory_api` and the pinned Horizon libraries. It passes borrowed real `ItemStack` values to the planner and checks that different custom data prevents a merge. It does not start the game or commit an inventory transaction.

Build `pluto_inventory_api` and the matching Horizon `horizon_items` and `horizon_world` libraries first. Compile the fixture with `rustc --test`, their dependency directories as `-L dependency=...`, and their exact rlib paths as `--extern pluto_inventory_api=...`, `--extern horizon_items=...`, and `--extern horizon_world=...`. Run the resulting test binary. Use Horizon rlibs from the same build.
