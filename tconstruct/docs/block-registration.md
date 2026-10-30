# Block registration

This work adds the original Tinkers' Construct blocks to Horizon in groups that share placement and rendering behavior. The first goal is to place, see, break, recover and save the registered blocks, including in a LAN world.

The source pack has separate blockstate files for some fluids, wall-mounted forms and other variants. The [source blockstate list](../content/reference/tconstruct-blockstates-list.txt) records the 435 distinct file names at source revision `e14bb5f1794f561ff164f88f84b8b095191c8ecf`; these files are not a count of unique inventory items. Registration will distinguish block IDs, item IDs and state variants, and reuse existing Minecraft resources and recipes where they already provide the same material.

The scope includes decorative and storage blocks; wood shapes such as slabs, stairs, fences and doors; glass and panes; and the appearance and placement of workstations and machines. Original assets and current Horizon shapes are used where they support the source block. Representative tests will cover placement and orientation, appearance and collision, drops, save and reload, and LAN synchronization.

A registered machine is not necessarily functional. Smelting, casting, alloying, their interfaces, fluid behavior, plant growth and world generation are separate work. Any shape or action that Horizon cannot yet represent will be identified rather than presented as working.

## Current work

The registration data currently covers grout, selected seared and scorched building shapes, slimy wood shapes and logs, several solid metal storage blocks, nahuatl, blazewood, and two lamps. Their registration and asset references have been checked, but placement, rendering, drops and multiplayer still need game validation. In particular, stripped logs do not yet establish that axe stripping works.

Clear glass uses the original Mantle connected-texture model. That loader is not yet connected to Horizon, so clear-glass rendering is still to be implemented. Crafting Station and Modifier Worktable have placement and appearance data, but their menus and storage are not implemented. Their current collision uses the full-cube template and does not yet reproduce the table legs. Slime crystal blocks use an amethyst sound template; their source-specific chimes are not connected. Other blocks and machine appearances are still in progress.

The remaining source visuals include connected glass (`mantle:connected`) and layered slime-metal blocks (`forge:composite`); those loaders are not yet connected to Horizon. Congealed slime also has a smaller collision shape and bounce behavior, so a full solid cube would not represent it accurately. Tinkers' chests and material-dependent anvils need their own storage or material handling. These blocks are not included in the current registration data.
