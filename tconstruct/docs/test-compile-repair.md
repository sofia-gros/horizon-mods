# Client library test compilation

On Horizon `0c7b5b61a5e746d621afffba919b12aeccf9c0fa` and
Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`, a combined
TConstruct/PlutoLib `horizon-client --lib` test build failed with 19 errors.
The test build omitted the Game-dependent Airborne helper because it was gated
by `cfg(not(test))`, while the generated bow and crossbow hooks still called
it. The helper now lives in a separate Game module and is compiled in both
normal and test builds. The pure projectile-stats module remains usable by
standalone tests. No projectile behavior was intentionally changed.

The remaining errors were test-only ZIP readers treating `ZipFile::name()` as
a string instead of the host's `Result`. They now require a valid entry name
and fail explicitly on malformed archive names; no assertions were removed.

On the combined candidate based on main `480b2512a02bcf1160bed18312f5fddeee67389c`,
official Mixins apply succeeded. Locked, offline `cargo test -p horizon-client
--lib --no-default-features --profile fast --no-run` succeeded. The affected
projectile, render-info, stat-type and transaction modules ran 75 tests with
no failures after staging the pinned `content/reference` and
`docs/upstream-recipes.json.gz` fixtures. An initial transaction run without
the catalog failed three tests; with the pinned catalog it passed 58/58.

The full generated library test binary ran 1,087 tests: 1,081 passed, one
failed and five were ignored by the existing host test suite. The failure,
`game::native_hooks::tests::hook_points_stay_hookable`, checks the generated
`native_hooks.rs` for macros; existing TConstruct helpers injected into that
file contain `matches!` and `format!`. This is a separate unresolved
integration-test failure, not a passing full suite. No game or LAN test was
performed.

## Native hook test follow-up

The seven injected helpers whose macros appeared in the generated hook-point
file now delegate to a separate Game module. Their function bodies were
compared with the previous versions and are unchanged apart from visibility;
the hook positions, call order, arguments, and existing hook test remain in
place. This includes the comparator, geode growth, sapling, slime leaf and
grass behavior, and Overwield harvest handling. A small adapter test also
checks the seven wrappers for the expected delegation and absence of macros.

On the same pinned host and Mixins revisions, official combined apply
succeeded (266 injections, 221 modules, 98 states). With the pinned source
fixtures available in the generated stage, the complete locked/offline
`horizon-client --lib --no-default-features --profile fast` test run passed:
1,082 passed, zero failed, five ignored by the existing suite. This establishes
neither in-game behavior nor LAN behavior for the moved helpers.
