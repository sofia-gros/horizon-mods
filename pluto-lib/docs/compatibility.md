# Compatibility

PlutoLib is unreleased; its Cargo manifest declares `0.1.0`. Each native bundle targets a specific Horizon revision. Other revisions and platforms are not covered by these checks.

## Verified combinations

On Linux, the [native Creative demo](native-creative.md) was built and run at PlutoLib development commit [`0b0b40e89c356e2bcdbabed5ffa7d5b7a68302dc`](https://github.com/sofia-gros/horizon-mods/commit/0b0b40e89c356e2bcdbabed5ffa7d5b7a68302dc) with:

- Horizon [`5afa1de5f750b4b89ca23347b1d3aba296c278b0`](https://github.com/horizon-rust/horizon/commit/5afa1de5f750b4b89ca23347b1d3aba296c278b0)
- Horizon Mixins [`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`](https://github.com/horizon-rust/horizon-mixins/commit/ddd9700b21997c95d95ac6ee5ecafe3830cf622a)

The standalone preparation wrapper uses the older `horizon-items` parser pinned in `pluto-lib`; the native demo uses its target host's parser. Combined Tinkers integration and Windows remain unverified.

The older [Creative paging adapter](creative-overflow.md) and [item behavior demo](item-behaviors.md) were built and run on Linux against Horizon [`b6c936f82b08798057161bdfa577e3aac53fb91e`](https://github.com/horizon-rust/horizon/commit/b6c936f82b08798057161bdfa577e3aac53fb91e) with the same Mixins revision. The first registration demo used this Horizon revision without Mixins. Their checks do not establish compatibility with the newer host.

## Development target

The native Creative and component-transport bundles target Horizon [`5197790e5846a82cfc84d9703e37c4c1b75be09c`](https://github.com/horizon-rust/horizon/commit/5197790e5846a82cfc84d9703e37c4c1b75be09c) and Mixins [`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`](https://github.com/horizon-rust/horizon-mixins/commit/ddd9700b21997c95d95ac6ee5ecafe3830cf622a). The Mixins plan and native Creative bundle check passed. On Linux, the independent transport demo was built and run with this pair; its [LAN and tooltip checks](component-transport.md) do not establish Tinkers integration.

The independent native Creative and component-transport demos were built and run on Linux with Horizon `3a264943c7a3a3ed02e6ee608443f558d98f208f` and the same Mixins revision. See [component transport](component-transport.md) for the observed host display limitation and the scope of LAN checks. Neither result establishes Tinkers integration.

Horizon `aa60eb76ea82bd66c29fea352e3ab5771e62ab26` is a newer, untested candidate. Its Creative slot hook changes the guest-to-host path, so the LAN checks above do not establish compatibility with it.
