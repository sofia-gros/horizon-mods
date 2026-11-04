# Geode cluster loot audit

Against TConstruct `f4e77d43`, the native loot data for Earth, Sky, Ender and Ichor Slime Crystal clusters and Cobalt Cluster retains the source's alternatives: Silk Touch drops the cluster; a `cluster_max_harvestables` tool gives four crystals or shards with Fortune; other tools give two with explosion decay. The twelve small, medium and large Slime Crystal buds drop themselves only with Silk Touch. The four TConstruct tool IDs added to the cluster harvestables tag match the source data.

The new data regression checks cover all five clusters, twelve buds, branch order, counts, conditions, modifiers, drop IDs, random-sequence IDs and the tag additions. These are static resource checks, not proof of actual tool-tag loading, game drops, Fortune distribution, explosions, persistence or LAN. Existing generation or budding behavior is unchanged; see the other geode and world-generation work for its own status.
