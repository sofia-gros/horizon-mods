# Magma Bottle

The pinned source Magma Bottle holds 250 mB of magma. Its Casting Table recipe uses a glass bottle and magma, consuming the cast.
This candidate registers the result and adds it to the supported fluid-container
transfer path. The
item stacks to 16 and has a glass-bottle crafting remainder. Source artwork
and the item model are retained.

Drinking is intentionally unavailable in this candidate. The source uses the
item for 32 ticks, sets the drinker on fire for 15 seconds and returns a glass
bottle unless the player has infinite materials. The host's consumable effect
hook has no way to cancel consumption if `ignite_for_ticks` fails while
resolving the burning-time attribute. Enabling drinking before an atomic
failure path is established could consume the bottle without its effect.
Creative, stacked-item and LAN drinking are therefore not claimed or enabled.

Tests cover the item registration and resource packaging, as well as 250 mB
transfer and rejection without loss for full tanks, blocked output and unknown
fluids. Casting, container transfer, saving and LAN behavior still require
game verification. Other slime bottle effects remain separate.
