# Material trait expansion (in progress)

> **Provisional build:** Some effects and operations described below are unavailable or under verification. Earlier test results refer to their stated revisions; see [current limitations](provisional.md).

![Station rejecting a part swap that would leave too few upgrade slots](enhanced-swap-rejection.png)

The screenshot uses supplied parts and upgrades in the Linux test world.

This unit follows the original Tinkers' Construct revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` and the existing material-trait dispatcher.

Tool definitions with supported material stats now also contribute their fixed `tconstruct:traits` entries in module order. This resolves, for example, the Pickaxe's Pierce and the boots' Snow Boots or Bouncy traits; resolving a trait does not itself implement every effect. Check the combined fixed and material levels, part replacement and save/reload in the target build. Optional external traits and definitions without a supported material-stat module are not established by this change.

Traveler's Boots' Snow Boots flag now updates the existing player powder-snow walking state after local body synchronization and equipment changes, including while the boots are broken. The separate block-placement collision context receives the same predicate. The original vanilla leather-boots check remains included. These changes are not yet verified in a target build; check movement onto and through powder snow, block placement, crouching, equipping, removing, breaking and repairing the boots, and vanilla boots in local and guest play. Slime Boots bounce remains unimplemented.

- **Tanned:** a leather trait limits ordinary durability loss to one point before Overslime and later wear modifiers. Source secondary modifier causes have a separate zero-loss rule; those modifier-driven actions are not yet connected.
- **Stringy:** a string trait adds String as a repair material, with the source repair value 140 per trait level before the existing material and repair multipliers.
- **Enhanced:** a Rose Gold trait grants one upgrade slot per level through the shared Station and Anvil transaction rules. A part replacement is rejected before consumption if the rebuilt tool would have negative free slots or invalidate Unbreakable's prerequisites.

Representative Linux tests built a leather-binding pickaxe from supplied parts, checked ordinary melee wear and shield-before-durability, and retained its material and shield after reload. Supplied Stringy tools repaired from damage 50 to 4 using four String locally and for a LAN guest; a leather-binding tool rejected that repair. A supplied Rose Gold pickaxe used its extra slot, rejected an over-budget head replacement without consuming inputs, and accepted a replacement once the used slots fit. The #50 surplus-bottle correction also retained a supplied bottle's name and custom data in a game crafting transaction.

Glowing's source effect places light blocks through several distinct interaction paths and remains unimplemented; its explicitly identified wear bypass remains reserved for that future action. Source secondary modifier-caused actions, many other material effects, unrelated equipment and Windows remain open. The tests did not collect the supplied materials naturally or exercise every source tool.
