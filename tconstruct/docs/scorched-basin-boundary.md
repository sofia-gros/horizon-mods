# Scorched Basin subset

Pinned source: TConstruct f4e77d43f39ba1047bf93ae816b7e1beaab514c2. The Scorched Basin uses the same Basin block entity, shape and recipe-driven tank capacity as the Seared Basin, but requires a physical cast. This implementation routes it through the existing basin machinery, including persistence, hopper, faucet and drop/spill paths. Those paths retain their existing host limitations; this is not a game-verified result.

The source recipe is `# # / #G# / ###`, where # is scorched brick and G is gold platform. A bounded Gold Platform item and the source gold-tag recipe provide the ingredient and the Basin empty cast. The item is intentionally not placeable: source platform connection, placement, waterlogging, neighbor updates and 64 shapes remain unimplemented; see f11-state-boundaries.md. Tag reload and LAN recipe behavior retain their existing limits, and the actual survival crafting and cast paths are unverified.

The item uses the pinned covered model. The host template does not expose source's `has_item` state, so the placed static model uses the open basin model, as the current Scorched Table similarly does. Visual state parity remains open.

The held-container and Faucet-resume paths from #560 recognize this basin as a
casting target, including retaining a reserved packet when its state is
unreadable. Their existing conservative metadata restrictions still apply.
Tests cover state transitions and static routing; actual host registration,
save/drop behavior, hopper, held-container transfers, and LAN remain unverified.

Gold Platform is currently an item-only partial implementation, not a placeable
platform block. Its localized default item name warns that placement is
unavailable; a user-supplied custom name can hide that warning. The source model
is used for the item. Survival crafting, item-name rendering, casting, resource
reload and LAN are not yet verified against the actual host. Adding the real
block later must reconcile this temporary same-ID item declaration.
The source Basin cast tag resolves with the new item in a standalone tag test,
and a higher-priority replacement removes it there; this does not establish
live world reload or LAN synchronization.

The content root supplies the source Forge gold ingot and nugget tags for the
host crafting loader. A fixture using the cached host Items catalog checks root
tags and a higher-priority overlay addition and replacement; an extracted copy
of the host string-ingredient parser was also checked against that catalog.
These are not a full content build, in-world reload or LAN test. Other mods may
change the tag contents and recipe availability.
