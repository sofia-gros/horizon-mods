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
