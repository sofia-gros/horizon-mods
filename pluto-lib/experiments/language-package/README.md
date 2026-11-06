# Owned language package boundary: unexecuted actual-crate fixture

This is a separate source candidate, not a change to the frozen 22-test
Language/Key/item CI. All eight Rust cases are **uncompiled and unrun**. No native
command, package creation, official checkout edit, source stage or dependency
resolution was performed while preparing this group.

## Actual path under test

Use official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` crates:
`horizon-mods` without optional `index`, and `horizon-client-core`.
`Package::write_files` writes a tiny content-only `.hmod`; `Package::read` parses
its actual manifest; `Content::read` validates its paths/capability/ownership;
`Content::file` supplies exact packaged bytes. A test-only `ResourceSource` view
passes those admitted bytes and namespace names into actual `Language::load`.
That view has no language parsing, merging, formatting, settings or dictionary.
An unreadable listed file panics rather than quietly returning None. No missing
fixture resource or environment condition is a successful skip.

The test content is original minimal `meadow` data in
`assets/meadow/lang/en_us.json` and `assets/meadow/lang/ja_jp.json`, with literal
Japanese text and untouched `%s`. It is not TConstruct data or user assets.
No package is installed into the user's game. The dummy code bytes in the
missing-capability negative case satisfy only Package's byte-presence condition;
no WebAssembly loader or execution is involved.

## Eight distinct cases

1. Package round-trip retains both exact locale byte strings and resolves both
   languages, English-only keys, raw formatting templates and absent keys.
2. Unknown exact `ja` retains English despite packaged `ja_jp`; no guessed alias.
3. Actual Content rejects a language file in an unowned namespace.
4. Actual Content rejects language-only assets without `registries`.
5. An admitted TOML asset does not become a runtime language resource.
6. Content admission alone accepts malformed language JSON; actual loader then
   retains English. This is not the pack layer's strict merge behavior.
7. Custom-owned language assets alone do not establish locale-menu discovery.
8. Losing an already admitted package is an error, not a silently passing fallback.

`inputs.json` pins the actual dependency and source files, fixture bytes and all
eight fully qualified test names. `Cargo.toml.in` is a source template only. The
controller must replace its explicit official path, seed the official lock,
allow only that lock's package identities plus `pluto-language-package-fixture`
0.0.0, seal the entire clean official tree/file set, compile, check the exact
test inventory, run all eight once, and verify source/lock/generated outputs
again. A passing source check is not that execution. No runner is dispatched.

## Deliberately excluded production steps

`Game::content_root` does more: enabled/dev selection, duplicate-id checks,
`known_components`, native declarations, `accept_mod`, content overlay creation,
registry registration and announcement. The fixture does not mock or bypass
those steps and does not claim them. Public Content APIs do not directly
implement ResourceSource; the explicit view here is test wiring, not a newly
installed runtime connection.

Native pack merging, filters and subsequent Mod whole-file overlay need their
own real overlay gate. Source-only findings and a compatibility proposal are
in `docs/language-overlay-compatibility.md`. Full host typing, actual installation,
settings changes, reload, retained UI, LAN and visible glyph output remain open.

## Source check result

Five Python checks passed against the exact official source bytes. The initial
source-check run found only a guard-count typo (seven excluded categories versus
the actual six); after correcting that assertion, all five were rerun and passed.
These checks do not execute any Rust or create a package. The eight native cases
remain uncompiled and unrun.
