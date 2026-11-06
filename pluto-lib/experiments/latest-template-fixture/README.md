# Latest Template fixture preparation only

This separate group prepares the six unchanged Rust cases from `fe5f2d41` against
Horizon `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`. No real preparation,
metadata, Rust compilation, test execution, plan/apply, workflow or production
activation has run. The old successful item-name codec32 remains a separate
`624c4f62` proof. The two proposed old-fixture literal changes remain unapplied.

`prepare.py` accepts only a clean official Git checkout at the exact revision and
Cargo.lock hash. It seals all regular bytes/modes, directory kinds and symlink
targets without traversing links, including ignored/untracked cleanliness. It
checks the entire frozen seven-file source group and this four-file preparer
group, with exact six test names. Extra files, directories and links are rejected.

The generated fixture contains the original test bytes, one byte-exact native
Template region, a real-path dependency manifest, a copy of the official lock and
a receipt. The region includes the actual declaration, parse/create/over,
assembly_components and json_tag; unique boundaries and its hash are mandatory.
No visibility or algorithm rewrite, mocked native type, source-file substitution,
feature gate or public API change is introduced. Actual items/client-core/world
path crates retain the official dependency direction; core is checked transitively.

`--resolve` is a separately authorized **offline metadata** step, not compilation.
Only the copied fixture lock may adapt to the smaller real-host graph. Every
resolved package must belong to the official lock or the single harness; all four
required native packages must have unique, exact official paths. The original
lock, official tree and frozen source are rechecked after metadata. Subsequent
Cargo compilation must use `--locked --offline` and an outer runner with actual
compiler evidence, full six-name inventory, one sealed-binary execution, output
seals, filesystem floors and bounded process-group teardown. This preparer does
not itself grant or perform that CI step. Git/metadata calls have a 60-second
bound and inherit the outer controller PGID. The helper refuses to launch them
unless the dedicated controller marker is present and this process is its PGID
leader. Internal timeout kills/reaps only the direct child; outer unconditional
group teardown also covers descendants if the preparer is killed. Generated output must be outside both
Mod and official source, and new; extra output files or links fail verification.

The pure Python checks test guards only. They do not prove native types or Rust
behavior. A future pass would show these finite Template raw-patch cases, not
arbitrary component losslessness, full crafting/menu behavior, output/remainder
transaction authority or latest ordinary-package composition.
