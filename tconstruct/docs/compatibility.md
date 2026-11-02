# Compatibility

## Verified combination

- **Tinkers’ Construct for Horizon:** development version `0.1.0` in the Mixins and content manifests; this is not a release claim. Representative chest and guide integration tests used runtime commit [`956fbb6`](https://github.com/sofia-gros/horizon-mods/commit/956fbb697c9792d875db87da0967540fd3d02caa) with the tested resource changes through [`97e9772`](https://github.com/sofia-gros/horizon-mods/commit/97e9772210dda9976053a17f0250213734ca420b).
- **Horizon:** [`b6c936f82b08798057161bdfa577e3aac53fb91e`](https://github.com/horizon-rust/horizon/commit/b6c936f82b08798057161bdfa577e3aac53fb91e).
- **Horizon Mixins:** [`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`](https://github.com/horizon-rust/horizon-mixins/commit/ddd9700b21997c95d95ac6ee5ecafe3830cf622a).
- **Platform and scope:** the manual Linux build without audio and representative local/LAN paths were tested. Windows, smartphone builds, dedicated servers, the native-bundle distribution path, and many source features remain unverified. See the [roadmap](roadmap.md) and individual guides for exact coverage.

The source TConstruct data used for this port is pinned to [`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`](https://github.com/SlimeKnights/TinkersConstruct/commit/f4e77d43f39ba1047bf93ae816b7e1beaab514c2). The repository’s machine-readable host and source pins are in [`compatibility.json`](../compatibility.json); the Mixins manifest records the host pin. Mixins is required by this mod’s native implementation.

## Development target

The separate [Horizon update draft](https://github.com/sofia-gros/horizon-mods/pull/59) investigates official Horizon [`e12185820a6bc85dd5ed04adc7b8b7a4bdc16102`](https://github.com/horizon-rust/horizon/commit/e12185820a6bc85dd5ed04adc7b8b7a4bdc16102) with the same Mixins revision. This is an adaptation target, not a supported or validated combination. Slime-island generation and source world fluids remain incomplete.
