# Initial workstation advancements

The Part Builder and Tinker Station acquisition advancements use the original
TConstruct 1.20.1 definitions at revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`.
The former is a child of Materials and You; the latter is a child of Part
Builder. Both retain the original `minecraft:inventory_changed` criterion;
obtaining the workstation by any method satisfies it. Their display icon field
is adapted from `item` to the host's `id` format. These achievements have no
recipe rewards and do not unlock the vanilla recipe book.

A Linux fixture with Horizon `02c8607ee8ebd216f2f535c1eb7b62b572830df2`
and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a` confirmed that a LAN
guest receiving the two supplied workstations completed its own advancements;
the host did not receive them. Another isolated world saved both workstations
before adding the definitions: on reopening with them, both criteria completed,
and their timestamps survived another save/reload. These are resource-overlay
fixture checks, not a packaged main build or a survival acquisition test.
Further original workstation progression and recipe-book unlocks remain open.

The Travelers' Gear advancement uses the source's four distinct inventory
criteria and the existing Tinker Station parent. The host icon uses `id` and
its current advancement Display reads only the item ID, not the source NBT
used for a material preview. Source trigger logic and requirements are retained;
gameplay, already-held gear, persistence and LAN actor attribution are not
verified. No recipe-book reward is added.
