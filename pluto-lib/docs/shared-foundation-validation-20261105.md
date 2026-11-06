# Grouped PlutoLib foundation validation (latest run 2026-11-06)

Complete ordinary-library cutoff: `b19b35a10b96e398ca8dabcb46b200d88950edaf`. Official Horizon:
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`. Mixins: `e8cd0aa38cffef49cb2e088ef716baa107148c02`.

**2 crates, 66 modules, 34 injections, 3 owned states.** This batch adds a bounded,
version-pinned terrain-cell adapter, explicit product/remainder pair staging,
native Font layout facts, and a fourth plain-cell consumer helper delegation.
The full TConstruct consumer bundle and isolated experiments are not installed.

## Actual complete-package results

- Official plan passed in 21.00 seconds.
- All twelve required source/composition guard groups passed.
- Official apply passed in 21.39 seconds.
- Host lib/bin type-check passed in 1m 04s, offline/locked, debug=0,
  incremental=0 and two jobs.
- 210 Python checks passed, invoking both complete actual API suites:
  43 gameplay/fluid API and 122 inventory/guide API tests. Counts overlap focused tests.
- Minimum free space: 10,897,825,792 bytes; disk guard did not stop.
- All 5,046 generated files are sealed. 5,040 byte-identical previous
  files received recorded timestamps after archive verification, without content edits.

Archive SHA256: `80adf5750e59abbaf3060a57025610230eb3cab73452761b0763699d03bbd6a2`.
Exact artifacts are in `grouped-v14`. Previous cutoffs remain in the JSON and
version history; this run does not retroactively validate old PR heads.

## Verified stage preservation

After the proof, the generated working stage was removed only after the existing
full v13 archive plus a 743,068-byte v14 delta were persisted and synchronized,
then completely restored into a separate temporary filesystem. All 5,046 hashes
and 5,792 file/directory attribute entries matched. The retained archives and
restoration manifest are identified in the JSON receipt. Original source, logs,
unpublished work and dependency caches were not removed.

This recovery-only operation retained at least 10 GiB + 32 MiB free under explicit
authorization; the usual 10 GiB + 128 MiB native-build guard was not lowered.
File types, permission modes, uid/gid, exact mtime and extended attributes were
verified. Access time is excluded; symlinks and hardlinks are not accepted.

## Supporting evidence

- Terrain: 17 focused standard-library tests, 19 source/package checks and actual
  plan/guards preceded this grouped apply/type proof. Actual CPU equivalence,
  timing and GPU tests remain unexecuted; no consumer provider is registered.
- Products: 15 actual native-item/codec fixture tests and 80 feature API tests.
  The initial 14/15 fixture assertion failure was corrected in test-only source
  by using an official canonical ominous item; production bytes did not change.
- Font layout: eight actual Font tests plus three converter tests using an original
  synthetic provider. Raw wrapping and logical legacy-resolved hit facts do not
  prove rendered bidi hit testing, UI ownership or live input behavior.
- Plain cells: 33 native fixture tests, 6,514 valid cases and 9,572 comparisons,
  including the fourth pickup_slot delegation. Caller admission and limits remain
  local. Zero-count/zero-limit rejection is a deliberate correction, not legacy parity.

## Remaining limits and publication

This is a **library package type-check**, not a linked game/consumer release.
No full TConstruct bundle, live gameplay, save/reload, LAN or ordinary launcher ran.
No native test experiment is silently included in the ordinary package.

Product pairs are explicitly staged and caller-released; no automatic crafting
hook or coherent native output-consumption transaction is activated. Plain-cell
operations have no menu-hook, notification, save/sync or actor/slot authority.

No native EffectInstance curative metadata or same-pass effectful contact hook is
installed. Cloud insertion is type-checked but unactivated; four private helper
and two factory tests remain unrun. Actual minimal-loader contact tests do not
prove production assets or Game-wrapper gameplay.

Guide request correlation is caller-owned in-process state. Native persistent
screen ownership, admitted input and delayed reply effects remain unproved.
The existing consumer response hook conflicts with the mandatory payload guard;
its LAN behavior has not been removed to make validation pass. The separate
navigation repair does not resolve that response boundary.

The exact source tree is preserved at [the v14 checkpoint](https://github.com/sofia-gros/horizon-mods/commit/43411ddb6e5be857db96740150319a7c2e83eed1).
[Draft PR #944](https://github.com/sofia-gros/horizon-mods/pull/944) remains the
reviewed v13 snapshot, not a main-ready full consumer. Original local development
history is also retained in a [verified recovery bundle](https://github.com/sofia-gros/horizon-mods/tree/checkpoint/plutolib-original-history-20261106/pluto-lib/history/20261106),
which is an archive rather than the ordinary PR graph. Main has not been merged.

Use [guarded staging](native-installation.md#mandatory-development-preflight).
The ordinary launcher does not run mandatory guards. See the
[migration ledger](shared-api-migration-status.md) for remaining responsibilities.
