# Fluid display and language resources

Source-only audit, 2026-11-06. MOD baseline `8715b123` is published as `89744814`; official Horizon is `624c4f6265c02296fbe8a80f029dd0d987fb94c0`. The companion JSON pins 17 inspected files. No native language loader, content packaging, locale switch, screen, or GPU ran in this audit.

## Existing display path

- `tconstruct/mixins/tconstruct/source-melter-render.rs::fluid_name` normalizes the pinned flowing-fluid aliases, then looks up `fluid.<namespace>.<path>` through `game.pack.names.display_get`. The official `Names::display_get` first queries the current selected language, then its English name map. This path is already connected to selected-language text; another fluid translation registry is unnecessary.
- Native water/lava additionally use block-name fallback. Milk uses `fluid_type.minecraft.milk`, with a literal Milk fallback. That key is absent from the inspected TConstruct English/Japanese maps; this is not a claim about all vanilla or installed packs.
- The metadata-complete potion path uses known source-era `item.minecraft.potion.effect.*` keys. Unknown/incomplete metadata retains the generic display policy. Potion interpretation and fallback wording remain TConstruct responsibilities.
- `source-gauge.rs` creates translated fluid/format/contents/capacity Text values with nested arguments; milk uses a resolved literal. Native `Text::translate` resolves spans immediately while retaining a translatable component source for serialization. Retained local text is not automatically rebuilt after a locale change.
- Tank/copper-can tooltips insert resolved fluid names into their surrounding text. Guide page generation uses `Game::translate` for fluid names; those generated strings are snapshots, not a live translation binding.

Do not change machine identity, bucket IDs, saved NBT, source-fluid aliases or potion semantics while localizing display text. The current TConstruct IDs have flat paths. Its helper only replaces `:` with `.`, whereas native item-name derivation also replaces `/`; this audit does not establish a universal key convention for arbitrary mod fluid IDs.

## Counted resources

The reference contains 142 IDs: 70 still/flow pairs plus unpaired `tconstruct:potion` and `tconstruct:powdered_snow`. Alias normalization yields 72 display families. These counts describe the reference and language coverage, **not** registration of 70 working custom world fluids.

- Tracked `content/assets/tconstruct/lang/en_us.json` has all 72 fluid keys and 71 corresponding bucket keys.
- `content/reference/tconstruct-part-visuals.zip` contains `assets/tconstruct/lang/ja_jp.json`, with the same 72 fluid keys and 71 bucket keys. For example, molten iron is `溶融した鉄`, and its bucket is `溶融した鉄入りバケツ`.
- The bucket-key exception in both maps is `tconstruct:powdered_snow`; native vanilla bucket naming is a separate path.
- Japanese is not a tracked file under `content/assets`. `package_hmod.py` copies original archive language files only when that output path is absent, so tracked language files have priority. The tool-visuals archive contains no language files. This is source evidence for packaging behavior, not a packaged-artifact test.

## Shared library boundary

The inspected Pluto `fluid::content` creates ordinary block declarations, block states and models. It declares `item: false` and creates no language files or bucket item. The later render-material extension adds a model rather than a language dictionary; the Declaration API also leaves key policy to the author. `Definition` has independent fluid, block and bucket IDs, so deriving one from another is not a generic guarantee.

Mod authors should supply native JSON language resources in their content `.hmod`, with the keys actually used by their display callers and block/item declarations. Native implementation `.hnmod` data is not automatically an asset source. A thin author-facing lookup may delegate to the single native language service; it should not add a global fluid locale or silently change missing-key, formatting, pack precedence or retained-content behavior.

See the independent [language resource audit](https://github.com/sofia-gros/horizon-mods/blob/d92b7046f86789103f27e7a7905e1a0bb7cc377a/pluto-lib/docs/language-resource-audit-20261106.md) for loader order, picker eligibility, lenient native JSON versus pack merge, and the final Mod-content overlay. Flattened output does not preserve every source/filter decision, so a later reverse merge alone is not a proven repair of native pack priority.

## Reproduction and remaining proof

Run `python3 pluto-lib/experiments/fluid-language-audit/check.py --repo <MOD git repository> --official <clean official source>` with the pinned commit available. The checker reads Git objects and ZIP members in memory, verifies the 17 file hashes and counted key sets, and performs source-boundary checks. It writes nothing and does not compile or run Horizon.

Observed result: 142 IDs, 72 families, 70 pairs, 72 English and 72 Japanese fluid keys, 71 bucket keys in each map. Source inventory passed.

Still unproved here: actual content-package language membership, native resource overlay/filter behavior for that package, chosen-locale loading, screen refresh, per-guest locale output, rendered font coverage, CPU mesh behavior, and GPU display. The separate material CI and later terrain author facade do not establish these localization results.
