# Block registration

This work adds the original Tinkers' Construct blocks to Horizon in groups that share placement and rendering behavior. The first goal is to place, see, break, recover and save the registered blocks, including in a LAN world.

The source pack has separate blockstate files for some fluids, wall-mounted forms and other variants. These files are not a count of unique inventory items. Registration will distinguish block IDs, item IDs and state variants, and reuse existing Minecraft resources and recipes where they already provide the same material.

The scope includes decorative and storage blocks; wood shapes such as slabs, stairs, fences and doors; glass and panes; and the appearance and placement of workstations and machines. Original assets and current Horizon shapes are used where they support the source block. Representative tests will cover placement and orientation, appearance and collision, drops, save and reload, and LAN synchronization.

A registered machine is not necessarily functional. Smelting, casting, alloying, their interfaces, fluid behavior, plant growth and world generation are separate work. Any shape or action that Horizon cannot yet represent will be identified rather than presented as working.
