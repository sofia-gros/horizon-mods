# Installed language module proof

Status: source preparation only. No local Cargo, Mixins plan/application, native
compilation or host test execution was performed for this checkpoint.

## Why this is separate from the successful 22-case proof

The first native i18n CI (0e4d68ea, run 42181415662) passed 22 cases on actual
Language/Text/ItemStack dependencies with exact Names/styled_name fragments.
Its original author example was type-checked under an explicit fixture alias,
not as part of the installed horizon-client library.

This job closes a different boundary using the unchanged source from
060d622da80f5c5e95a2c1e536f68011290852f0 (runtime ea7095d8): package the complete
Pluto bundle, plan all its declarations with official Mixins, run all twelve
complete-plan guards, apply to pristine official Horizon, check actual client
lib and binary targets, then compile and execute the real installed client-lib
Language eight and Key six tests. It does not use a replacement host or alias
for those fourteen tests.

The exact archive is f35294757323aa2889122d02bf40fa168566880f1dd5a446030307482cc230b7,
244,620 bytes, with two crates, 68 support modules, 34 injections and three states.
The language addition is two public support modules in client_lib.rs; it adds
no language hooks, locale owner, cache, event, screen or network registration.
The plan must contain each exact language payload once, at the actual public
client-lib owner. The bundle hash binds all other packaged source bytes too.

## Planned gates

1. Sixteen runner regressions and nineteen existing language/TOML/source checks.
2. Fetch only official Horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0 and official
   Mixins e8cd0aa38cffef49cb2e088ef716baa107148c02, and verify the official lock.
3. Deterministic package hash and complete declaration counts.
4. Official complete plan, both exact public module payloads, twelve native-plan
   guard groups, and official application.
5. Actual rustc checks for pluto_api, renderer, client-lib, client lib and binary;
   Fresh-only evidence cannot satisfy this gate.
6. Actual horizon-client-lib --test compilation from its official
   ../horizon-client/src/client_lib.rs source, then separate exact filters for
   eight internal and six Key tests, no omitted, failed, duplicate or ignored cases.
   Unrelated host tests are filtered and not claimed as executed.
7. Full Mod/official/export/package/stage source checks, clean tracked/untracked/ignored
   Mod and both official checkouts (complete files, modes and symlink targets) and generated OUT_DIR integrity. Recompilation in execution logs
   fails the result; this is detection, not a compiler pre-launch blocker.

The public GitHub-hosted job is dedicated to its exact branch. Contents-read only,
no persisted checkout credentials, no caches, artifacts or billing changes.
One compiler job, debug/incremental disabled, a 10 GiB +128 MiB disk floor on all
output filesystems and a 2 GiB setup reserve are enforced. Owned process groups
are killed/reaped and their absence checked with bounded waits. Initial/partial
failures and final verification errors cannot produce a passing receipt.

## Deliberate exclusions

This is not a live Game/window test. It does not prove complete mod-content
admission, resource-pack/content overlay priority, same-locale resource reload,
retained Guide page reconstruction, guest locale synchronization or delayed LAN
reply validity. It does not translate title strings still used as protocol or
screen identity, and does not validate arbitrary unknown NBT/Wire preservation.
It does not combine the separate Java/menu, newest authoring facade, or rendering
facade checkpoints into a new common runtime. Main and PR states are unchanged.

Preparation evidence: the first sparse local source check lacked the unchanged
language_text.rs example input (18 passed, one missing-file error). That exact
tracked input was materialized; nineteen source checks and sixteen runner
checks then passed. No Rust command ran during either local check.
