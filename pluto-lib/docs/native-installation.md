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

The single-file package targets Horizon `cabe8b5de7d0b45541c923c4a6b3c5cb3e3686c7` and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. The package passed official validation, the launcher extraction harness and Mixins transformation. A Linux client built from the extracted package ran the combined Creative, Tool and wand demo, including a LAN guest save and reconnect. Full launcher installation, Windows and Tinkers integration remain unverified. See [compatibility](compatibility.md).
