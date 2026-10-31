# Fluid containers

Fluid-container support for the Smeltery and Foundry is in development. This work will connect their existing container slots to shared fluid transfer, preserve the returned container, and support the same transactions in single-player and LAN.

The existing lava-bucket interaction with a fuel tank remains available. The controller screen's two container slots remain unavailable until this work is implemented and tested.

The bucket icons in this work are generated from the first frame of the original TConstruct fluid sprites and the Forge bucket-fluid mask. The reference assets are from [TConstruct e14bb5f](https://github.com/SlimeKnights/TinkersConstruct/tree/e14bb5f1794f561ff164f88f84b8b095191c8ecf) and [Forge 243490d](https://github.com/MinecraftForge/MinecraftForge/blob/243490d78001917c7e6ee6e94b45ef98776aa36f/src/main/resources/assets/forge/textures/item/mask/bucket_fluid_drip.png). Regenerating the snapshot with `tools/import_fluid_buckets.py` requires Pillow and those original sources; preparing game resources uses the checked-in snapshot and does not require Pillow.
