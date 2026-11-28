<div align="center">

<img src="docs/branding/horizon-mods-icon.png" alt="Horizon Mods icon" width="120">

# Horizon Mods

<img src="docs/branding/horizon-mods-banner.png" alt="Horizon Mods banner" width="900">

**Mods and a Fabric-API-style library for [Horizon](https://github.com/horizon-rust/horizon) and [Sunrise](https://github.com/sunrise-org/sunrise).**

English · [日本語](README.ja.md)

</div>

## What is here

Four projects share this repository. Everything is built on **PlutoLib**, which owns all injection into the game; the other three are ordinary mods that call PlutoLib's public API and never inject anything themselves.

| | |
|---|---|
| **[PlutoLib](pluto-lib/)** ([API pages](pluto-lib/docs/api/index.md)) | The counterpart of the Fabric API: an independent library mod that other mods call, with Fabric-like names and behaviour. It is always released on its own, and its API has the same shape on every platform it supports. |
| **[Tinkers' Construct](tconstruct/)** | A clean port of the original Java Tinkers' Construct, written against PlutoLib's public API only: build tools from parts, smelt and cast metals, modify your equipment. |
| **[Classic Pipes](pipes/)** | A clean-implementation pipe mod in the BuildCraft style: item pipes that connect furnaces, tanks and chests, with the items in transit visible inside the pipes. |
| **[Stellarsky](stellarsky/)** | A realistic-sky mod: a real star catalogue with twinkling stars, the Milky Way with its dark lanes, zodiacal light and airglow, and a black hole at Sagittarius A*. |

## Status

### Sunrise — released

**PlutoLib v1.0.0**, **Classic Pipes v1.0.0** and **Stellarsky v1.0.0** are published in the Sunrise mod registry; install them from the launcher's mod catalogue. The same releases are on GitHub: [PlutoLib](https://github.com/sofia-gros/horizon-mods/releases/tag/pluto_lib/v1.0.0) · [Classic Pipes](https://github.com/sofia-gros/horizon-mods/releases/tag/pipes/v1.0.0) · [Stellarsky](https://github.com/sofia-gros/horizon-mods/releases/tag/stellarsky/v1.0.0).

> **Classic Pipes and Stellarsky require PlutoLib v1.0.0 or later.** They were built for v1.0.0; if a newer PlutoLib version does not work, use 1.0.0. PlutoLib is a separate download — it is not bundled with the mods.

On Sunrise, parts of PlutoLib's API are not connected yet, so some features are present but do nothing there for now. The [API pages](pluto-lib/docs/api/index.md) mark every item with its state on each platform. Two examples: Stellarsky currently shows its dynamic sky — the stars, their twinkling and the black hole — while its static layers (the Milky Way sphere, airglow and zodiacal light) are not connected yet; and the four pipes work and can be crafted, but cannot be taken from a creative tab yet.

There is no Tinkers' Construct release for Sunrise. Tinkers returns on the Horizon line below.

### Horizon (new mod API) — in development

Horizon has moved to a new mod API, and PlutoLib is being built for it. Nothing on this line is released yet. Tinkers' Construct follows on this line, on top of the new PlutoLib.

### The old Horizon line — archived

The previous generation, built on the `.hnmod` format, is closed. Its releases remain available under their tags for the launcher generation they were built for: **[v0.0.1](https://github.com/sofia-gros/horizon-mods/releases/tag/tconstruct-v0.0.1)** (2026-11-23) and the **v1.0.0-beta** prereleases (2026-11-25). There will be no formal v1.0.0 on this line: current Horizon launchers no longer load the `.hnmod` format.

Verified in game at the time (Windows, the v0.0.1 build):

- Crafting a pickaxe from its parts on the inventory grid, and the Part Builder screen.
- The melter and the smeltery: ores melt into fluids and cast back into parts.
- Tools dig, wear, hit and describe themselves by their materials and modifiers.

<p align="center">
  <img src="docs/images/part-builder.png" alt="The Part Builder screen in game" width="48%">
  <img src="docs/images/crafted-flint-pickaxe.png" alt="A crafted flint pickaxe in the inventory" width="48%">
</p>

## Where to go next

- **Developers:** the [PlutoLib API pages](pluto-lib/docs/api/index.md) (every public item, with its per-platform state), [docs/development.md](docs/development.md) (build, local checks, CI) and [docs/architecture.md](docs/architecture.md).
- **Plans:** [docs/roadmap.md](docs/roadmap.md).
- **Releases:** [docs/releasing.md](docs/releasing.md). Each component is released separately, under its own tag.
- **Working rules for contributors and agents:** [AGENTS.md](AGENTS.md).

## Licence

See the licence files in each directory.
