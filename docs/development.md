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

- 287 of 290 TConstruct injections (all but three the pinned Mixins CLI rejects) plus the PlutoLib features whose guards pass stage (Mixins plan, PlutoLib `protection` guard), pass `cargo check`, build, and launch into a test world with the mod loaded. `tools/smoke-local/smoke.py` prints SMOKE PASS: survival crafting through data recipes yields a flint pickaxe, and the real Part Builder block opens a container screen with slots. See [the Part Builder gap](migration/part-builder-plutolib-gap.md).

Not verified / not working:

- The three rejected injections are parked: mining speed (`source-host-mining-context`, `source-client-mining-context`) and Tinkers Chest pick color (`tinkers-chest-pick-color`). Their targets changed upstream.
- Most enabled hooks are the legacy direct-host hooks, tolerated by the lint baseline. They work at plan and compile level but are not PlutoLib based, and nothing beyond the two smoke assertions was exercised in game (no Part Builder crafting, no mining, no save/reload, no LAN).
- The staging copy of Horizon is patched in one formatting spot (`0. ..=1.` becomes `0.0..=1.`, same meaning) because Horizon Mixins re-prints edited files from a token stream and produces invalid `0...=1.` (see `staging-normalizations.txt` in the stage output). Five helper sources were adapted to Horizon API changes (`ModelQuad`, `PotionCloud`, `Breath`, `open_container`).
- 10 of PlutoLib's 11 composition guards fail against this Horizon (advancement, cloud, current_book, equipment, matching, payload, placement, station, terrain, wire_snapshot). The hooks they protect are excluded from the slice and need per-feature re-review.
- CI changes (pull_request trigger, slice check, weekly upstream drift) have not run on GitHub yet.
- The tests under `tconstruct/tests` were not green when last run (32 failures, 5 errors of 458); causes not yet triaged.
