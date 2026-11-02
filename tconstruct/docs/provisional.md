# Provisional development build

This build temporarily disables specific hooks whose Horizon Mixins targets cannot currently be transformed. It is not a complete port. Their definitions are retained in `disabled-injections.toml`; restoring them requires a compatible official Mixins implementation and renewed testing. Registered items and blocks remain available for existing worlds.

| Feature | Status and limitation |
| --- | --- |
| Reinforced and Unbreakable | New application is temporarily disabled because ordinary wear prevention is unavailable. Existing modifier data is retained but does not give the intended protection. |
| Dense and other durability-prevention traits | The affected wear effect is unavailable; material items and tool construction remain available. |
| Wearable material armor | Armor remains available. Its material armor and toughness calculation is separate from the disabled dynamic wear and armor Overslime writeback; those durability effects are incomplete. |
| Crafting Station | The screen and recipe remain available. On the local host, grid clicks use a synchronized transaction; normal and Shift-click results, ingredient persistence, container remainders, drag, number-key swap and outside drop passed representative Linux tests; recipe-book auto-placement is temporarily blocked. LAN guests use a separate path and have not been verified for this revision. |
| Material-specific storage rendering and connected glass | Storage material data remains, but material-specific world faces, connected glass faces and Tinker's Chest world tint are unavailable. Dyed items keep their data. |
| Excavator and Battlesign built-in knockback | Their extra built-in melee knockback is unavailable; their other behavior is separate. |
| Crumbling with Stonebound | An isolated mining test did not match the expected ordering; the combination has not been validated. |

Previously captured game tests describe the revisions stated alongside them. They do not verify the full provisional build. The provisional product client passed representative local and LAN Part Chest / Tinker's Chest transactions; the affected world appearance and Crafting Station integration were not covered. See [chest details](tinker-part-chests.md) and [host update evidence](host-upgrade.md).

The previously verified combination uses Horizon `cabe8b5de7d0b45541c923c4a6b3c5cb3e3686c7` and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. The current development target is Horizon `fd6cfda6bdef59a53aee830009f78c14832b0308` with the same Mixins revision; the matching PlutoLib integration and game checks are pending.
