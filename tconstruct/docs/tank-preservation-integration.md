# Smelting, casting and fluid preservation integration

This first functional batch integrates machine-state preservation work onto `main`.
The snapshot cutoff is PR #922 at 2026-11-05 13:56 UTC. Later implementation
work and world-fluid/host feature branches remain separate. This does not
complete the broad #379 checklist.

## Included heads

The following exact heads are preserved as ancestors, not squashed copies.

- [#684](https://github.com/sofia-gros/horizon-mods/pull/684): `ee7c0bf297fd020d6e1abc6514411fd7b96bff78` (Consume blocked container interactions at formed Drains).
- [#692](https://github.com/sofia-gros/horizon-mods/pull/692): `ace3fb1bdc3f3d1276695957092855f82b3b475e` (fix(tconstruct): preserve casting destination metadata safely).
- [#832](https://github.com/sofia-gros/horizon-mods/pull/832): `a5f9e6c1313e6b2f2604f03d73c520227a0879dd` (fix(tconstruct): preserve metadata during held casting container transfers).
- [#834](https://github.com/sofia-gros/horizon-mods/pull/834): `d6b710fdd9b2376628a79081d582c6829dc501c2` (fix(tconstruct): preserve portable Lantern tank extensions).
- [#836](https://github.com/sofia-gros/horizon-mods/pull/836): `86867533d31f09e0adfca94252d63caf251aa3bd` (fix(tconstruct): reject ambiguous Faucet metadata before routing).
- [#838](https://github.com/sofia-gros/horizon-mods/pull/838): `177a77db5edca965ae4da9fc813a8e0750646e77` (fix(tconstruct): reject lossy and opaque Copper Can rewrites).
- [#840](https://github.com/sofia-gros/horizon-mods/pull/840): `51a07aebb604414f8781b8ec8fb9f610f7d8f7c4` (fix(tconstruct): preserve controller data during held Drain transfers).
- [#841](https://github.com/sofia-gros/horizon-mods/pull/841): `7b55f4882ff03eea54b87530a2c255eaf02c77f4` (fix(tconstruct): preserve Cannon data during held fluid exchange).
- [#844](https://github.com/sofia-gros/horizon-mods/pull/844): `2d09578cc13ec8424f039b365e8ab0d69b78082b` (fix(tconstruct): preserve Cannon metadata across item saves).
- [#846](https://github.com/sofia-gros/horizon-mods/pull/846): `8647bc5d2a6a92d9d7e08068cfa37c6ceed15852` (fix(tconstruct): preserve Casting Tank metadata and slots).
- [#848](https://github.com/sofia-gros/horizon-mods/pull/848): `50c0136a8a66dda5861bc93e5f30bac0f3743d20` (fix(tconstruct): guard Casting Tank interaction snapshots).
- [#849](https://github.com/sofia-gros/horizon-mods/pull/849): `e678dc43200eb5c3c3d155caf88e0d0293d40dbc` (fix(tconstruct): guard Cannon hopper callback endpoints).
- [#852](https://github.com/sofia-gros/horizon-mods/pull/852): `698089600d55afd0352f0da72b619fb7c1c01deb` (fix(tconstruct): preserve native controller tank row metadata).
- [#853](https://github.com/sofia-gros/horizon-mods/pull/853): `839137ac92f7b603fbd57831afb7b23fefa03d10` (fix(tconstruct): preserve controller metadata in fluid menu actions).
- [#856](https://github.com/sofia-gros/horizon-mods/pull/856): `3cb745b58cfc012853c5164b57068b5bd50b1216` (fix(tconstruct): preserve carried fluid clicks and exact layer identity).
- [#857](https://github.com/sofia-gros/horizon-mods/pull/857): `6cff411a8b6b4d33e481558c6906cbaa514d0341` (fix(tconstruct): preserve direct Melter and fuel tank exchanges).
- [#859](https://github.com/sofia-gros/horizon-mods/pull/859): `126af15fe2de50303827dd43e1fe3c857561b636` (fix(tconstruct): preserve metadata in fluid container slot commits).
- [#860](https://github.com/sofia-gros/horizon-mods/pull/860): `298856c3e313b08599d0cf7af5b653d699a9b12d` (fix(tconstruct): preserve staged melting slot inventory commits).
- [#863](https://github.com/sofia-gros/horizon-mods/pull/863): `b68616ab77ad9f05a8f354aa940ece64c5ed09f6` (fix(tconstruct): guard Cannon item clicks against opaque inventory merges).
- [#867](https://github.com/sofia-gros/horizon-mods/pull/867): `9d2ba7fc0d2757a8899497a45c8a02edfa0a0e8b` (fix(tconstruct): preserve held fuel tank exchanges across callbacks).
- [#869](https://github.com/sofia-gros/horizon-mods/pull/869): `f3498ffb4e0d4327bf31e1e17fee5ecee3266f2b` (fix(tconstruct): preserve ambiguous tanks during liquid-fuel withdrawal).
- [#870](https://github.com/sofia-gros/horizon-mods/pull/870): `df7efd895da89e0a2dfeb32873b97eeb55397b10` (fix(tconstruct): guard casting item fallback after fluid probes).
- [#872](https://github.com/sofia-gros/horizon-mods/pull/872): `6d8e058038f4b753eac3c991257a8d8d264ea3c2` (fix(tconstruct): guard Melter held transfers and menu fallback).
- [#900](https://github.com/sofia-gros/horizon-mods/pull/900): `0f3aed6da7c1131774d7b3ffa0026b579b6dae49` (fix(tconstruct): stage lossless Heater fuel and remainder consumption).
- [#918](https://github.com/sofia-gros/horizon-mods/pull/918): `1636a304a15725cbbe8139fdbcb727ef530a27aa` (fix(tconstruct): preserve native tank payloads through drops and placement).
- [#922](https://github.com/sofia-gros/horizon-mods/pull/922): `1ba0c453ebf89eb0e696f09e5d5ff5d760bb8813` (fix(tconstruct): preserve portable tank casting recipe metadata).

## Integration changes

- Keep all existing CI steps and add the shared Cannon/Casting Tank production
  fixture to CI alongside Drain and casting preservation fixtures.
- Compose both Cannon and Casting Tank fixture state, callbacks, native inventory
  handling and required cases. Remove duplicate state fields from that merge.
- Update the legacy Melter wiring expectation to the new preserving fuel-debit
  helper. Behavioral coverage remains in the production tank and route fixtures.
- Preserve both documentation additions and the existing main-side fluid effect
  selection validation.

## Validation and scope

Local validation uses the unchanged pinned Horizon `0c7b5b61a5e746d621afffba919b12aeccf9c0fa`
and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`.
Drain/Melter/portable tank fixture: 178 tests passed. Shared Cannon/Casting Tank
fixture: 160 passed. Casting persistence and interaction fixture: 58 passed.
The suites share some unit cases, so these counts are not a unique-case total.
An independent integration review found the duplicate fixture fields, corrected
here, and no further concrete serious regression in the reviewed production code.

The first full Python run found one obsolete wiring expectation; it was corrected.
The final Python rerun, generated-host build and exact-head CI results are reported
in the integration pull request rather than implied by older per-feature notes.
The underlying fixtures use real item/NBT/state implementations with bounded
world/storage/recipe callbacks. They do not establish full game UI, destruction,
save/reopen or LAN behavior. Prepared game resources are unavailable locally.
