# Provisional development build

This build temporarily disables features whose Horizon Mixins targets cannot currently be transformed. It is not a complete port. The disabled injection definitions are retained in `disabled-injections.toml`; restoring them requires a compatible official Mixins implementation and renewed testing.

| Feature | Availability | Current limitation |
| --- | --- | --- |
| Reinforced and Unbreakable | New application disabled | Ordinary wear prevention is not connected. Existing modifier data is retained but does not provide the intended protection. |
| Dense, Reinforced and Unbreakable material traits | New additions disabled | The wear effect is unavailable; existing tools retain their data but do not receive that wear prevention. |
| Wearable material armor | New assembly disabled | Wear and armor Overslime consumption are not connected. Existing armor remains in saves but its durability behavior is incomplete. |
| Crafting Station | Disabled | Its persistent grid click synchronization is unavailable. Existing block data is retained; the original recipe is preserved under `disabled-recipes/`. Use a vanilla crafting table. |
| Material-specific storage rendering and connected glass | Appearance disabled | Storage material data remains, but material-specific world faces, connected glass faces and Tinker's Chest world tint are unavailable. The chest dye recipe is disabled; existing dyed items retain their data. |
| Excavator and Battlesign built-in knockback | Disabled | Their extra built-in melee knockback is unavailable. Other tool behavior is separate. |
| Crumbling with Stonebound | Incomplete | An isolated mining test did not match the expected ordering; the combination has not been validated. |

Previously captured game tests describe the revisions stated alongside them. They do not verify the full provisional build. The Part Chest and Tinker's Chest transactions were tested with the eight affected injections omitted; their world appearance and Crafting Station integration were not covered. See [chest details](tinker-part-chests.md) and [host update evidence](host-upgrade.md).
