# WIP: latest official resource-pack to Mod language chain

**Source only: eight Rust cases are uncompiled and unrun. No runner or trigger
is created by this unit.** The six original cases are preserved byte-for-byte
inside a separate fixture; neither the old Pack6 source at `55e4d1d0` nor the
passed overlay8 at `ce495a88` is changed or relabeled as latest proof.

The target is official Horizon `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`.
This fixture is not a Mixins bundle. It calls actual public `packs::folder`,
`enabled`, `server_pack`, `build_overlay`, Package/Content, `mod_content::build`,
`Language::load` and the newly public `read_lang_file`. It does not copy a
private merger or create a process-wide locale registry.

## Eight intended cases

The first six retain selected-pack order, supplied-server order, filters with
and without replacement, definitely invalid `not json` bytes, and unchanged
pack-output/selected-locale behavior. Case five is not a claim that every
non-strict input is invalid: the latest native pack merger now calls the
lenient language reader for Lang when strict object parsing fails.

7. An original `#`-comment pack is read by the public native reader and retained
   by the actual pack merger, overriding a lower conflict while preserving a
   lower-only key. A following Mod file still replaces that entire locale.
8. The public reader converts numeric and boolean values to strings for both
   strict and commented input. Actual pack output, however, retains JSON scalar
   types on its strict-object path and uses strings on its lenient path. Native
   Language resolves both equally. Literal `%s`/`%d` text is not rewritten;
   invalid input must not clear an existing caller-owned map.

Case eight checks fixed ASCII fragments in the native compact JSON output;
it neither reparses the file through a substitute parser nor promises arbitrary
JSON formatting preservation. The actual source is pinned, and any serializer
change must be reviewed rather than silently changing the oracle.

All paths are exclusively-created fixture scratch. Inputs/base/Mod archives
are checked for preserved bytes where applicable. Native public pack cache
validation stays in use: repeated equal output does not prove a cache hit.
The new Mod stamp-prefix root, stamp aging/pruning, `live_switch` and Game owner
are deliberately not invoked. No existing user cache directory is touched.

## Dependency and source contract

`inputs.json` pins 24 actual official files by Git blob SHA, size and SHA256,
including the new official lock, four direct crate manifests, native loaders
and all three official root local patches. `zip = 9.1.3` is already in that lock.
The latest client-lib adds a transitive cosmetics dependency; no local metadata
or compilation has resolved the resulting graph here.

`test_source.py --source-root PATH` checks only exact selected file bytes and
contract structure. PATH may be the small official connector export. This
check does **not** certify a complete or clean Git checkout, dependency closure,
native type compatibility, package application, or native execution.

A future reviewed CI preparer must require a clean complete official checkout
at the exact revision and URL, seed its lock, inherit the exact three root
patches, and check every resolved package identity and actual local manifest
path. It must reject registry substitutes, symlink/alternate-root redirects
and new identities other than this fixture. Real core/mods/client-lib/renderer
and fixture compilation, all eight exact names once with zero ignored/filtered,
and complete source/lock/metadata/generated input seals are separate gates.
No runner/metadata/Cargo command is added or run by this source unit.

## Exclusions

Priority repair and lower-key retention remain unimplemented. Existing native
selected/server/filter precedence and Mod whole-file replacement are the
baseline, not defects silently repaired by these cases. Game admission/pack
settings, locale installation, live UI, retained Guide/LAN refresh, production
assets, cache-policy invalidation and flow/contact effects remain unproved.

The Python validation receipt is source-only. Old successful evidence and the
prepared old Pack6 CI stay immutable until separately reviewed latest work is
granted an execution slot.
