# Part Chest and Tinker's Chest

This feature is in progress. It targets the original TConstruct revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` and is not yet implemented.

The original Part Chest accepts only tagged parts, keeps at most eight identical item-and-component variants per slot, forbids duplicate variants in separate slots, and has up to 256 slots with a growing visible inventory. The Tinker's Chest has 64 slots of up to 16 items each. Their original shaped recipes use a wooden chest, pattern and planks; the Part Chest uses wooden rods and the Tinker's Chest uses lapis.

The implementation scope is registration, source recipes and appearance, inventory rules and scrollable UI, persistent storage and destruction behavior, and use as an adjacent station inventory, including client/server and LAN transactions. Existing adjacent vanilla chest behavior must remain intact. Exact insertion, removal, shift-click, filtering, and source chest interactions will be checked against the source before implementation.

Representative acceptance will cover natural crafting and placement, limits and component identity, rejected items and full inventories, manual and shift-click transfers from adjacent Part Builder and Station, save/reload, destruction and recovery, LAN host and guest access, and an ordinary chest regression. No new heavy build or game test has been run for this unit.

The shared source-specific slot limits, acceptance decisions, and growing Part Chest display size are modeled and lightly tested. They are not yet connected to a placed block, menu, transfers, or saving.
