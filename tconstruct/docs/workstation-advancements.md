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

The source Tinkers' Anvil goal is intended to be recorded when either the Tinker's Anvil or Scorched Anvil enters the inventory. Its source parent, the Smeltery-open advancement, is not yet connected, so this goal is not placed in the advancement tree until that parent exists. The host may nevertheless show a toast or chat announcement when its criterion completes. The source goal has no reward. Actual acquisition, progress persistence, toast and LAN behavior have not been verified in a running world.

The source Tool Forge challenge has eight separate inventory criteria, one for each listed large or ranged tool; progress can accumulate across separate acquisitions. It is a child of the Tinkers' Anvil goal and has no reward. The Smeltery-open ancestor remains unavailable, so this branch is not laid out in the advancement tree; the host may still issue toast or chat notifications on completion. The source icon's display-only material NBT is omitted because the host advancement icon supports an item ID only. This records item acquisition, not whether each tool's behavior or ordinary acquisition is complete. Actual inventory events, icon, persistence and LAN are unverified.

The source Ancient Tools challenge records four separate inventory criteria:
Melting Pan, War Pick, Battlesign and Swasher. They may be obtained at different
times; this does not implement their ordinary loot sources or tool behaviors.
Its Tinkers' Gadgetry parent is not registered yet, so it is not laid out in
the advancement tree; the host may still show a toast or chat announcement.
The source icon's material-preview NBT is omitted because the host icon accepts
only an item ID. Actual inventory events, progress persistence, icon and LAN
behavior remain unverified in a running world.
