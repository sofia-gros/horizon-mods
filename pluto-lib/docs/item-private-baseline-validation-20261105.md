# Applied native retained-baseline comparison

## Frozen input

- Complete integrated MOD source: `4d82580b32a1c040001add8c853d93d331bc493a`.
- Official Horizon: `624c4f6265c02296fbe8a80f029dd0d987fb94c0`.
- Official Mixins: `e8cd0aa38cffef49cb2e088ef716baa107148c02`.
- Complete library archive SHA256:
  `0bc30a5d85807cf09a04f67092246873311ed87c897333de2a8cf4dbb9d4ab84`.
- Applied stage: `upgrade-official-baseline-20261105/.work/plutolib-foundation-proof/current-staged`.
- Frozen proof metadata: the adjacent `grouped-v9/cutoff.json` and
  `grouped-v9/stage-source-sha256.json`.
- Full 5,039-file applied-stage seal SHA256:
  `9eaeccb885f497dfc6f7b023cc9d87c85481f9ab6cef5284daae631206f0a6d5`.

The complete library has 2 API crates, 53 native modules, 30 injections and 3
states. Before this bounded run, its actual plan, all seven mandatory preflight
guard groups, apply and native library/binary check passed. The new reader and
comparison are additive modules; they add no state, hook or consumer write path.

## Executed boundary

The preparer used the real applied `horizon-items` crate, including the inserted
child module that can borrow native Wire's private decoded baseline. It copied
only the unchanged packaged `stack-storage.rs` and `stack-snapshot.rs` client
modules into a small test crate. No native declaration or behavior was mocked.
The fixture resolved one actual stage-packaged inventory API and actual native
path dependencies within that stage.

Every original stage input matched the clean official checkout. The full applied
stage matched its frozen seal both before and after execution. The native module
identities, source bytes, reviewed reader bytes and exact official private
declaration were checked. The official lock seeded dependency resolution, which
added only the fixture and actual API packages. Locked offline metadata checks
revalidated all resolved package identities and native paths after execution.

Command: `cargo test --locked --offline --lib -- --test-threads=1`, using the
prepared fixture manifest. The shared target used two jobs, debug info disabled
for dev/test and incremental compilation disabled. An external continuous disk
guard sampled available space every 250 ms and would stop this process group
below 10 GiB + 128 MiB.

## Result

Terminal exit 0. All 8 required real native tests passed, with 0 failed, ignored
or filtered. Compilation took 26.33 seconds; the test execution reported 0.00
seconds. Required-case verification and final source/lock verification passed.

The executed cases establish:

1. The reader returns the actual borrowed decoded baseline and patch.
2. Different stale baselines with identical exposed bytes and usability are
   rejected by the additive comparator while the observable API remains equal.
3. Independently allocated equal values pass; different bytes or wire presence
   fail without an address-based equality shortcut.
4. Baseline count, native component and runtime bundle-selection changes fail.
5. Container, charged-projectile, pot-decoration and bundle-content children all
   reach the private-baseline comparison.
6. Baselines containing further retained baselines are compared recursively.
7. Independently allocated 24-level shared baseline DAGs finish without expanding
   their 2^24 paths; a different deepest leaf is rejected.
8. Inputs stay unchanged and the persistence adapter still refuses all retained
   wire, including nested or stale backing.

Initial free space was 11,089,649,664 bytes; observed minimum was 10,946,400,256;
final free space was 10,946,433,024. The guard did not stop execution.

## Retained evidence

- Fixture: `.work/pluto-retained-baseline-proof-4d82580b`.
- Fixture `fixture-sources.json` SHA256:
  `5aafe6cef79fec669f1a30ed91bf2c6a715d050a2c12bea482e1895da0dd2299`.
- Official lock SHA256:
  `463913123985e344f2889e5beae65982773e6dbfacdcac7a916bcec522a5bd31`.
- Resolved fixture lock SHA256:
  `9f96a060640d1724e5b702670aaf91515191560d315c64f9a5a7746ec6e871e9`.
- Full local preparation/test/verification log:
  `/tmp/pluto-retained-baseline-v9-proof.log`, SHA256
  `6b2b774e5472f097ade5ddf9151ded07f7ad1c69dd7ee6707f93532c0ae6d287`.
- Native test log: `/tmp/pluto-retained-baseline-native-tests.log`, SHA256
  `77256f5ad1598cd2db0bb1ab79a78b06939b75b27ec5b5b6eba3a3676423f90e`.

After all v9 gates and this fixture reached terminal success, the coordinator
archived every stage byte and verified all 5,039 payloads with no open handles.
The retained archive is `.work/official-cleanup-evidence-20261105/grouped-v9-staged.tar.gz`
(45,586,663 bytes), SHA256
`8cc4445ecea4612b0729af70c997da8953bc034da388f795192a1dca39c06f23`.
The fixed stage directory was then removed; v9 cutoff, seal, locks and logs remain.

A later cutoff may replace the fixed stage path. The seal must then fail until
the exact archived v9 stage is restored; later contents cannot be relabeled as
this proof. The copied fixture source and evidence alone do not recreate native
private-field access.

This closes the bounded native reader/comparator gate, not a caller migration,
live authoritative transaction, network round trip or complete TConstruct
runtime gate. Existing observable comparison and all consumers are unchanged.
