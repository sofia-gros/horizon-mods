# Developing mods

Each mod lives in a directory at the repository root. The repository currently contains one mod, `tconstruct/`.

```text
horizon-mods/
├── .github/              Shared CI configuration
├── tools/                Shared CI tools
├── pluto-lib/            Shared mod library
└── tconstruct/
    ├── README.md         Mod overview
    ├── docs/             Guides and screenshots
    ├── mixins/           Horizon Mixins and Rust code
    ├── assets/           Mod assets
    ├── content/          Reference data
    ├── tests/            Mod tests
    └── tools/            Build and resource preparation tools
```

Start with the [Tinkers' Construct development guide](../tconstruct/docs/development.md) for its requirements, build commands, and tests. A new mod should keep its own implementation, resources, tests, and documentation in its own top-level directory. Shared repository configuration belongs at the root.

See [pluto lib](../pluto-lib/README.md) for the Creative tab registration API.

## Validate a change

1. `python3 tools/lint_forbidden_api.py` and `python3 -m unittest discover -s tests` (also run by CI on every pull request).
2. CI (`mixins.yml`) additionally stages the vertical slice against the pinned Horizon/Mixins and runs `cargo check`. It never launches the game.
3. For gameplay-affecting changes, run the local gate on a machine with the game resources:
   `tools/smoke-local/build.py` then `tools/smoke-local/smoke.py` (see [AGENTS.md](../AGENTS.md)). Paste its `SMOKE PASS` output in the PR.

Working rules (PlutoLib only, one PR per verification unit, at most 5 open PRs) are in [AGENTS.md](../AGENTS.md).

## Current status (2026-11-07)

Verified on a Windows PC against Horizon `ebcfa226` and Mixins `5a64af93` (upstream HEAD that day):

- The vertical slice stages (Mixins plan and the PlutoLib `protection` guard pass), builds, launches into a test world with the mod loaded, and `/tconstruct parts` yields the three flint-pickaxe parts (`SMOKE PASS`).

Not verified / not working:

- Only 2 of 290 TConstruct injections are enabled. The rest are parked until re-absorbed; the Part Builder as it exists hooks Horizon UI internals directly and will be rebuilt on PlutoLib.
- 10 of PlutoLib's 11 composition guards fail against this Horizon (advancement, cloud, current_book, equipment, matching, payload, placement, station, terrain, wire_snapshot). The hooks they protect are excluded from the slice and need per-feature re-review.
- CI changes (pull_request trigger, slice check, weekly upstream drift) have not run on GitHub yet.
- The tests under `tconstruct/tests` were not green when last run (32 failures, 5 errors of 458); causes not yet triaged.
