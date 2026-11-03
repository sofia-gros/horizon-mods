# Current limitations

This page records known unavailable and unverified operations. Specific hooks that the tested Horizon Mixins version cannot transform are retained in `disabled-injections.toml`; restoring those hooks requires a compatible boundary and renewed testing. Other incomplete or unverified mod behavior remains open work and is not automatically an upstream limitation. Registered items and blocks remain available for existing worlds.

| Feature | Status and limitation |
| --- | --- |
| Reinforced and Unbreakable | New application is temporarily disabled because ordinary wear prevention is unavailable. Existing modifier data is retained but does not give the intended protection. |
| Dense and other durability-prevention traits | The affected wear effect is unavailable; material items and tool construction remain available. |
| Wearable material armor | Armor remains available. Its material armor and toughness calculation is separate from the disabled dynamic wear and armor Overslime writeback; those durability effects are incomplete. |
| Crafting Station | The screen and recipe remain available. On the local host, grid clicks use a synchronized transaction; normal and Shift-click results, ingredient persistence, container remainders, drag, number-key swap and outside drop passed representative Linux tests; representative Linux recipe-book placement and rejection checks passed locally and for a LAN guest after a prior item-loss failure; Windows and every click combination remain unverified. Representative LAN guest placement and concurrent host/guest placement also passed. |
| Material-specific storage rendering and connected glass | Storage material data remains, but material-specific world faces, connected glass faces and Tinker's Chest world tint are unavailable. Dyed items keep their data. |
| Excavator and Battlesign built-in knockback | Their extra built-in melee knockback is unavailable; their other behavior is separate. |
| Overwield mining speed for LAN guests | A remote client does not receive the host-only broad-tool tag snapshot used by this helper, so its Overwield speed bonus can be skipped. Track this separately in [#363](https://github.com/sofia-gros/horizon-mods/issues/363); the local/host snapshot path is separate. |
| Crumbling with Stonebound | An isolated mining test did not match the expected ordering; the combination has not been validated. |

Previously captured game tests describe the revisions stated alongside them. They do not verify every feature in the current integrated build. An earlier client passed representative local and LAN Part Chest / Tinker's Chest transactions; the affected world appearance and Crafting Station integration were not covered. See [chest details](tinker-part-chests.md) and [host update evidence](host-upgrade.md).

The machine-readable [current target](../compatibility.json) is Horizon
`0c7b5b61a5e746d621afffba919b12aeccf9c0fa` with Mixins
`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. A target pin is not proof that
every operation has passed build or game validation.

An earlier combined Linux build on Horizon `fd6cfda6bdef59a53aee830009f78c14832b0308` and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a` passed representative game checks. That evidence does not establish the current target; advanced durability text for some component-bearing tools remains incomplete. See [compatibility](compatibility.md) for the tested scope and the earlier combination.
