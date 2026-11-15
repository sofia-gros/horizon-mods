# Horizon Mods

[日本語](README.ja.md)

Two products: **PlutoLib**, an independent general-purpose Fabric-API-style library for official Horizon, and **Tinkers' Construct**, a consumer of its public API only.

## Current state

The clean port is being established. `tconstruct/` now contains an **empty native entry point with zero injections**; no original gameplay has been imported or verified in this new product. This is not a playable release. Existing PlutoLib remains a candidate requiring review and native validation.

The previous Tinkers implementation, including uncommitted source and resources, is preserved under `legacy/2026-11-14/tconstruct/`. It is not built or packaged by the clean-product command. Old screenshots and inventories describe the old product, not current functionality. Existing CI remains enabled and needs migration reconciliation; no full-CI success is claimed.

## Work

```bash
python tools/port.py next
```

```bash
python tools/port.py check
```

- [Rules](AGENTS.md) · [Plan](docs/roadmap.md) · [Architecture](docs/architecture.md) · [Commands](docs/development.md)
- [PlutoLib API reference](pluto-lib/docs/api/index.md): consult source/support status; an API declaration is not proof of a working adapter.
- [New Tinkers](tconstruct/README.md)
- [Verified development launch](docs/development.md#verified-development-launch): explicit source-bound binary only, fresh isolated home, no legacy fallback; not a playable Tinkers release.

Preserve original licence notices when recovering assets or code. No release is authorized by the reconstruction.
