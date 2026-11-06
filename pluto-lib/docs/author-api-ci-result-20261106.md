# Pure Mod-author API composition: verified result

Exact source head: 2ee73a700647a5a73142cd20fea065b44c8de598

[Run 42176619324](https://github.com/sofia-gros/horizon-mods/actions/runs/42176619324), job 127756443147, completed successfully on 2026-11-06. This report is a docs-only child and does not retrigger CI.

## Executed evidence

- Complete actual inventory API: 143 tests, including the recipe builder and Guide/event additions.
- Complete actual public API: 58 tests, including all six fluid Declaration tests.
- Guide external example: one compiled and executed test.
- Fluid external example: one compiled and executed test, including Air, Unloaded/Unknown and registered source observations.
- Three external binaries were actually type-checked: storage, recipe presentation and matching registration. The recipe examples have empty main functions, so this proves typing rather than runtime registration.
- The unchanged storage example was separately compiled and executed with its six assertions and required successful plan. No live commit or event is performed.
- Thirteen runner regressions passed. All 203 Rust test cases passed with zero failures, ignored or filtered cases.
- Actual rustc invocations, not Fresh-only acceptance, are required for each tested target. Sources, exact file sets, seed/derived locks and generated inputs were checked. The checkout remained clean including untracked files, and every owned process group terminated.
- Minimum free space on monitored filesystems: 91284918272 bytes, above the 10871635968-byte floor. No local compiler was started for this run.

## Compatibility boundary

This is a mixed, explicitly pinned composition of the existing two generic crates. The author-facing builders and event entry delegate to their existing implementations and registry owners. Their old API paths remain. Source inverse checks and old/new registration or value tests cover the scoped contracts; this is not a general guarantee for every API or future Horizon version.

The native host adapters from the source branches were not composed into this job. It does not prove Game/Font/HUD/GPU behavior, LAN lifecycle, material-provider activation, the native Book adapter, item display localization, or completed TConstruct migration. New inventory re-export and terrain facade cutoffs remain separate. Nothing was merged to main and no PR state changed.

## Reproduction

The immutable workflow, runner, input file and exact source cutoffs are under this commit's parent, in pluto-lib/experiments/author-api-ci. The JSON companion preserves all 203 test names through the receipt, phase terminals, source/copy/output seals and actual compiler command witnesses. No cache or build artifact was uploaded and no permissions or billing settings were changed.
