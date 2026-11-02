# Compatibility

## Verified combination

PlutoLib is unreleased: its Cargo manifest currently declares `0.1.0`. The Creative paging demo at development commit [`42e31b0b7ab35f2b8afc154d4c68888032f5d795`](https://github.com/sofia-gros/horizon-mods/commit/42e31b0b7ab35f2b8afc154d4c68888032f5d795) was built and run on Linux with:

- Horizon [`b6c936f82b08798057161bdfa577e3aac53fb91e`](https://github.com/horizon-rust/horizon/commit/b6c936f82b08798057161bdfa577e3aac53fb91e), content API 0.2.
- Horizon Mixins [`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`](https://github.com/horizon-rust/horizon-mixins/commit/ddd9700b21997c95d95ac6ee5ecafe3830cf622a).

The initial registration demo was also run on this Horizon revision on Linux, but did not use Mixins. The paging demo uses the Mixins adapter. The Rust preparation API also depends on `horizon-items` from that same Horizon commit to parse and compare search stacks. See [Creative paging](creative-overflow.md) for the checks and limitations. Other platforms and revisions have not been verified.

## Development target

The combination above is the current development target, not a released compatibility guarantee. Horizon [`e12185820a6bc85dd5ed04adc7b8b7a4bdc16102`](https://github.com/horizon-rust/horizon/commit/e12185820a6bc85dd5ed04adc7b8b7a4bdc16102) is a candidate for future work and has not been tested with PlutoLib. PlutoLib does not promise compatibility with every Horizon or Mixins revision; a new combination may require rebuilding and source changes.

The experimental item behavior and wand demo are not yet built or verified in game or over LAN. They target the same pinned combination above.
