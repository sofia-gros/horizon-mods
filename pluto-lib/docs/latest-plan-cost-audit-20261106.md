# Latest planner timeout: bounded source audit

Status: source inspection only. The timeout's internal phase and cause are **not determined**. No CLI, production code or guard was changed, and this audit ran no native program.

Inputs are Horizon `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd` and Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`. Historical comparisons use `624c4f6265c02296fbe8a80f029dd0d987fb94c0` / `e8cd0aa38cffef49cb2e088ef716baa107148c02` only. The exact main is `08afa992d5091e03360f3473ea93fd17aa793987`; the common archive is SHA-256 `4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d`. The latest candidate changes only two manifest revision lines.

[Run 42201486827](https://github.com/sofia-gros/horizon-mods/actions/runs/42201486827) timed out after 180.231 seconds with 0-byte planner stdout, according to its terminal report. CLI build, source/archive checks, inverse revision edits and final seals passed. No E_* diagnostic or complete-plan guard result was reached. Compatibility remains undetermined.

## New glob traversal is not an established explanation

The new [glob resolver](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/modules.rs#L94-L178) has no memo or visited set. Each `file_of` scans `source_owners`; recursive glob edges may revisit the same file and name. Depth greater than eight conservatively rejects, as does a matching already-planned support module. Short-circuit rejection can terminate a cycle before every branch is visited. This establishes possible repeated source work, not a measured recursion problem.

The exact manifests contain 317 support modules (248 main plus 69 common), targeting 35 source files. Bounded source inspection found **no top-level glob import in those destinations**. Nested test `use super::*` statements do not enter the [top-level `Item::Use` check](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/modules.rs#L58-L75). Seventeen latest destination blobs equal the old source; the eighteen changed bodies were fetched at the exact latest SHA and checked against the official Git-tree blob IDs. The JSON records all 35 paths and module counts.

This is a bounded textual/source inspection, not instrumentation of the official syn visitor, macro expansion or a runtime call graph. No concrete entry from this module inventory into repeated glob traversal was found. Do not attribute the timeout to the new resolver on this evidence.

## Existing repeated work worth observing

1. **State coverage:** main declares 108 states, common three. All target ChunkMap and the same explicit Default constructor. For each state that passes earlier checks, [state.rs](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/state.rs#L255-L309) independently calls `Sites::in_block` over every indexed function, then traverses patterns. It does not share coverage results across states with the same target. [Sites](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/sites.rs#L158-L181) clones and token-fingerprints each visited expression, including nested subtrees and additional call/constant entries. Both files are byte-identical between the old and new Mixins cutoffs. An earlier diagnostic can stop before all 111 declarations are processed.
2. **Module fingerprints:** the [module loop](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/plan.rs#L375-L440) repeats destination checks and `fingerprint(tokens(tree))` for every support module. Latest game.rs is 1,031,429 bytes and is the destination of 155 modules. The source-byte count is not a token count or measured work estimate. The old planner already repeated this fingerprinting.
3. **Earlier indexing:** source hashing, offline locked metadata and AST indexing precede these loops. A timeout with no phase observation cannot distinguish them from state, module or later injection work.

## Exact ordering matters

[build_staged](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/plan.rs#L314-L375) first loads manifests, hashes source inputs, resolves native hooks, loads the source index, and prepares bundled crate/data/post-build inputs through `native::plan`. That function name does **not** mean per-instance `state::plan` has run.

Bundles are sorted by mod ID. For each bundle, its exact revision is checked **before** its states, support modules, bridges and injections. Thus common's three states/69 modules/34 injections are considered before the main revision gate. Main's 108 states/248 modules/three bridges/290 injections are after that gate, if all earlier checks succeed.

The historical unchanged main+common run's 21.544-second E_REVISION is therefore not an old/new speed benchmark: it rejected before main's 108 state declarations. The latest candidate explicitly admits that additional work by changing the two revision lines. Neither comparison proves a performance regression nor confirms where the 180-second attempt stopped.

## Evidence and next boundary

Pure source checks confirmed the two manifests' counts and all 111 constructor declarations, the 35 destination inventory, 17 unchanged Git blobs, 18 fetched latest Git blobs, and unchanged state/sites source bytes. The GitHub latest tree had 5,956 entries and was not truncated. Source details are in the companion JSON.

A separately authorized, bounded observation of the unmodified CLI may record child argv and own-process elapsed/CPU samples even on failure. Until that yields evidence, all cost explanations above remain candidates. Do not increase scope, change CLI validation, relax guards, or label old CPU/renderer results as latest compatibility proof.
