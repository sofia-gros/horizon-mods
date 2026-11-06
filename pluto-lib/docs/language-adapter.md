# Internal language loading and prepared assets

Source candidate based on verified central local `8715b123` / published
`89744814b8d536a117e82f8ea3c8cdc9798faa95`. Targets official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. This group adds one client-lib support
module and no hook/state/locale owner. It is separate from authoring CI and the
unapplied contact/effect proposals.

## Internal adapter, not a new resolver

`horizon_client::pluto_language` delegates to the actual
`horizon_client_core::lang` and native `Text`:

- `load_ordered(sources, code)` returns a detached native `Language`. It neither
  changes selection nor installs the result. Sources must already be admitted and
  supplied in their native order.
- `get`, `get_or`, `has`, `code`, and `tr` retain the official lookup, scoped-language,
  missing-key and formatting behavior.
- `text` and `text_or` call native `Text::translate` / `translate_with_fallback`
  once. They return the same native Text type, resolved now. A retained span
  snapshot does not automatically follow later language changes. Native component
  source and translation identity are retained; they are not stripped by Pluto.

The Mod-author key holder/facade is a separately owned follow-up delegating here.
This module intentionally exposes native source/value types at the internal
compatibility boundary. It introduces no independent dictionary, `CURRENT`,
locale setter, reload listener, key aliases or format converter. Host dependencies
remain inside this adapter, rather than being copied into consumer gameplay.

JSON loading, merging and fallback remain official: English then selected code;
namespace order outside source order; no inferred regional fallback. Missing
selected values retain English, then `get` yields the key and `get_or` its explicit
fallback. Deprecated keys keep the native post-merge rules. Unsupported `%d`
formats are not rewritten to `%s`.

**This does not change pack priority.** It consumes the exact existing ordered
sources. It does not reverse-merge a flattened root or restore hidden pack files.
The current pipeline builds resource-pack merges and filters, then overlays Mod
content files; a same-path Mod language file can replace a pack language file
whole. A future change to that pipeline needs its own compatibility decision and
actual overlay tests. See the separately published language-resource audit
`d92b7046f86789103f27e7a7905e1a0bb7cc377a`.

## Optional preparation-time TOML

`tools/prepare_language_json.py input.toml new-output.json` uses Python 3.11's
standard `tomllib`; it adds no Rust dependency or runtime TOML language support.
The official lock contains a TOML package used elsewhere, but neither client-core
nor the existing Pluto language adapter needs a new dependency edge.

The input must be one flat table of strings. Quote dotted translation keys:

```toml
"fluid.meadow.rain" = "雨 %s"
```

The tool preserves key spelling, Unicode (including non-BMP characters), values
and all placeholders. It does not infer a namespace prefix, flatten nested tables,
coerce numbers or invent fallback values. TOML duplicate keys are rejected by
`tomllib`. The generated JSON contains one entry per input key; JSON input is not
accepted or normalized by this tool. Existing native JSON duplicate-key/lenient
parsing behavior is consequently unchanged, not made stricter by this helper.

Preparation limits are explicitly 1 MiB input and output and 16,384 keys. These are
tool budgets, not a claimed limit of the native language API. Validation finishes
before output publication. A complete sibling temporary file is linked to a new
output name; existing files and symlinks are not followed, truncated or replaced.
Unsupported hardlink filesystems fail instead of falling back to overwriting.
The tool does not create missing directories, install a package or update settings.

Consumers put the resulting JSON in their own `assets/<owned namespace>/lang/`
content package, normally both English and a selected supported locale. Pluto's
source-only `.hnmod` still contains no consumer assets. The content manifest,
`registries` capability, native package admission and host resource discovery are
separate prerequisites. A new locale in a Mod namespace alone does not satisfy
native locale-menu discovery's `assets/minecraft/lang/<code>.json` existence gate.

## Evidence and remaining validation

Executed locally, with no native tools: four source guards and eleven Python
preparation tests passed. They cover the exact module entry with no language hook/state, exact native
delegation, Unicode/quoted keys/placeholder preservation, duplicate/nested/nonstring
rejection, budgets, existing-file/symlink protection and repeat-output conflict.

Eight Rust tests are included in the actual module but **unrun**. They use the real
client-core `Language`, `ResourceSource`, `scoped` and Text rather than copied host
types. They cover source/namespace priority, English/key/explicit fallback, detached
loading, unknown-region behavior, lenient/partial file semantics, formatting and
custom-locale discovery limits.

`experiments/language-resources/prepare_fixture.py` can prepare a tiny crate that
path-imports this exact module and depends on the approved actual client-core.
It checks committed input hashes, approved clean host identity and seeds the
host lock. It never invokes Cargo. In a separately granted bounded slot, the
controller must seal the complete official tree, resolve offline, verify the lock
allows only official package identities plus this fixture, then run locked/offline
and verify again. Use the continuous 10 GiB + 128 MiB guard and shared job limits.
No fixture preparation, metadata, compilation, plan or apply has run in this group. The source guard checks
this entry structurally, so unrelated later module additions do not fail it; the
isolated manifest hash is provenance, not a false requirement on a combined bundle.
Source review caught inner `//!` comments that the actual Mixins module parser rejects;
they were changed to plain comments before freezing, and a source guard now rejects
inner module attributes. This is a source correction, not an observed planner failure.

Still required before claiming real asset distribution/display:
1. Actual `Package::write_files` → `Package::read` → `Content::read` for a minimal
   `.hmod` containing owned English/Japanese assets, then the actual Language loader.
2. Native pack overlay then native Mod content overlay cases, including filters,
   duplicate paths, malformed JSON and missing keys. Pure loader tests do not prove
   this different pipeline.
3. The real consumer item/menu label with selected-language changes and re-resolution,
   coordinated with the item and Guide owners. No missing-resource skip counts as success.
4. Complete packaged module planning/type checks on the selected official target.

No TConstruct policy, contact authority, save data, effect lifetime, official source
checkout or existing production locale settings is changed here.
