# Upstream source and attribution

`tconstruct/` contains an explicit subset of SlimeKnights/TinkersConstruct,
revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` (Minecraft 1.20.1 branch).
Copyright (c) 2022 SlimeKnights. The original MIT license is preserved in
`tconstruct/LICENSE` and in this port's root `LICENSE`.

`upstream-lock.json` at the project root records every imported file and its
SHA-256. Files keep their original Git blob bytes. No Minecraft game files or
decompiled Mojang sources are included.

The Rust numeric accumulator, selected material/stat/modifier interpretation,
and tool-state behavior are adaptations of the corresponding MIT source.
Selected original Java source files remain here as references and as input to
the numeric-oracle generator. The oracle copies the selected arithmetic method
bodies verbatim into a minimal Java host; it does not run Forge or the full
ToolStack implementation. Other Rust code provides the Horizon adapter and
explicit validation for this port's supported subset.

The selected mining-speed and effectiveness methods also feed the separate
`server-tools/tools/generate_mining_oracle.py` oracle. Its identity-modifier
host checks eight speed cases, including a broken tool; it does not simulate
the complete Forge block-breaking and loot pipeline. The pinned harvest and
tool-damage helpers are retained as references for later gameplay integration.

Import from a local clone using `tools/import_upstream.py --source PATH`.
Validate the committed snapshot with `tools/import_upstream.py --check`.
The importer reads pinned Git objects, not the clone's working files.
