# Minimal official Index observation proposal

Uncompiled source, not a CLI extension or a new CI. Offer this to the next
existing diagnostic controller only. It resolves three exact original native
functions and uses public official tokens and fingerprint, exactly like
plan.rs. It does not create a manifest/hook, call build_plan, transform source,
modify an original checkout, or run Game. The known native observer and
handle_packet plan fingerprints are control assertions; poll_remote remains
unknown until execution. Index::load itself invokes locked/offline/no-deps
Cargo metadata and scans source, so it must stay in the existing guarded
process group and a separately bounded phase, with before/after source seals.

Use a separate tiny package outside both source trees. Dependencies are the
exact official 5a64 horizon-mixins-core path plus serde_json =1.0.154, copied
from that official lock. Seed that unchanged official lock and admit only the
one new harness local package identity when adapting it. No new dependency
versions, fake token parser or official-source edit is proposed. The controller
owns manifest paths, lock/metadata verification and compilation/timeout budget;
this source does not authorize execution or promise cache reuse.

Official 5a64 source evidence:
- lib.rs Git blob 0b55bac3da16482dee9ea9035f2279f5ff44e1b6 exports index/source.
- index.rs SHA256 c9ddfb2233bebb472d8d548a4756785a5ca526f3d3ef4a2824fd6fb039ce4711: public Index::load, Index::resolve, Function signature/block/file/node, tokens.
- source.rs Git blob 00d786bc82f6c2d1073f976cab6039e4e1135a6e: public fingerprint accepts bytes.
- plan.rs SHA256 5226f837305a778f73cf428411d5cf7a1cf1b8b33de2fd603a1459e7863e32db, lines488–489: exact same token concatenation/hash.
- core Cargo.toml Git blob 905ddb159ca5c99a576e3792c0fcf2bb41f3099c; official lock has core0.6.0 and serde_json1.0.154.
- CLI main SHA256 c051512984c7e19a53c45a90a551f620754194abb6bb285b60e3ac4322169678 has only Plan/Apply/Check, no inspect command.

No Rust, metadata, compile, planner or additional remote job was run to prepare
this source. Successful observation would supply source identity only, not
caller composition, transport, lifetime or installed-host proof.

## Pure preparation and admission helper

index_probe.py never invokes a subprocess. Its four functions are:

- prepare(root, mixins, main_source): fresh external single-member manifest,
  exact official core path, exact serde_json version and original lock seed.
- verify_lock(adapted_bytes, seed_bytes): permit only an exact official subset
  plus pluto-packet-index-observer 0.0.0. Retained registry identities, checksums
  and dependency edges are unchanged; the actual core and serde_json remain.
- verify_metadata(full_metadata_dict, root, mixins, adapted_bytes, seed_bytes):
  require the same exact package set, only core/harness local identities, exact
  source paths/targets, a unique workspace/default member and full resolution.
  Return the actual core features as evidence; do not assume they are Fresh.
- validate_result(raw_stdout_bytes): admit only one bounded compact JSON result
  with a terminal newline, three ordered targets, two exact fingerprint/node
  controls, and the observed poll_remote identity. Duplicate fields, extra
  rows/lines, floats/nonfinite numbers, unknown keys and malformed nodes reject.

The outer controller owns clean official Git/revision/full-file seals and must
also keep the new package outside the Horizon workspace. Seed the manifest with
prepare, run a separately captured full `cargo metadata --offline` for lock
adaptation, admit that lock, then require full `cargo metadata --locked --offline`
before compilation. Never combine metadata stderr progress with JSON stdout.
No lock version drift or new registry version is allowed. The unchanged official
CLI build can supply core compiler evidence; separately verify the actual new
harness compiler/artifact and binary before running it. Target/cache reuse is
only an optimization, not proof. Index::load's own no-deps metadata is separate
from this harness dependency admission and the plan's metadata witness.

Twenty pure synthetic regressions pass, including fresh-root rejection, lock
subsets and bad dependencies, exact source/target/member/graph admission and
strict output controls. Synthetic seed/source pins are explicitly patched only
inside those tests. This does not prove real Cargo adaptation, actual library
typing or an observed poll_remote fingerprint. No workflow or trigger is added.
