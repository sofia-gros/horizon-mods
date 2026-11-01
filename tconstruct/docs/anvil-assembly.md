# Tinker’s Anvil and Scorched Anvil

![A crafted Slimesteel Tinker’s Anvil](anvil-alloy-placement.png)

*A Tinker’s Anvil made with Slimesteel blocks.*

Craft either Anvil with three alloy blocks across the top, one seared or scorched block in the center, and three matching-category blocks across the bottom. Matching alloy blocks give the Anvil their appearance; mixed alloys use the fallback appearance. Use seared blocks for the Tinker’s Anvil and scorched blocks for the Scorched Anvil. Supported alloy blocks include Slimesteel, Amethyst Bronze and Manyullyn; Netherite blocks also work.

![A LAN guest assembling a Sledge Hammer](anvil-assembly-lan.png)

Open the Anvil and choose a tool layout. Make its parts at a Part Builder, then place them in the displayed slots and take the result. Large tools such as the Sledge Hammer need four parts and require an Anvil. Place an existing tool in the tool slot to repair it or exchange a part; the Anvil also lets you name the resulting tool.

A Linux test built and placed both Anvils, made Sledge Hammer parts in the Part Builder, assembled them, repaired and renamed a hammer using the fifth input, and checked the Station’s smaller layout. A Slimesteel Anvil retained its appearance through save/reload and its material when mined. A LAN guest saw the alloy, assembled a Sledge Hammer, exchanged a part and received an excess shard from an upgrade. Matching compatibility alloy blocks were also checked; mixed compatibility materials were rejected. Windows and a complete survival playthrough remain untested.

The Linux test used Horizon `b6c936f82b08798057161bdfa577e3aac53fb91e`, Horizon Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab`, code at `5354bf61ca82ebe4585332973089a0d3ab1c989d`, and the pickaxe tag from `fab71de`. The client binary SHA-256 was `674aab54a2292c945aaf49733e185ccab861c8219f83a98418a85503bc732f8a`. These are representative checks, not a claim that all tools, materials, Windows or the remaining roadmap are complete.
