# Material Tool Forge acquisition

The source `seared_forge_material` and `scorched_forge_material` recipes use
four equal-material fake storage blocks, three matching masonry blocks and a
Tinker Station. Twelve native variants now cover the existing Bronze, Constantan,
Invar, Electrum, Pewter and Nicrosil material-bearing blocks in both families.
They reuse their existing Anvil outputs, fallback textures, material identities
and models, adding the source Tool Forge name as a native component.

The existing material guard still compares every fake input's stored Material
against the output's RequiredMaterial. The required count remains three except
for the exact 3x3 source Forge shape: four singleton fake-block ingredient cells
at positions 3, 5, 6 and 8, one Tinker Station at 4, an empty 7, and three nonempty
non-fake/non-station masonry ingredients across the top. Only that shape requires
four. The native shaped matcher still checks the actual grid and owns consumption;
this guard neither assembles the output nor mutates stacks.

Two source discovery advancements retain the fake storage block OR recipe-unlocked
condition and reward the six bounded variants each. Existing ordinary-metal Forge
recipes and three-block Anvil recipes are unchanged.

## Limits and evidence

This is not a general `crafting_shaped_materials` implementation. It inherits the
existing six-material mapping, saved Material representation, native item-matching
limits and fallback appearance. Arbitrary live material-tag replacements, new
materials, malformed/opaque component migration, material availability in every
pack, and exact generic source texture behavior remain separate. Recipe-book
placement may choose a fake block of another material; the guard rejects the
result rather than spending wrong material. It does not implement smart placement.

Four exact source recipe/advancement fixtures have URLs and SHA-256 hashes.
Python cases cover costs, metadata, discovery and packaged resources. The Rust
fixture includes the actual guard body with minimal adapters and covers old/new
counts, each mismatching or missing material, malformed shapes and ordinary
passthrough. It is not a native host integration test.

Only brief source/diff and whitespace review was performed. Tests were written but
not run. No builds, CI inspection, version checks or merges. Native injection,
actual crafting/shift/crafter behavior, name display, reload, save/reopen and LAN
remain unverified. Part of high-priority acquisition work in issue #379.
