# Third-party credits — Pipes mod

## BuildCraft pipe textures (MMPL)

The four pipe block textures in `content/assets/pipes/textures/block/` are
textures from **BuildCraft** (by SpaceToad and the BuildCraft Team,
http://www.mod-buildcraft.com), used **unmodified** under the
**Minecraft Mod Public License 1.0 (MMPL)**, version 1.0.1 — the licence
found in the `LICENSE` file at the root of the upstream repository.

- Upstream source: https://github.com/BuildCraft/BuildCraft
- Exact version used: tag `7.99.24.7`,
  commit `74463a28c1784cc28a716dfd5c989df012b34b91`
- Upstream licence text: `LICENSE` in that repository (Minecraft Mod
  Public License, Version 1.0.1)

### Files used (byte-identical to upstream; sha256 shown)

| File in this mod | Upstream file (in `buildcraft_resources/assets/buildcrafttransport/textures/pipes/`) | sha256 |
|---|---|---|
| `content/assets/pipes/textures/block/item_pipe.png` | `iron_item_clear.png` | `6838ed3ae0a70e4cfde32e60bdb2ba884781161abc5a610836685089d4308d10` |
| `content/assets/pipes/textures/block/extraction_pipe.png` | `wood_item_clear.png` | `8f1e387a9081a4b1977e0d371f75d06b957f1d2210e3da5a555a0b5837185828` |
| `content/assets/pipes/textures/block/fluid_pipe.png` | `iron_fluid_clear.png` | `31498ef2c69f25762a63bfa3471f91e77a5e6a1f3c0ffaa3c68629030b5f6c4a` |
| `content/assets/pipes/textures/block/fluid_extraction_pipe.png` | `wood_fluid_clear.png` | `d68b31f79b43a8fcebc23e0e4d87d9f8aa9831602f4b49f691714d77ecf7905e` |

Notes on the mapping: BuildCraft's extraction pipes are its wooden pipes,
and its iron pipe family is the metal pipe family that has both item and
fluid variants, so the fluid pipe uses the iron fluid texture (BuildCraft
has no copper pipe). BuildCraft uses the `_clear` variant for an empty
pipe and `_filled` only while contents are inside; this mod has a single
static texture per block, so the `_clear` (empty) variant is used.

These textures are used **unmodified**: no recolouring, no pixel edits,
no resizing or cropping. The source of this mod, including these texture
files, is published in this repository
(https://github.com/sofia-gros/horizon-mods), which is the source
disclosure for the MMPL.
