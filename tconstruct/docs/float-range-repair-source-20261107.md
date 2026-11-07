# Generated float/range boundary repair candidate

This is a source-only candidate for the fixed Horizon 41f822b6 / Mixins 5a64af93 build. It has not repaired a retained actual generated file, run a Rust parser, reproduced prettyplease natively, or passed a new game build. Twenty-four bounded synthetic Python tests pass. The exact pristine file and the reviewed full plan's target-file closure also pass read-only source checks.

## Evidence and cause

[The failed build](https://github.com/sofia-gros/horizon-mods/actions/runs/42334764929) completed fresh planning, all guards and official apply before Cargo rejected `if !(0...=1.).contains(&frequency) {` in generated `start.rs`. The immutable result closure is e752b933a3b2722079fda983188577cdb089195e. The actual generated full file was not retained.

[The official original](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/crates/horizon-world/src/worldgen/structure/start.rs#L441-L450) has `if !(0. ..=1.).contains(&frequency) {` at line 448 in `StructureRegistry::new`.

[Official transform](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-transform/src/lib.rs#L341-L343) unparses affected complete files with prettyplease. The pinned 0.2.37 archive checksum is 479ca8adacdd7ce8f1fb39ce9ecccbfe93a3f1344b3d0d97f20bc0196208f62b. Its relevant cached sources equal the archive members byte for byte. In [expr.rs](https://github.com/dtolnay/prettyplease/blob/c971184fa8c5ef5a2828196e35bd99469455b46b/src/expr.rs#L796-L817), a range with a start expression prints that expression immediately followed by `..=`. The dot-separation branch applies only to a range without a start expression. [lit.rs](https://github.com/dtolnay/prettyplease/blob/c971184fa8c5ef5a2828196e35bd99469455b46b/src/lit.rs#L46-L47) preserves the float token spelling `0.`. The word/string writer adds no separator. This static path explains `0.` followed by `..=` becoming `0...=`. A native serializer reproducer has not been run.

The companion JSON records the exact official locks, transform/native source hashes, archive checksum and relevant prettyplease member hashes. No dependency or official source is updated.

## Exact repair contract

`stage_float_range_repair.repair(original, modded, mixins_source, plan_path, *, horizon_revision, mixins_revision)` runs only after normal complete-plan guards and official apply. Rendering owns that caller and the launcher pin. It takes no new stage command-line option.

The helper requires the exact original and serializer source pins, plan schema/input hashes, and the reviewed target-file patch closure: one `try_generate` HEAD and nine support modules, with no injection in `StructureRegistry::new`. The closure came from the retained dc4f Linux plan. The fresh Windows plan must independently match it; a mismatch is a refusal, not permission to update the pin.

The generated file must contain exactly one malformed spelling, zero corrected spellings, one exact method signature, surrounding frequency/placement text, method tail and following member. Uniform LF or CRLF is preserved. These are narrow textual admission checks, not a Rust AST parser.

Only one ASCII space is inserted after `0.`. The helper proves exact full-file inverse and checks all other generated files, modes, directory entries and locks unchanged. It also rechecks the pristine pins and complete plan bytes. Symlinks/junctions, changed roots, repeated invocation, unknown spelling/context and budget violations are rejected. Bounds are 64 MiB per file, 2 GiB/20,000 generated entries and 16 MiB for the existing plan.

The inserted space restores the original token boundary. It does not change the `0.` or `1.` literals, inferred f64 frequency, closed interval, frequency default, or NaN rejection. It does not reconstruct the function, alter guards, remove features, or use a general replacement rule.

A fresh `float-range-repair-receipt.json` beside `modded` records target, before/after hashes and byte counts, one-byte offset, inverse and stage seals. It is at most 8 KiB and explicitly sets `ast_verified=false`. Ordinary subsequent Cargo remains the actual syntax/build gate. No `rustfmt --check` is used, since unrelated formatting differences can produce a nonzero result.

All admission checks precede the generated-file replacement. A later readback/seal/receipt failure stops the stage without a success receipt; it does not automatically retry or claim rollback. The driver must preserve that failure separately from later Cargo results. The source candidate contains no CI trigger and makes no build-success claim.

## Validation scope

Run `PYTHONDONTWRITEBYTECODE=1 python -B tconstruct/tests/test_stage_float_range_repair.py -v`. The 24 tests cover exact inverse, LF/CRLF, duplicate/already-correct/context/method rejection, pinned inputs and plan closure, fresh receipt, root/link rejection, whole-stage preservation and post-write drift detection. Their fake source trees and pins are explicit test fixtures. An artificial malformed copy of the exact pristine source also round-trips to its original bytes; it is not actual prettyplease output.

The helper and tests remain a separate candidate until independent review and the connected stage source are approved. No local Cargo, metadata, native build or CI was run for this candidate.
