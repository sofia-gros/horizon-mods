# Material trait expansion (in progress)

> **Provisional build:** Some effects and operations described below are unavailable or under verification. Earlier test results refer to their stated revisions; see [current limitations](provisional.md).

![Station rejecting a part swap that would leave too few upgrade slots](enhanced-swap-rejection.png)

The screenshot uses supplied parts and upgrades in the Linux test world.

This unit follows the original Tinkers' Construct revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` and the existing material-trait dispatcher.

- **Tanned:** a leather trait limits ordinary durability loss to one point before Overslime and later wear modifiers. Source secondary modifier causes have a separate zero-loss rule; those modifier-driven actions are not yet connected.
- **Stringy:** a string trait adds String as a repair material, with the source repair value 140 per trait level before the existing material and repair multipliers.
- **Enhanced:** a Rose Gold trait grants one upgrade slot per level through the shared Station and Anvil transaction rules. A part replacement is rejected before consumption if the rebuilt tool would have negative free slots or invalidate Unbreakable's prerequisites.
- **Vintage armor attributes:** each equipped armor piece applies the original per-level movement-speed multiplier of -10% while unbroken and gravity multiplier of +10% through the separate equipment-change behavior. This is connected to the local equipment attribute path; handheld gravity, offhand attack-speed effects and LAN host behavior remain unverified or unimplemented. Test multiple pieces, removal, breakage and interactions with other attribute modifiers in the target build.
- **Heavy armor attributes:** each equipped piece applies -10% movement speed while unbroken using the original multiply-base operation and +5% gravity using the separate equipment-change behavior, per level. The local equipment path requires runtime verification; held-item effects and LAN host behavior are separate.
- **Featherweight armor movement:** a source armor-tagged piece adds 5% movement speed per level using multiply-base through the local equipment path. Its protection penalty and item-use speed effect are separate and remain unimplemented. Test the source tag, multiple pieces, equipment removal, breakage and interactions with Heavy; LAN host behavior is unverified.
- **Solid and Entwined armor movement:** Solid applies -10% movement using multiply-base while unbroken and +5% gravity using the separate equipment-change behavior per level; Entwined applies +10% movement using multiply-total per level while unbroken. This covers the local armor attribute path only. Entwined's unequip damage, Solid's armor protection, held-item attributes and LAN host behavior remain separate. Solid's Invar material requires its original external integration conditions. These effects need target-build testing.

Representative Linux tests built a leather-binding pickaxe from supplied parts, checked ordinary melee wear and shield-before-durability, and retained its material and shield after reload. Supplied Stringy tools repaired from damage 50 to 4 using four String locally and for a LAN guest; a leather-binding tool rejected that repair. A supplied Rose Gold pickaxe used its extra slot, rejected an over-budget head replacement without consuming inputs, and accepted a replacement once the used slots fit. The #50 surplus-bottle correction also retained a supplied bottle's name and custom data in a game crafting transaction.

Glowing's source effect places light blocks through several distinct interaction paths and remains unimplemented; its explicitly identified wear bypass remains reserved for that future action. Source secondary modifier-caused actions, many other material effects, unrelated equipment and Windows remain open. The tests did not collect the supplied materials naturally or exercise every source tool.
