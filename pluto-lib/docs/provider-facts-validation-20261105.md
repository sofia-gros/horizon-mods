# Bounded native provider facts validation, 2026-11-05

The complete Pluto library source cutoff is
`4244c26773476c8a4e6b23188e828da28105248a`, packaged as archive SHA-256
`d702b7104a889d2ec59fed60ba60763c7fc52c387c25a34cd42efcc58ab7ad70`.
The fixture preparation tool is `7a08e11146eff1ddc0e4efc979f174c3d90b7052`.

Official Horizon is `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins is
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. The separate complete-library plan,
preflight, apply and client library/binary check for this cutoff passed before
this bounded fixture ran.

## Result

Seven existing tests passed using the actual packaged native module sources:

- Two component-view tests: exact UTF-16/float bits and default-versus-unavailable data.
- Two equipment-snapshot tests: captured slots, empty-versus-unsupported slots,
  cloned components and isolation from later source mutations.
- Two protection-provider facts tests: native known/unknown catalog answers
  and captured equipment rather than a fresh live read.
- One contribution test: scoped changes cannot overwrite another provider's
  entries, using the actual staged native `Policy` implementation.

No client-library test binary was built. The fixture used the real staged
`horizon-world`, `horizon-items` and `horizon-client-core` crates, plus exactly
one stage-packaged `pluto-inventory-api` instance. The complete unmodified
native `advancements.rs` was compiled as a small dependency to supply its
actual `ItemView`. Its brewing/NBT references use the real core potion and NBT
reexports, not substitutes. No native struct, codec, catalog or Policy was
reimplemented for the test.

`prepare_provider_facts_fixture.py` checks the clean official revision/origin,
all original stage input hashes, archive hash, exact module bytes and packaged
API files. It seals all staged and fixture files and verifies resolved package
identities against the official lock plus the three explicit local packages.
Source, API, path and lock verification passed both before and after testing.

## Execution and limits

The serial run used the existing shared target, two jobs, no debug information
and no incremental compilation. The continuous 10 GiB + 128 MiB disk guard did
not stop it. Minimum free space was 11,022,041,088 bytes; terminal status was 0.
Two preparation/source guards passed separately.

Local evidence is `/tmp/pluto-provider-facts-proof.log`, with sealed fixture
inputs at `.work/provider-facts-proof-4244c267/fixture-sources.json`.

This proves isolated converter, capture, catalog-adapter and contribution
semantics. It does not execute native Game hook dispatch, a live damage event,
world persistence or LAN behavior. Native surrounding wiring is covered only
by the separately recorded complete package apply/check, not by these tests.
The native world dependency's own test suite is not rerun merely because it
is linked into this fixture.

## Nested opaque component fallback

A follow-up at fixture/tool commit
`25ed7a494804d9818c3005d31420713dd097f7bb` passed eight tests against the same
unchanged `4244c267` native package. The additional real-type matrix covers
container contents, charged projectiles, pot decorations and bundle contents,
each with usable and stale retained wire on the child only.

`ItemFacts::capture` and the protection provider's `Equipment::capture` both
report `components=None`; the outer item ID, count and damage remain available.
The original native snapshot remains unchanged. An ordinary nested stack has
available component data, so absence is not mistaken for an empty patch.

The complete native source/path/API/lock verification passed again. Terminal
status was 0, with eight passed and none failed/ignored/filtered. The continuous
disk guard minimum was 11,081,048,064 bytes; it did not stop the run. Evidence is
`/tmp/pluto-provider-nested-proof.log` and the separate sealed fixture
`.work/provider-facts-proof-4244c267-nested/fixture-sources.json`. The original
seven-test fixture remains preserved. This closes the direct cross-service
fallback test gap without adding a mutation capability or live Game claim.

## Preserved stage archive

After the later complete v6b library proof passed and both provider fixtures
were terminal, the superseded v5 generated stage was archived to recover disk
headroom. All 5,031 regular file payloads were SHA-256-checked against the archive
before the original directory was removed; no active file/cwd handles existed.
Source checkouts, shared compiled dependencies and the current stage were not
removed.

The complete stage is preserved in
`.work/official-cleanup-evidence-20261105/grouped-v5-staged.tar.gz`, SHA-256
`285292839efba66492c9261bd5051cdaf3651a6764c0de509290d85c75a5228b`.
Metadata, official revision, Cargo lock, native hooks and source cutoff were
also retained separately. Restore the archived `staged` directory at its
recorded original path before rerunning the sealed provider fixture's source
verification. The archive preserves the exact tested bytes; it is not a new
source revision or a replacement test result.
