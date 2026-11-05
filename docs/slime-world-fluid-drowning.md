# Slime-fluid drowning immunity

Continues high-priority world-fluid contact in #379, stacked on the world-fluid
path policy draft #866. Requires [host draft #60](https://github.com/sofia-gros/horizon/pull/60)
and its dependencies. The host pin is unchanged.

Earth Slime, Sky Slime, Ender Slime, Magma and Ichor now explicitly name
`forge:slimes` as their drowning-immune entity tag. They retain `can_drown: true`
for other entities. The tag has exactly the source's vanilla slime/magma cube
and TConstruct sky slime/ender slime/terracube members. The existing content
manifest already allows the forge namespace; no namespace permission is added.

This follows MIT TConstruct `SlimeFluidType.canDrownIn`, the `registerSlime`
registrations, TinkerTags' common tag alias and the generated tag at
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. Attribution and source paths are in
`tconstruct/third_party/world-fluid-drowning-NOTICE.md` and the provenance JSON.
No Forge implementation was read or copied.

Other cool fluids and Molten Cinderslime are not given this exception. The tag
does not register missing custom slime entities, grant native-water immunity,
or change player/armor-stand air handling. Item buoyancy remains separate.

Brief static review and `git diff --check` only. Two tests were added for exact
policy/tag scope and package pass-through, but were not run. The provenance JSON
is repository reference data, not a newly packaged runtime resource. Builds,
tests, CI/version checks, integration and merge remain with the separate batch
workflow. No full world-fluid parity or checklist completion is claimed.
