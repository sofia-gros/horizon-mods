# Excluded item-name removal CI

Status: first actual candidate plan/guard/apply passed in run 42196270581, but the fixture preparer failed before native test compilation. The 32 Rust tests remain unrun. The fixture-only correction and original partial receipt are preserved; no corrected retry has run. This workflow does not activate the candidate in the ordinary PlutoLib package.

Source base: `47422d690b08f9db0575e5b5cb5d2260cb04f02f`. The five runtime inputs are unchanged from `0f528de0332afc5d86910dd07b08dcebe98a04dd`; the validation body/oracle remain the originals from `29a34ed5`. Only the narrowly checked official export marker handling was corrected at the later base.

## Execution boundary

This is a public, read-only-token, standard GitHub runner workflow. It accepts a push only to `checkpoint/plutolib-item-name-removal-ci-20261106` in `sofia-gros/horizon-mods`. Publishing these files to the separate source checkpoint does not execute it. There is no manual-dispatch, pull-request trigger, package upload or merge step.

It cold-fetches only the immutable official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and official Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`. Exact lock hashes are checked before a locked dependency fetch. Thereafter the actual CLI build and fixture compilation are offline and locked. The fixture's own copied lock is first adapted by a separate offline metadata operation; every identity must be from the official lock except the single local fixture package. Native path dependencies must point into that same official checkout.

Separate gates establish:

1. Python runner, adversarial exclusivity guard and preparer checks.
2. Actual official CLI compilation, actual isolated plan, whole-guest/receive exclusivity guard, and actual apply. The applied marker must equal the checked plan, but that marker alone is not accepted as evidence of applying it.
3. Byte-exact extraction of the applied native regions and separate pristine native oracles. This is not an installed whole-client build.
4. Actual compilation of the official core, client-core, items, protocol and world crates, plus exactly one test compilation of the fixture.
5. Direct sealed-binary inventory and one complete 32-test execution: 18 actual native codec cases, 8 one-shot entry-gate models and 6 owned-Tag cases. Exact names, no omissions, no ignored/filtered tests. The same full execution is split into three result groups; it is not run three times.

The runner seals the entire Mod and official checkouts, including ignored/untracked entries, regular-file mode bits, directory presence/kind and symlink targets. Directory permission bits are not part of this seal. Applied stage and fixture file sets, generated OUT_DIR source, executable, CLI, plan and locks are rechecked even on failure. Test execution invokes the sealed binary directly, so Cargo cannot silently recompile it. Child process groups receive bounded termination and must be absent after every phase, including a normal leader exit.

All writable filesystems must retain 10 GiB + 128 MiB, with another 2 GiB available before startup. Jobs are serial, debug info and incremental outputs are disabled. Raw command logs remain ephemeral runner files and are validated in full; console output contains bounded diagnostics, compiler witnesses, exact names and receipts. Their hashes do not make the raw logs retrievable later. No artifact-upload permission is requested.

## Meaning and limits

A successful result would establish this isolated composition and its finite regression cases, including panic restoration, concurrent thread independence, nested one-shot consumption, native codec preservation and raw-versus-canonical send distinctions. It would not establish universal TLS safety, unbounded-depth stack safety, arbitrary third-party hook composition, full Game execution, LAN delivery, gameplay, all opaque/NBT preservation or a default item-name owner.

The sender reserves the relevant native boundary; an extra hook/module/state in the reserved files fails before apply. The ordinary package remains disconnected regardless of this job's result. Raw usable wire is deliberately echoed through the native raw path; a semantically edited stack uses a separate canonical proposal. Unknown Tag preservation is only claimed at the adapter's tested input/output boundary, not across native lossy decoding. Default component ownership and the actual Game raw-path call site remain separate work.

## First attempt

Run 42196270581 at exact source `014d99aaca6fbcf71dc57b01171a58a60aa0f1de` failed before fixture extraction/metadata/native compilation. Its Python 23+12+5 checks, actual official CLI, plan, exclusivity guard and apply passed. Final source/stage verification passed; overall result was false. The preparer omitted the native CLI's unconditional empty post-build manifest from its expected file set. The correction adds only that exact generated empty file, reserves both generated names against input collisions, and preserves unknown-file rejection. Preparer regression count is now eight; all 32 runtime test bodies and candidate runtime are unchanged.
