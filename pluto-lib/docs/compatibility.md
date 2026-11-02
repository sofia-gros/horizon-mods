# Compatibility

PlutoLib is unreleased; its Cargo manifest declares `0.1.0`. Each native bundle targets a specific Horizon revision. Other revisions and platforms are not covered by these checks.

## Verified combinations

On Linux, the [native Creative demo](native-creative.md) was built and run at PlutoLib development commit [`0b0b40e89c356e2bcdbabed5ffa7d5b7a68302dc`](https://github.com/sofia-gros/horizon-mods/commit/0b0b40e89c356e2bcdbabed5ffa7d5b7a68302dc) with:

- Horizon [`5afa1de5f750b4b89ca23347b1d3aba296c278b0`](https://github.com/horizon-rust/horizon/commit/5afa1de5f750b4b89ca23347b1d3aba296c278b0)
- Horizon Mixins [`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`](https://github.com/horizon-rust/horizon-mixins/commit/ddd9700b21997c95d95ac6ee5ecafe3830cf622a)

The standalone preparation wrapper uses the older `horizon-items` parser pinned in `pluto-lib`; the native demo uses its target host's parser. Combined Tinkers integration and Windows remain unverified.

The older [Creative paging adapter](creative-overflow.md) and [item behavior demo](item-behaviors.md) were built and run on Linux against Horizon [`b6c936f82b08798057161bdfa577e3aac53fb91e`](https://github.com/horizon-rust/horizon/commit/b6c936f82b08798057161bdfa577e3aac53fb91e) with the same Mixins revision. The first registration demo used this Horizon revision without Mixins. Their checks do not establish compatibility with the newer host.

## Development target

The native Creative bundle and package target Horizon [`3a264943c7a3a3ed02e6ee608443f558d98f208f`](https://github.com/horizon-rust/horizon/commit/3a264943c7a3a3ed02e6ee608443f558d98f208f) with Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. On Linux, the independent demo built and ran with this pair: page switching and retrieval of a component-bearing stack were checked. The standalone package also passed the official native bundle check. Combined Tinkers integration remains unverified. Rebuilding or source changes may be necessary when changing the Horizon or Mixins revision.

The independent component-transport demo was also built on Linux with the `3a264943` / `ddd9700` pair. Its [validation and known display limitation](component-transport.md) are separate from Tinkers integration. Upstream Horizon `848a87b7319dc8c799bc0e812368ade7c8e8baa8` was observed later but has not been tested with this prototype.
