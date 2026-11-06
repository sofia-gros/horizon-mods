# Language file replacement: two different boundaries

Source-only follow-up, official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`, target MOD baseline
`8715b123` / published `89744814`. No runtime merge rule is changed.
The thin Pluto adapter preserves the current loader and priority. The unrelated
22-test Language/Key/item fixture does not exercise these overlay boundaries.
Exact official source hashes are in `experiments/language-package/inputs.json`.

## Confirmed current behavior

1. **TConstruct preparation:** `tconstruct/tools/package_hmod.py:96-113` keeps
   a tracked locale file when an imported visual archive contains the same path.
   The archive file fills missing paths only. This can discard archive-only keys
   in that locale, but the tracked file is intentionally authoritative for
   Horizon additions and `mark_unavailable` is consumer policy. This is not a
   native resource-pack priority rule.
2. **Native runtime:** `game/packs.rs:60-70` orders selected packs, then server
   packs. `horizon-client-render/src/packs.rs:620-724,880-918` applies filters and
   merges same-path language JSON objects bottom-up, higher values winning.
   `game/packs.rs:497-516` then calls Mod `content_root` on that prepared pack root.
   `mod_content.rs:2461-2496` sends ordinary language files to `Overlay::write`;
   `mod_content.rs:2113-2173` detaches the output link and writes a complete file.
   A Mod locale replaces the flattened pack locale, including lower keys absent
   from the Mod file. It does not modify the original pack/base file.
3. **Loader fallback:** only after those steps does `Language::load` load English
   and the exact selected code. It cannot reconstruct pack keys already removed
   by whole-file replacement. Namespace ordering and deprecated-key rules still
   apply; a new locale registry or a second dictionary is not a repair.

## Smallest solutions without pretending priority is unchanged

**Preparation-only completeness:** a separately reviewed TConstruct packaging
change could combine original source locale keys first, then tracked additions
last. A generic preparation helper may accept ordered flat string catalogs and
write a new output file, with existing output never overwritten. TConstruct
retains source selection, unavailable-entry policy and format decisions. This
would recover archive-only keys while keeping tracked conflicts authoritative.
It would not fix runtime pack-versus-Mod replacement. No such merge helper or
consumer change is implemented in this checkpoint.

**Runtime key retention with existing conflict precedence:** a narrowly scoped
future native adapter could merge an existing already-filtered pack-root language
object as lower input and the admitted Mod language object as higher input, then
publish through the existing overlay writer. This keeps current Mod-wins conflict
precedence and preserves surviving lower-only keys. It still changes whole-file
replacement semantics and must be labelled as a compatibility change. Reading
raw packs, reversing a flattened root, or restoring pre-filter files is forbidden.

The authoritative pack merge helper `merge(Merge::Lang, ...)` is private in the
renderer crate. The content overlay and its writer are private to a different
client-library module. No public reusable call between those owners was found.
Do not copy a second merger into the lookup facade: first prove a narrowly pinned
module/adapter placement that can delegate to the real helper, and guard the exact
call site plus full installed composition. Invalid whole JSON layers, nonstring
values, duplicate keys, filters and I/O failures need explicit preserved behavior.
The current direct Language loader is lenient while the pack merge is strict;
changing the higher Mod layer to strict parsing would also change behavior and
must not be hidden as a formatting-only refactor. This proposal is therefore
not yet an implementation-ready production fix.

**Resource packs overriding Mod defaults** is a different policy: Mod defaults
would need placement below the selected/server packs before native filtering and
merging, with matching startup/reload/cache identity. The current flattened
post-pack Content root cannot provide that provenance. It is not safe to reverse
merge the final root or install another CURRENT. No such ordering change is made.

## Non-skipping proof gates before any activation

- First run the separate actual Package→Content→Language eight-case fixture.
  This proves packaged bytes and public admission only, not the game pipeline.
- On an admitted real overlay harness, establish native baseline: lower-only,
  higher-only, conflicting keys, same and different namespace ordering; selected
  pack versus server pack; filtered lower path; malformed strict JSON; duplicate
  keys; nonstring entries; explicit deprecated entries; missing selected locale.
- Then compare the proposed path with expected differences only. In a key-retention
  variant, conflict values remain Mod-wins, only surviving lower-only keys change.
  Define malformed/lenient Mod behavior before applying, not after a failing test.
- Verify native overlay errors preserve existing fallback behavior and do not
  mutate source roots, plus cache invalidation, selected-code recheck and reload
  installation through the one existing owner.
- Prove full actual module/host typing separately. There is no current live UI,
  runtime overlay, game admission or installed-package result from this study.

The user-facing API can remain a thin `key(...).resolve(...)` over the native
owner. None of the proposals adds language ownership or changes contact/effects.

## Asset-only native baseline fixture, still unrun

The separate `experiments/language-overlay` source candidate imports actual
public `horizon_client_lib::mod_content::build`, real Package/Content admission
and the native PathBuf Language resource view. Eight Rust cases are prepared,
not compiled or executed. Six source guards passed against exact official 624c.
The successful Package8 result does not count as overlay execution.

Two native phase constraints are explicit in this candidate. `build` rejects
overlapping owned namespaces before creating the overlay: Mod-wins means Mod
versus the prepared lower root, not same-namespace Mod-versus-Mod last-wins.
`Overlay::new` removes and recreates its output after Plan creation, so a later
package read error is not an atomic rollback promise. The fixture can exercise
that behavior only inside its own exclusively-created scratch tree.

Minimal original facts/gameplay/items tables suffice for the asset-only source
path; they do not represent playable assets or selected/server pack preparation.
The real client-library dependency makes eventual compilation a separate native
gate. Options/CURRENT mutation, cache installation, live reload and actual UI
remain outside this fixture. Guide retains ownership of those caller/lifetime
questions; no second locale owner or priority repair is introduced here.

## Locale transitions still owned by the official caller

At the same exact 624c source, `game/options.rs:391-393,465-471` dispatches
Language to Game::load_language: load from options.resources, install through
the native owner, refresh remote-language state, rewrap chat, then forget cached
creative item names. `game/startup_menu.rs:523-529` is a different path: load
from menu.root and save menu settings. The source hashes for both callers and
remote.rs are now pinned in the baseline ledger; none is changed or executed.

`game/packs.rs:441-449` reads the current selected code again before installing
a staged reload. If the selection changed during preparation, it reloads the new
root using that current code instead of installing the stale selection. A reload
with the same code can still change strings, so a code-equality check is not a
translation-generation or retained-screen invalidation contract.

A new wrapper around lang::set alone would not execute all those owner-specific
settings, remote and UI steps. The existing Key.resolve uses the single native
current language each time; an already-resolved Text remains a span snapshot.
No new locale setter, independent generation counter, event or retained Guide
refresh is added here. A later UI proof must enter the actual appropriate caller
and preserve its load/install/refresh order, including the stale-reload selection
case. Neither Package8 nor this unrun overlay baseline is that proof.

## Bounded repair candidate after the baseline gate

Source-only design follow-up; neither an implemented merger nor an approved
activation. The reviewed baseline CI remains frozen at `14902fcc`; this section
is not an input or a result of those eight Rust cases.

A potentially smaller compatibility domain is **two strict JSON objects whose
values are all strings**. The native `read_lang_file` fast path already accepts
that domain into `HashMap<String, String>`. For a same-path, eligible locale:

- Read only the surviving prepared base path, never raw packs or a pre-filter
  source. Keep current admitted Mod-over-prepared-base conflict precedence.
- If both layers are accepted by the exact native fast-path domain, merge lower
  then higher and serialize an object of strings. Duplicate keys must resolve as
  that native deserializer does, not a new duplicate rejection rule. Values,
  UTF-8 strings and placeholders are unchanged; key order/formatting bytes need
  not be preserved in the new output.
- If either layer is absent, unreadable, non-object, lenient-only, nonstring,
  over a declared preparation budget, or otherwise unsupported, retain the
  existing higher-file bytes and the native writer behavior. This conservative
  fallback **does not repair missing lower keys for every historical file**.
  Failure to read an admitted Mod package must still fail in its current phase;
  do not turn that error into an absent language file.
- Match only the intended single-locale leaf under `assets/<ns>/lang/`; explicitly
  exclude `deprecated.json`, metadata and other assets. No new namespace clash
  policy, broad file interception or content admission authority is implied.

This changes the intentional behavior of `{}` from removing a lower catalog to
retaining it, and preserves lower-only keys that previously vanished. It must be
named and reviewed as a compatibility change even within its bounded domain.
It cannot be described as resource packs overriding Mod defaults. A candidate
that promises that different ordering still needs the earlier pipeline design.

Do not implement this candidate by serializing `Language::load(...).map()`.
That map includes bundled strings, English fallback, all namespaces and the
post-load deprecated-key transformation; it is not the raw one-file layer.
Do not run a new parser in the public Key/Text facade. A future adapter remains
inside Pluto's pinned native compatibility layer and should reuse the existing
native parse/merge ownership where possible. The private renderer merge helper
and private client overlay still require actual placement/borrow/composition
proof. No new helper, callback, global setting or dictionary is added here.

### Cache hits are a separate required boundary

At `mod_content.rs:2711-2788`, `resource_root` returns the existing output when
its stamp matches, without calling `build` or `Overlay::write`. The stamp includes
the executable's metadata, prepared `horizon-resources.json`, rewritten table
metadata and Mod source metadata/declarations. It is **not** an explicit identity
for a newly proposed runtime merge policy. A write-only hook cannot prove a
previously cached root was rebuilt. The private `game/mods.rs:70-179` owner also
keeps admission and fallback to the original root; the merge must not bypass it.

Before activation, demonstrate a policy/version transition cannot silently reuse
the old whole-file output; same-policy reuse still behaves as native; and a
failed rebuild keeps the existing owner error/fallback contract. Do not delete
user caches to make a test pass. The baseline8 calls public `build` directly, so
it does not supply this resource_root/cache evidence. Dynamic toggles would need
an explicit policy identity in the actual owner path; no toggle is proposed now.

### Exact comparison matrix for a later prototype

1. Flat lower/higher conflicts, escaped/Unicode strings, placeholders, duplicate
   keys and empty objects: higher values are identical to native baseline;
   surviving lower-only keys are the explicitly intended difference.
2. Lenient syntax, BOM, numbers/booleans, null/array/object entries before and
   after valid keys, malformed bytes and size-limit edges: exact higher bytes
   stay native, including native partial-file/abort behavior. No silent strict
   conversion and no success-by-skip.
3. A filtered lower path stays absent; selected/server order remains the native
   renderer's order. These need the real pack overlay before Mod overlay, not
   the current baseline's fabricated lower root.
4. Namespace conflicts and admitted-file I/O failures retain their existing
   before/after output recreation phases; only fixture-owned output may change.
5. Cache hit/miss and policy transition, then startup and staged reload under a
   changed or unchanged locale code, use their actual owners. Retained UI and
   remote refresh still require the independent Game/UI proof.

Exact source for this follow-up is official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`:
- `crates/horizon-client-core/src/lang.rs:397-475`, SHA-256
  `7384b98b80acdea2540ca46c5cd1b608c5b5361abe8c83da9edd372b7c9afe5d`.
- `crates/horizon-client/src/mod_content.rs:2113-2173,2445-2496,2711-2788`, SHA-256
  `8f4121d5a0a045ea2796d79f3548efd7436859e2648be99d6504a10f8368ec93`.
- `crates/horizon-client-render/src/packs.rs:620-724,880-918`, SHA-256
  `c7212c509709788e4276fec1dba65531428033981df4b9ee102b3f62a48e29ed`.
- `crates/horizon-client/src/game/mods.rs:70-179`, SHA-256
  `9bee92a0108eebf4077e446c22740e7b68930527f428ed874e1f4bff01076ffb`.
