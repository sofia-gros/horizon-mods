# Releasing

Each product is released on its own from this repository. Nothing is released by hand: a tag starts a workflow that checks, builds, validates and publishes.

## Tags

`<directory>-v<MAJOR.MINOR.PATCH>`, optionally `-rc.N` for a pre-release:

| Tag | Workflow | Publishes |
|---|---|---|
| `pluto-lib-v0.1.0` | `.github/workflows/release-pluto-lib.yml` | `pluto-lib.hnmod`, `SHA256SUMS` |
| `tconstruct-v0.5.0` | `.github/workflows/release-tconstruct.yml` | `tconstruct.hmod`, `tconstruct.hnmod`, the legacy `plutolib.hnmod`, `SHA256SUMS` |

The tag's version must equal the product's `[mod] version` in its `horizon-mixin.toml`; `tools/validate_release.py` fails otherwise. It also checks that the
package manifests carry the Horizon and Mixins pins from `tconstruct/compatibility.json`.

Each file states what it needs: `pluto-lib.hnmod` declares the Horizon it was built for in `horizon-native.json`; a mod that uses PlutoLib declares
`pluto_lib = "^X.Y"` under `[dependencies]` in its `horizon-mixin.toml`. A Horizon update that PlutoLib can absorb is a PlutoLib minor or patch release and needs no change in its dependants.

## Release criteria

A release is made only when all of this holds; it is never made to have something to show.

1. CI is green on the release commit and `tools/smoke-local` passes against the pinned Horizon.
2. The release comes from a written-down triple: product version, Horizon commit, Mixins commit.
3. Save/reload and multiplayer paths of the shipped feature groups were exercised, not assumed.
4. The README and the product README say what is and is not supported.

## Steps

1. Merge everything for the release; check that `[mod] version` is the new version.
2. Run the local gate ([development.md](development.md)).
3. `git tag pluto-lib-v0.1.0 && git push origin pluto-lib-v0.1.0` (or `tconstruct-v…`).
4. Watch the workflow; it fails closed (validation first, publication last). A bad release is fixed with a new patch version, not by moving a tag.

Local dry run of the PlutoLib package: `python tools/package_bundle.py pluto-lib/mod dist/pluto-lib.hnmod` then `cargo horizon-mod check dist/pluto-lib.hnmod` (a tool built from Horizon's `cargo-horizon-mod` crate).

No release exists yet.
