# Install the native Creative API

The native Creative and item APIs share a source-only Horizon bundle. Install it once alongside each mod that uses it. It adds the `pluto-api` crate to the native client build; it contains no demo hook and makes no visible changes by itself.

## Build the bundle

From `pluto-lib`, with Python 3.11 or later:

```sh
mkdir -p .work
python3 tools/package_native.py .work/pluto-native-creative.hnmod
```

Choose a new output name each time: the command does not overwrite files. The bundle contains its native manifest and the sources in `mixins/pluto_native_creative`; it does not contain demo code or mod assets. Python is only needed to create the archive, not by the native API.

## Use it with a mod

Install and enable the PlutoLib `.hnmod` and the consuming mod's own `.hnmod` as two separate native bundles in a compatible Horizon launcher. A content mod may also require its own `.hmod`; follow that mod's installation instructions. The launcher includes both native bundles in the same native build. Include PlutoLib only once, and use the versions required by the consuming mod. Native bundles have no general dependency field, so the user must install and enable the matching library manually.

A consuming native mod can call `pluto_api::creative::register_tabs` after completing its own stacks. See [native Creative preparation](native-creative.md) for the API and initialization order. Do not install the old PlutoLib paging adapter with this API.

This development package targets the Horizon candidate `cabe8b5de7d0b45541c923c4a6b3c5cb3e3686c7` and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. The earlier `5197790` package passed the official bundle check and its independent transport demo ran on Linux; the combined native demos ran on Linux for the new target before the API crate was consolidated; the consolidated crate has not yet been verified in game; the package itself has not been installed through the launcher. On the earlier `3a264943` host, the demo ran on Linux and a harness verified the launcher’s bundle extraction function. Installation and launch through the full launcher, Tinkers integration, and Windows remain unverified. See [compatibility](compatibility.md) for the full scope.
