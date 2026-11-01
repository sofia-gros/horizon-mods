# Kama and Scythe harvest

This draft covers source crop harvest and replanting, Scythe area mining, and
source-conditioned shears loot for Kama and Scythe. Each successful action
consumes tool durability and stops on breakage. Server spawn protection applies
to every affected position. Four-part survival assembly and Silky acquisition
remain unfinished; Silky-dependent behavior must not be granted by default.
Combat, armor and projectile-specific actions remain on the roadmap.

Implementation and in-game verification are pending.

## Implementation status

The scythe has been connected to the existing area-break path in source code;
this change has not yet received a formal transform, build, or game test. Crop
harvesting and replanting are not implemented. The shearing selector in
`source-shear.rs` is an unconnected prototype and does not affect loot.

The official Mixins CLI at `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab`
rejected a `HEAD` injection into
`horizon_items::items::LootContext<'_>::condition` with
`E_UNSUPPORTED_SIGNATURE`: the target has the generic parameter
`R: RandomSource`. The rejected injection was removed. A supported way to
change only the `minecraft:tool/can_shear` capability needs to be established;
substituting a shears item into the entire loot context would also change other
conditions and is not an acceptable substitute. This is a constraint on the
attempted target, not proof that all alternative hooks are unavailable.
