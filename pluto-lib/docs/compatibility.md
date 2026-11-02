# Compatibility

PlutoLib is unreleased; its Cargo manifest declares `0.1.0`. Each native bundle targets a specific Horizon revision. Other revisions and platforms are not covered by these checks.

## Verified combinations

On Linux, the [native Creative demo](native-creative.md) was built and run at PlutoLib development commit [`0b0b40e89c356e2bcdbabed5ffa7d5b7a68302dc`](https://github.com/sofia-gros/horizon-mods/commit/0b0b40e89c356e2bcdbabed5ffa7d5b7a68302dc) with:

- Horizon [`5afa1de5f750b4b89ca23347b1d3aba296c278b0`](https://github.com/horizon-rust/horizon/commit/5afa1de5f750b4b89ca23347b1d3aba296c278b0)
- Horizon Mixins [`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`](https://github.com/horizon-rust/horizon-mixins/commit/ddd9700b21997c95d95ac6ee5ecafe3830cf622a)

The standalone preparation wrapper uses the older `horizon-items` parser pinned in `pluto-lib`; the native demo uses its target host's parser. Combined Tinkers integration and Windows remain unverified.

The older [Creative paging adapter](creative-overflow.md) and [item behavior demo](item-behaviors.md) were built and run on Linux against Horizon [`b6c936f82b08798057161bdfa577e3aac53fb91e`](https://github.com/horizon-rust/horizon/commit/b6c936f82b08798057161bdfa577e3aac53fb91e) with the same Mixins revision. The first registration demo used this Horizon revision without Mixins. Their checks do not establish compatibility with the newer host.

## Development target

The [combined native demos](../experiments/unified-target/README.md) were built and run on Linux with Horizon [`cabe8b5de7d0b45541c923c4a6b3c5cb3e3686c7`](https://github.com/horizon-rust/horizon/commit/cabe8b5de7d0b45541c923c4a6b3c5cb3e3686c7) and Mixins [`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`](https://github.com/horizon-rust/horizon-mixins/commit/ddd9700b21997c95d95ac6ee5ecafe3830cf622a). The demo-only native item limit of 200 was checked in Creative at 199, 200 and 201; Tinkers integration, Windows, and the full launcher path remain unverified.

The independent [short item-use demo](item-use-b2.md) was built and run on Linux with Horizon [`b2e71aca04c7f62f24eefd7dd0324bae9c04954f`](https://github.com/horizon-rust/horizon/commit/b2e71aca04c7f62f24eefd7dd0324bae9c04954f) and Mixins [`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`](https://github.com/horizon-rust/horizon-mixins/commit/ddd9700b21997c95d95ac6ee5ecafe3830cf622a). Its local and LAN checks cover the short-use path only; Tinkers and Windows remain unverified.

The independent [stack durability bar demo](component-bar.md) was built and run on Linux with Horizon [`b2e71aca04c7f62f24eefd7dd0324bae9c04954f`](https://github.com/horizon-rust/horizon/commit/b2e71aca04c7f62f24eefd7dd0324bae9c04954f) and Mixins [`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`](https://github.com/horizon-rust/horizon-mixins/commit/ddd9700b21997c95d95ac6ee5ecafe3830cf622a). This check does not establish compatibility for other PlutoLib bundles or Tinkers.

The native Creative and component-transport demos were checked against Horizon [`5197790e5846a82cfc84d9703e37c4c1b75be09c`](https://github.com/horizon-rust/horizon/commit/5197790e5846a82cfc84d9703e37c4c1b75be09c) and Mixins [`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`](https://github.com/horizon-rust/horizon-mixins/commit/ddd9700b21997c95d95ac6ee5ecafe3830cf622a). The Mixins plan and native Creative bundle check passed. On Linux, the independent transport demo was built and run with this pair; its [LAN and tooltip checks](component-transport.md) do not establish Tinkers integration.

The independent native Creative and component-transport demos were built and run on Linux with Horizon `3a264943c7a3a3ed02e6ee608443f558d98f208f` and the same Mixins revision. See [component transport](component-transport.md) for the observed host display limitation and the scope of LAN checks. Neither result establishes Tinkers integration.

The [Tool rules demo](tool-rules.md) was built and run on Linux with Horizon [`aa60eb76ea82bd66c29fea352e3ab5771e62ab26`](https://github.com/horizon-rust/horizon/commit/aa60eb76ea82bd66c29fea352e3ab5771e62ab26) and Mixins [`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`](https://github.com/horizon-rust/horizon-mixins/commit/ddd9700b21997c95d95ac6ee5ecafe3830cf622a). Its representative LAN check covers an allowed Creative slot update and reconnection. The separate Creative cancellation experiment has a documented non-atomic result; it is not part of the Tool rules bundle.

The consolidated Creative and short-use item API was built and exercised on Linux at development commit `0f36b1fe779565fff37a445a485d920baa1bcf2f` with Horizon `cabe8b5de7d0b45541c923c4a6b3c5cb3e3686c7` and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. See the [combined demo](../experiments/unified-target/README.md) for the verified operations. Full launcher installation, Windows, and Tinkers remain unverified.

Horizon `e6406b7a04521323dace51d860747dc93dc67ebc` is a newer upstream candidate and has not been verified with this combined API.

The [single-file native library](native-installation.md) was built and exercised on Linux from development commit `2d70296717a49dee97fbb03e45e3bcef40ec14c4` with Horizon `cabe8b5de7d0b45541c923c4a6b3c5cb3e3686c7` and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. The official bundle validator and launcher extraction harness passed; the complete launcher application, Windows and Tinkers integration remain unverified.
