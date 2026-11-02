# Compatibility

## Verified combination

- **Tinkers’ Construct for Horizon:** development version `0.1.0`; representative provisional tests used code [`ed70aa9`](https://github.com/sofia-gros/horizon-mods/commit/ed70aa9baedc33c6613733c4f5e560420aede457). This is not a release claim.
- **Horizon:** [`cabe8b5de7d0b45541c923c4a6b3c5cb3e3686c7`](https://github.com/horizon-rust/horizon/commit/cabe8b5de7d0b45541c923c4a6b3c5cb3e3686c7).
- **Horizon Mixins:** [`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`](https://github.com/horizon-rust/horizon-mixins/commit/ddd9700b21997c95d95ac6ee5ecafe3830cf622a).
- **PlutoLib:** unified native bundle built from [`5520a221655e9ea5b55b9ce32a46bca511c3ae71`](https://github.com/sofia-gros/horizon-mods/commit/5520a221655e9ea5b55b9ce32a46bca511c3ae71).
- **Platform and scope:** the provisional Linux client without audio built and passed representative local Crafting Station and local/LAN Chest tests. Exact scope and code revisions are in the [host update evidence](host-upgrade.md). The build temporarily omits eight injections; see [provisional limitations](provisional.md). Windows, smartphones, dedicated servers and full survival gameplay remain unverified.

Earlier chest and guide tests used Horizon [`b6c936f82b08798057161bdfa577e3aac53fb91e`](https://github.com/horizon-rust/horizon/commit/b6c936f82b08798057161bdfa577e3aac53fb91e) with TConstruct runtime [`956fbb6`](https://github.com/sofia-gros/horizon-mods/commit/956fbb697c9792d875db87da0967540fd3d02caa) and resource changes through [`97e9772`](https://github.com/sofia-gros/horizon-mods/commit/97e9772210dda9976053a17f0250213734ca420b).

The source TConstruct data is pinned to [`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`](https://github.com/SlimeKnights/TinkersConstruct/commit/f4e77d43f39ba1047bf93ae816b7e1beaab514c2). The machine-readable Horizon and Mixins pins are in [`compatibility.json`](../compatibility.json). Mixins and the separate PlutoLib bundle are required.

## Development target

This branch targets Horizon [`fd6cfda6bdef59a53aee830009f78c14832b0308`](https://github.com/horizon-rust/horizon/commit/fd6cfda6bdef59a53aee830009f78c14832b0308) with Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. This pair and the matching PlutoLib bundle are **not yet validated together**. The verified combination above remains the previous pair. Check upstream again before merging; see the [host update evidence](host-upgrade.md). Slime-island generation and source world fluids remain incomplete.
