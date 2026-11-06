# Native language and item-name contracts

Status: prepared for a dedicated public-runner CI. No local Cargo, metadata,
native compilation or game execution was performed during preparation.

## Exact source and ownership

The parent is df0645ec2bdd1063364eb82d85c45ef45d61c876: the composed
Language/Key source from ea7095d8 plus the frozen item fixture. Runtime source is
unchanged by this CI addition. Twelve input files are pinned in inputs.json.
The exact official Horizon source is 624c4f6265c02296fbe8a80f029dd0d987fb94c0.
No user engine fork is read or cloned.

The internal adapter delegates to the one official lang owner. The public Key
retains borrowed key/fallback settings and resolves through that adapter. It
adds no selected locale, registry, cache, event or automatic screen update.
JSON lives in the consumer's assets/<namespace>/lang/<locale>.json. The optional
TOML converter runs at preparation time; it emits flat string JSON and refuses
existing output. It does not add a runtime TOML loader or change pack priority.

## Planned real-crate evidence

- Fifteen runner regressions run before setup. Twenty-five source/preparer
  checks include the eleven TOML cases, five adapter checks, three Key checks
  and six item-fixture checks. None of those Python checks proves native typing.
- Fetch the pinned official repository and its exact lock on the clean public
  runner. Prepare the fixture from the original modules and byte-exact official
  Names/styled_name fragments. Native ItemStack, Text and Language are actual
  dependencies, not replacement types.
- Resolve the copied fixture lock offline after official locked dependency
  fetch. The preparer admits only official registry identities and exact local
  native package paths, plus the one named fixture. Verify the result before
  any compilation.
- Observe real rustc invocations for horizon-client-core, horizon-items,
  horizon-world and the fixture's test target. Execute all 22 tests once with
  no filter, ignored or skipped cases. Classify the complete named results as
  item display 8, Key/Text 6, and internal loader 8. The subsets are not separate
  repeated runs and are not counted twice.
- The original author example is compiled inside that bounded fixture using an
  explicit horizon_client alias. This checks the public-path code against the
  actual modules and core Text type; it is not an installed horizon-client build.
- Check source/file-set/lock identities and generated inputs before and after.
  Fail the receipt if execution logs show rustc recompilation; this detects
  recompilation after execution, rather than blocking the compiler before launch.

The official Names/styled_name fragments cover standard item/block name fallback,
selected language, custom-name priority, explicit item_name re-resolution and
translation argument/style component preservation. The actual large Game tooltip
body and content-default propagation route are only source-pinned, not executed.
The tests do not prove arbitrary unknown NBT/Wire or invalid UTF-16 preservation.

## Deliberate limits

This job does not plan/apply the full native package, instantiate Game, draw a
window, perform a resource reload, prove pack/content overlay priority, load a
real admitted hmod through the entire pipeline, synchronize a guest language,
or refresh retained Guide pages. The source preserves the existing title-based
screen/LAN identity behavior. Those paths need their own acceptance gates before
claiming automatic translation of a particular UI.

## Execution safety and evidence

Only the exact dedicated branch in the public repository triggers the job.
Permissions are contents-read, checkout credentials are not persisted, and no
cache or artifact upload is used. No billing/settings/PR changes are made.

All commands use one job and zero debug/incremental output. The preflight
requires 10 GiB +128 MiB plus a 2 GiB setup reserve on every output filesystem;
the floor is monitored while subprocesses run. Each owned process group is
terminated and its absence checked with bounded waits, including normal leader
exit. Official and Mod checkout cleanliness includes untracked files.
Failures preserve partial gates, verification failures and a false final receipt.
The corresponding game/UI/resource claims remain unproved after a failure.

Preparation history: the first local materialization had a source-byte mismatch,
which the new input-hash regression rejected before any Cargo/native action.
Inputs were rematerialized verbatim and all twelve Git blob identities checked
against the exact remote source. The subsequent fifteen runner and twenty-five
source checks passed. The initial failure is not a native compatibility result.
