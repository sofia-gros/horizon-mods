# Native placement fixture

`wet-sponge-host.rs` uses the pinned Horizon world library and prepared resources to check a successful wet-sponge write in the Nether. The host changes its final state to dry sponge, and the placement commit helper must still commit the item. This is a host-library fixture, not a game or LAN test.

With matching, already-built `horizon_world` and `horizon_core` rlibs, set `HORIZON_RESOURCES` to the target host's prepared resources and compile the test with `rustc --test`, passing their paths using `--extern horizon_world=...` and `--extern horizon_core=...` and their dependency directory using `-L dependency=...`. Run the resulting test binary. Use rlibs from the same host build; mismatched builds cannot be linked reliably.
