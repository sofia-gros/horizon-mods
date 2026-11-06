# Bounded consumer identity helper proof

Status: the exact production helper fixture passed its dedicated CI at
`93424e7914f8e31ca289beb29254c4e7740c4f82`, run
[42187284494](https://github.com/sofia-gros/horizon-mods/actions/runs/42187284494).
The immutable `inputs.json` describes the original source-preparation cutoff;
the later [result receipt](../../docs/guide-identity-ci-result-20261106.json)
records actual execution without rewriting those frozen inputs.

The fixture includes the exact production BookId and page-index helpers, without
copying their algorithms or introducing a mock Text. A standalone `rustc --test`
build needs only the Rust standard library. It does not compile source_pages or
any native host dependency, and does not package/apply a Mod.

`inputs.json` pins the source candidate, complete helper and fixture hashes, all
21 discoverable test names and two separate five-case filters. Eleven pre-existing
index tests are compiled but not selected. Each five-case run must report exactly
five successes, zero failures/ignored/measured tests and sixteen filtered tests.
The two groups must not be combined into a claim about the native Font12 or i18n
proofs.

For the separately authorized CI runner, the bounded commands are:

```sh
rustc --edition=2021 --test --crate-name guide_identity_tests -C debuginfo=0 \
  tconstruct/tests/guide-identity-proof/fixture.rs -o "$OUTPUT/guide_identity_tests"
"$OUTPUT/guide_identity_tests" --list
"$OUTPUT/guide_identity_tests" identity::tests:: --test-threads=1
"$OUTPUT/guide_identity_tests" index::modifier_anchor_tests:: --test-threads=1
```

The runner must create its own exclusive output root, check source hashes before
and after, require an actual compiler invocation, validate all listed/selected
names and counts, and retain the normal disk/process guards. No local execution
is authorized merely by this README. No Cargo, network fetch, native resource
tree, Game factory or installed-consumer claim is needed for this narrow proof.

The existing 23 Python oracle/scope checks passed locally at the runtime source
candidate. Their historical Git-oracle reads use the local immutable audit
objects; they are not implicitly runnable in a shallow remote checkout. Any CI
that adds those checks must explicitly provision the corresponding published
baseline tree and verify its pinned source bytes. This helper-only fixture does
not pretend to run those Git-dependent guards.

The ten selected Rust tests passed and prove only the two pure production
helpers' tested behavior. Actual source_pages typing, native Text/Font generation,
complete consumer hook compatibility, screen lifetime, localization refresh and
LAN behavior remain separate gates.
