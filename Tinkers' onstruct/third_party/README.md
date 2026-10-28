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

## Workstation GUI textures

`assets/tconstruct/textures/gui/tinker.png` and `part_builder.png` are copied
without modification from SlimeKnights/TinkersConstruct commit
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, under the MIT license in
`tconstruct/LICENSE`. SHA-256: `tinker.png`
`db597ebec5a5a6cb06e0d702a63ee1a2371f0efb006878e65dc176fac86e8336`;
`part_builder.png`
`971d1b29f4cec328602fbfee415382cdef822b5afec89b63eb46a2d545cf03aa`.

The Part Builder panel and three pattern icons additionally retain unmodified
upstream `textures/gui/panel.png` and `textures/gui/tinker_pattern/` files for
`pick_head`, `tool_handle` and `tool_binding`, from the same MIT-licensed
commit. Their SHA-256 values are respectively
`6dce4d43b21133c75d774dfb3043bec34f9b326b9b3dd9475ff6973a8b329916`,
`ab1d44ba8937357be7cbc02fcb314ce6ee5ce144073156d4eb790a37f45c8dfb`,
`06bb82ef9c82cf984549ba23a8b991290c160cffe8e7dbb98ad062a9a763fc0e`, and
`5587625a90f5e2a677bbe323f504ecb80f80c3c0c45e99a7efb43cf96b4f3313`.
