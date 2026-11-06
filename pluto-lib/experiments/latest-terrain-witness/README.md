# Excluded latest-terrain plan witness

This is a read-only observation utility, not a new compatibility guard and not
part of the ordinary PlutoLib bundle. It does not run Mixins, invoke a compiler,
apply a plan, update metadata, change a cache, or execute rendering.

## Fixed scope

- Horizon `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`.
- Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`.
- Existing common bundle `4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d`
  supplies the unchanged eight Rust payloads. Its old revision compatibility
  and production guard metadata are not modified by this utility.
- Exactly the reviewed `PackResources::assemble` preparation HEAD and
  `Mesher::mesh_reusing` replacement HEAD, with six deduplicated support modules.
- Thirteen selected latest official source/lock hashes and all eight actual
  helper/module payload hashes are checked before and after collection.

The expected node indices and whole-file module fingerprints are deliberately
not copied from the old guard. They are observations from the supplied plan.
Schema 6 support modules expose a destination file and fingerprint, but no AST
node field. The utility records that distinction rather than inventing a node.
Owner, deterministic packaged ID, destination/name, visibility, phase, priority,
bindings, return contract and payload remain fixed. Unexpected owners, missing
entries, duplicates, changed source/payload bytes and exceeded budgets reject.

Other injections are reported when they target either canonical function, share
its observed resolved file/node or function fingerprint, or enter the entire
reserved mesher file. This matches the existing guard's conservative scope.
Competitors are **observed**, not silently removed or accepted. A zero competitor
count is not a compatibility verdict. Source-identical functions at other sites
can appear as conservative fingerprint matches and need human review.

## Usage after a separately authorized real plan

```
python3 pluto-lib/experiments/latest-terrain-witness/collect.py \
  --plan COMPLETE_PLAN.json \
  --official-root EXACT_PRISTINE_OFFICIAL_EXPORT \
  --mod-root EXACT_REVIEWED_HORIZON_MODS_SOURCE
```

Only compact JSON is printed. It contains the complete input plan's SHA-256 and
byte count, selected source hashes, actual compiler/version strings, two hook
records, six module records, payload hashes/lengths and bounded competing-hook
records. Rust payload text is not echoed. Success means `status=observed_only`,
with `compatibility=null`; rejection also retains that non-verdict. No files are
written by the collector. The caller must preserve the console receipt if needed.

The caller's CI controller must independently establish the exact official
checkout/CLI, complete package and plan provenance. These selected-file checks
do not establish full source ancestry, authenticate arbitrary JSON, prove the
other modules, or run any existing complete-plan guard. Contract file integrity
also depends on that controller's immutable source seal.

Budgets are 32 MiB raw plan, 4,096 hooks and 4,096 modules, 20,000 input entries,
2 MiB per selected source/payload, 64 competitors and 128 KiB output. Reads reject
symlinks/nonregular inputs, duplicate JSON keys, nonfinite JSON literals and
observed file changes. No concurrent hostile filesystem/TOCTOU guarantee is
claimed; execute against controller-sealed quiescent inputs. A budget rejection
requires explicit investigation, not an automatic retry with a larger limit.

## Current evidence and limitations

- Twenty pure Python regressions passed using the eight **actual pinned** Rust
  payloads and explicitly synthetic plan shapes. Tests cover changed observed
  node/fingerprint values, owner/payload/source drift, aliases, whole-file
  competitors, missing/duplicate entries, bounds, and symlink/path rejection.
- A separate read-only check matched all thirteen selected latest official
  files and eight existing payloads. No latest plan was substituted into tests.
- The first test invocation exposed an eager temporary-directory default in the
  test harness. The next exposed a nonregular-input exception/fd lifetime in the
  reader. Both were corrected before the final twenty-test pass.
- Latest actual main+common diagnostic `ada5a943` / run `42204940940` ended after
  575.217 seconds with `E_LOCAL_NOT_FOUND` for `source-host-mining-context` at
  `Game::tick_mining` (`speed:0`). It did not yield a successful complete plan or
  reach the twelve guards. Therefore **no latest terrain witness exists yet**.
- This excluded source has not been installed in a CI runner. Production guards,
  runtime payloads, the current diagnostic and the CLI remain unchanged.
- Old CPU success belongs to Horizon `624c` / Mixins `e8`, not this latest cutoff.
  There is no new Rust/apply, renderer CPU, Game reload, streaming, GPU or
  performance result from this artifact.
