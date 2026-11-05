# Slime-fluid drowning policy and entity tag

Adapted from [Tinkers' Construct](https://github.com/SlimeKnights/TinkersConstruct)
at commit `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`.
Copyright (c) 2022 SlimeKnights. MIT License, reproduced in
[tconstruct/LICENSE](tconstruct/LICENSE).

The `SlimeFluidType.canDrownIn` policy, its five registrations and the generated
`forge:slimes` entity-type tag are retained. Changes: explicit Horizon
`drowning_immune_tag` declarations and the host's singular `tags/entity_type`
resource directory. The entity membership values are unchanged. Source paths
and scope are recorded in `content/reference/world-fluid-drowning-source.json`.
No Forge implementation was read or copied.
