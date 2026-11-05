### Fluid-effect selection isolation

Source `FluidEffectManager.apply` evaluates conditions before parsing ingredient
and effect types and catches malformed definitions independently. The native
selector now skips disabled definitions before looking at their fluid shape,
and isolates invalid JSON, non-object roots, malformed condition-array fields,
missing fluid fields and non-string direct fluid/tag IDs. Only the highest
resource layer is considered; an invalid override does not revive a lower pack.

This restores supported Slurping/Spilling selections that an unrelated invalid
or disabled file previously prevented. Unknown conditions on a matching fluid,
complex/unsupported ingredient shapes and unsupported first matching effects
remain fail-closed; this is not a complete implementation of every source
serializer. Existing sorted-key precedence is unchanged and not claimed to
match every source duplicate-match order. The fixture compiles the production
module against pinned host NBT and actual potion-registry exports. Live fluid
consumption, pack reload, game interactions and LAN are not verified here.

Source: [FluidEffectManager at f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/fluid/FluidEffectManager.java).
