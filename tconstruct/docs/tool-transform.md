# Tool block transformations

The intended right-click actions are tilling with a Mattock, Kama or Scythe,
pathing with an Excavator, and stripping, scraping or wax removal with a Hand
Axe or Broad Axe. The implementation reads the held material tool and uses
Horizon's existing transformation rules for the target block. A tool that has
broken is kept and cannot keep transforming blocks.

Unexpanded Scythes use the source side-hit box; Excavators use the source
side-hit plane. Broad Axes follow the source tree or vertical non-tree area.
Each successful transformation consumes durability outside Creative. On a hosted
server the existing spawn-protection policy is checked for every candidate,
including tools in the off hand. Pathing also attempts the source behavior for
replaceable vegetation above the block. Client prediction ends the interaction
on a successful transform so that the other hand does not act as well.

**Verification status:** the changes and client prediction are implemented but
have not passed an official full transformation, Rust type check, or in-game
validation. Targeted source mapping, geometry and server-protection tests pass;
they are not proof of game behavior. Test normal and LAN uses, conditions above
the block, transformation areas, durability and breakage, offhand behavior,
spawn boundaries, save/reload, and vanilla tool regression using one build.

Creative currently supplies four-part tools; their survival assembly remains
unfinished. Expanded modifiers, crop harvesting/replanting, Scythe attacks,
walking transformations and campfire extinguishing are separate unfinished
behaviors. Armor and projectile-specific actions and Windows verification also
remain open.
