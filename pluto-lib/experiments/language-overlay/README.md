# Actual content-overlay baseline candidate

**Source-only: eight Rust cases are uncompiled and unrun.** No runner is dispatched.
This is a separate boundary from the already-passed Package/Content/Language
eight-case proof at `5a577f12`. It makes no production change and introduces no
locale owner, dictionary, merger, event, setting or consumer callback.

## Actual owner, minimal inputs

The fixture imports public `horizon_client_lib::mod_content::{build, ContentMod}`
from exact official Horizon `624c4f62`. Packages and content are admitted by real
`horizon_mods`, and the final output is read through the actual native PathBuf
ResourceSource and Language::load. No alternative parser or modeled overlay is
used. The dependency is the real client-library graph, including its index
feature dependencies; this is not a small core-only execution or a Game harness.

The fixture creates one original air row in facts.json, a matching gameplay row,
items.json as an empty object, and its own tiny locale files. These are the
table shapes read by Plan::new, tables and Known::new for an asset-only build.
They are not production or playable game assets. Exact source hashes, dependency
template hashes and all eight names are in inputs.json.

## Contracts under test

1. An admitted Mod replaces the entire same-path lower locale file. Conflicting
   keys are Mod-wins, lower-only keys disappear, and base/package bytes survive.
2. An untouched selected locale survives from the base. English fallback comes
   from the resulting output, and `ja` does not guess `ja_jp`.
3. Different namespaces retain native language namespace ordering regardless of
   the supplied Mod order. This is not another global priority algorithm.
4. Two admitted Mods owning the same extra namespace are rejected before output
   recreation. There is no same-namespace Mod-versus-Mod last-wins promise.
5. Malformed Mod locale bytes are copied verbatim; native Language fallback
   applies afterward. This does not make the content stage a strict JSON merge.
6. Loss of an admitted package fails after output recreation. The test explicitly
   does not claim atomic rollback or preservation of the previous output.
7. Missing mandatory base schema fails during Plan creation, before output reset.
8. A second build recreates only the fixture-owned output, removing its earlier
   Mod file and restoring the untouched lower file.

Every output is a child of an exclusively-created fixture directory. Rebuild and
negative cases deliberately use a previously populated directory owned by that
same fixture. No existing user or host output path is accepted. Cleanup removes
only the owned scratch tree, and source-base bytes are compared explicitly.

## Limits and next validation

The lower directory represents an input already prepared by native pack handling;
the fixture does **not** run selected/server pack merging, pack filters or the
private renderer merge helper. Explicitly loading two locale codes is not changing
Options or CURRENT. Game admission, live reload, cache identity/install, GUI,
retained Guide/LAN updates and gameplay remain outside this baseline.

Only source guards may run now. A later separately reviewed CI must use sealed
official inputs/lock/metadata, actual client-library compilation, the exact eight
tests without skips, and final source/output integrity checks. No native job,
Cargo metadata, Mixins operation or stage copy is authorized by this source file.
Existing runtime whole-file behavior remains unchanged until this baseline and
the compatibility decision are reviewed separately. See
[the design](../../docs/language-overlay-compatibility.md).
