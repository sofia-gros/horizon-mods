# Install the native library

PlutoLib's development package is one source-only `.hnmod` shared by native mods. It contains the Creative and item APIs, a two-slot [inventory move planner](inventory.md), component transport, Tool rules, and durability tooltip and bar adapters. It does not contain a consuming mod, demo hooks, mod assets or the unfinished world-placement prototype.

## Build the bundle

From `pluto-lib`, with Python 3.11 or later:

```sh
mkdir -p .work
python3 tools/package_native.py .work/plutolib.hnmod
```

The native mod ID is `pluto_native_creative`; its Mixins source is under `mixins/pluto_native_creative/`, and its Cargo crate is `pluto-api` (imported as `pluto_api`). The output must not already exist. Python builds the archive; it is not needed by the native API. The component source directories remain available for development, but the generated archive combines them as one official Mixins bundle with one `pluto_api` crate and one shared inventory-planning crate.

## Use it with a mod

Install and enable PlutoLib once alongside each consuming mod's separate `.hnmod`. A consumer may also need its own content package. All participating bundles must target the same Horizon and Mixins revisions; the official launcher has no general dependency field, so install the matching library manually.

Use `pluto_api::creative` and `pluto_api::item` as described in [Creative preparation](native-creative.md) and [short item use](item-use-b2.md). Tool functions currently use host-side paths shown in the [combined demo](../experiments/unified-target/README.md). Do not enable the individual six component bundles alongside this archive: doing so would register the same hooks or crate twice. Do not combine the older Creative paging or item-network adapters with it.

The single-file package targets Horizon `7084e67744cbb4fe25961f24126e15b6b36f0523` and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. The earlier `cabe8b5de7d0b45541c923c4a6b3c5cb3e3686c7` package passed official validation and extraction, and its Linux demo ran with a LAN guest save and reconnect. This target awaits integration testing. A joint TConstruct Linux build and representative game checks passed on the earlier `fd6cfda6bdef59a53aee830009f78c14832b0308` target; see [compatibility](compatibility.md) for scope. Full launcher installation and Windows remain unverified. TConstruct adopted selected APIs on the earlier pair; see [compatibility](compatibility.md).

The development bundle can include Mixins per-instance `[[state]]` declarations
and their `[[state.constructor]]` entries. The landing state is transient and
belongs to each player motion instance; it does not prescribe an equipment rule
or a landing effect. Its combined-package planning, constructor coverage, type
checking, and gameplay behavior still require validation.


The native package checks cover both the landing-state manifest and a fixture
without state. They check unique declaration IDs, constructor preservation,
module and helper archive references, and deterministic package output. The
updated checks must be run by the validation owner on the integrated candidate.

The landing-state follow-up targets `horizon_client_lib::motion::PlayerMotion`,
the crate that owns `motion.rs`; the earlier `horizon_client` target failed the
official apply. The corrected target still requires official plan/apply and type
verification.
