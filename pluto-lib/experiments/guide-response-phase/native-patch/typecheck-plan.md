# Actual Game typecheck: separate proposed gates

Status: **source preparation only**. No new workspace copy, Cargo metadata,
host application or check has run. The seven helper tests are not a substitute
for compiling the actual Game field and its BookScreen callers. Ordinary LAN
handling and installed Pluto guards are unchanged.

## A. Pristine official-only type experiment first

Use the exact official Horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0
workspace and lockfile, with the already frozen `prototype.patch` from 43178aef.
The patch changes eleven host source files and no Cargo manifest or lockfile.
The pristine lock SHA-256 is
`463913123985e344f2889e5beae65982773e6dbfacdcac7a916bcec522a5bd31`.

The target would be the real `horizon-client` library and its thin named binary,
with `--no-default-features`, `--offline`, `--locked`, debug information disabled,
incremental disabled and one job. The real `game` module, native BookScreen,
Text, EditBox and all ordinary production callers must be compiled together.
Do not extract only the enum, replace the Game with a mock, construct a server
fixture, or treat a standalone helper compile as this gate. This is typechecking,
not execution of a Client, renderer, audio, networking or lifecycle test.

The authorized official checkout must stay immutable. Only a separately
approved temporary experiment workspace may receive the diff. Its complete
baseline and post-delta seals must show the exact eleven-file change and no
missing original source. No patch is applied by this directory's generators.

## B. Metadata and capacity gate before any check

The proposed pristine experiment cannot inherit v15's cache-success claim.
Read-only manifest comparison found that v15 adds two native API workspace
members, a `pluto-api` path dependency to horizon-client, and a
`pluto-inventory-api` path dependency to horizon-client-lib. Its Cargo.lock also
differs. Reusing a logical path would not erase those dependency graph changes.
The core/world/items/render manifests happen to match, but transitive identity,
features and generated build-script inputs still require verification.

After a separate grant, collect locked/offline metadata into temporary output.
Compare local package IDs, manifest paths, target source paths, workspace roots,
dependency edges, resolved feature sets and lock inputs against preserved v15
metadata. A no-dependencies metadata result establishes local identities only,
not the full dependency/feature graph. If the required metadata cannot be
resolved offline, stop and report rather than fetching or assuming cache hits.

The known pristine export has 117,567,239 apparent bytes, which is a source-copy
measurement, **not** an estimate of Cargo's output growth. At initial inspection
the overlay had only about 4 MB above the 10 GiB +128 MiB floor. The temporary
filesystem had about 3.42 GB free. Neither figure authorizes a copy or build.
An approved runner must budget the full source copy on its actual filesystem,
metadata/log outputs, and a conservative bound on nonreusable build artifacts;
unknown output growth is a blocker, not a zero-byte estimate. It must continuously
guard both filesystems and stop before the workspace floor. The preserved shared
target and v15 stage are not disposable or writable scratch space.

## C. Separate v15 composition candidate

The sealed ordinary v15 runtime is f5a83539525c26fd2142e26e43789a3dd0107033.
Its complete 5,047-file seal has SHA-256
`e16a3781aae1dea6d1a627f4bd28641469e16e822dae9704d0507a75a8d02428`.
This is a different input, not a convenient replacement for pristine source.

Mixins reprinted books.rs: the original one single-line and three multiline
`matches!(self.book, ...)` expressions became four single-line expressions.
The pristine generator correctly rejects that input. `stage_composition.py`
provides a separate, explicit four-site rule and checks every sealed stage file
before and after its **in-memory** transformation. The original generator,
helper and prototype remain hash-pinned and unchanged. Its stored
`v15-composition.patch` is also unapplied and excluded from installation.

Only game.rs, books.rs and native_hooks.rs among the ten existing target files
differ from pristine; the other seven still match. All other original rewrite
counts match. The complete appended Pluto helper tails in modified files remain
byte-identical, including current-book and the other Game modules. The current
book adapter's `game.book.as_ref()` call retains its Option-shaped return type.
These are source facts, not successful typing, alias analysis or compatibility
with arbitrary mods. Existing injected blocks outside the reviewed delta are
untouched; no new observer or reply callback is added.

Do not run ordinary guards against a patched host and call that an ordinary
package proof. The current-book guard intentionally pins pristine game.rs,
books.rs and remote.rs; changing those inputs requires a separately reviewed
composition proof, not weakening or refreshing its production hashes here.
Even a future successful v15-plus-prototype typecheck proves neither the full
TConstruct consumer nor the unchanged response HEAD's observer compatibility.

## Still missing after either possible typecheck

- Native owned-open acquisition, final opening validation and conditional reply
  mutation capability. An observed stamp is not an owner handle.
- Full legacy body/envelope and native Text compatibility, including the eight
  prefix bytes beyond the existing read-only observer ceiling.
- Proven retained refresh under overlays, cover preservation, identical reopen,
  attachment reset, delayed/duplicate reply handling and destruction/restoration.
- Actual Client/window resources and runtime tests, plus complete consumer
  composition. No existing LAN feature is disabled while these remain open.
