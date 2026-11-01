# Material trait expansion (in progress)

This unit follows the original Tinkers' Construct revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` and the existing material-trait dispatcher.

- **Tanned:** a leather trait limits ordinary durability loss to one point before Overslime and later wear modifiers. Source secondary modifier causes have a separate zero-loss rule; those modifier-driven actions are not yet connected.
- **Stringy:** a string trait adds String as a repair material, with the source repair value 140 per trait level before the existing material and repair multipliers.
- **Enhanced:** a Rose Gold trait grants one upgrade slot per level through the shared Station and Anvil transaction rules. A part replacement is rejected before consumption if the rebuilt tool would have negative free slots or invalidate Unbreakable's prerequisites.

The lightweight arithmetic and transaction checks do not establish full game behavior. Glowing's source effect places light blocks through several distinct interaction paths and remains unimplemented; its explicitly identified wear bypass remains reserved for that future action. Many other material effects, unrelated equipment and modifier causes, and Windows remain open. Existing saved items and part replacements need representative tests.
