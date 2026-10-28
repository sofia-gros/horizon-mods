# Tinkers’ Construct for Horizon

This is an experimental port of TConstruct 1.20.1 to the [Horizon fork](https://github.com/sofia-gros/horizon), not a Forge mod loader. Its eventual goal is the complete original mod. Current `main` supports a small wood/flint pickaxe subset: making three parts, assembling a pickaxe, limited matching-material repair, and connected mining, melee and durability behavior. The tool uses a vanilla wooden-pickaxe carrier. Other tools and materials, the smeltery, many modifiers and material-dependent tool rendering remain unfinished.

The content/client package is in this directory; the experimental server code package is in [`server-tools/`](server-tools/). Build and load both as described in the [player guide](../docs/playing-tconstruct.md). The main documentation index is [here](../docs/README.md).

Original TConstruct GUI assets are attributed in the packaged [source notice](assets/tconstruct/source_notice.txt) and [MIT license](assets/tconstruct/license.txt). The pinned revision and copied research sources are documented in [third_party/README.md](third_party/README.md). The [historical README](README-history.md) retains earlier setup and milestone records; its old version claims and local Windows paths describe those earlier tests, not the current baseline.
