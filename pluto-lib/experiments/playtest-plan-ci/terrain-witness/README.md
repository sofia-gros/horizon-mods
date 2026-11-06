# Excluded latest-terrain plan witness

This is a read-only observation utility, not a new compatibility guard and not
part of the ordinary PlutoLib bundle. It does not run Mixins, invoke a compiler,
apply a plan, update metadata, change a cache, or execute rendering.

## Fixed scope

- Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf`.
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

## 41f contract delta and limits

This cutoff preserves the collector implementation byte-for-byte. Only the
contract's Horizon revision and `game.rs` source SHA-256 change from the frozen
f4dc collector `5b87b28d`. The eight payloads, two hooks, six modules, bounds,
owner/binding checks and observed-only/null-compatibility behavior are unchanged.
No expected node or fingerprint was filled in, and no production guard changed.

`source-delta-41f.json` records thirteen exact Git-tree blobs: twelve are equal
to f4dc (including Cargo.lock and all selected renderer files). The complete
six-hunk official game.rs delta was applied in memory with all old context lines
checked; the resulting full bytes match the independently read latest Git blob.
The whole PackResources impl, including assemble, is byte-identical. The entire
mesher.rs file, including mesh_reusing, is byte-identical. This establishes
selected source identity, not a successful latest plan or type compatibility.

Twenty-two pure Python regressions use the actual pinned payloads and synthetic
plan shapes. They include exact reversal of the two contract value changes and
rejection of the old game.rs hash. No official planner/compiler/rendering runs.
The controller must pin all five files here, pass an explicit repo-shaped
`--mod-root`, verify complete source/package provenance, and record whole-plan
SHA/bytes BEFORE invoking this collector. A collector refusal must remain an
observation failure with compatibility null; do not skip it or update a guard.

The f4dc full-646 attempt `74fc9a53` / run `42215886835` ended after 580.106 seconds
with `E_UNSUPPORTED_CONTROL_FLOW` at `tinkers-chest-pick-color` / Game::picked_stack.
It produced no accepted plan; collection and the twelve old guards were not
reached. No 41f terrain witness exists at this cutoff. Prior CPU success remains
624c/e8 evidence only. There is no new Rust/apply, renderer CPU, reload, streaming,
GPU, playable Windows artifact or performance proof from this source change.
