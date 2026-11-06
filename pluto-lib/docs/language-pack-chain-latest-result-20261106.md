# Latest official Pack8 result

**PASS, baseline only.** [First CI run 42206862972](https://github.com/sofia-gros/horizon-mods/actions/runs/42206862972) completed successfully at exact reviewed `dfd09ee78c98ce6603571c749a4b070f631d7068`, job 127855618585. Its fixture was frozen at `15397f42`; later repair-design docs were not included.

Target: official Horizon `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`. No Mixins apply or official source modification. The trigger remains frozen; no retry was used.

## Observed evidence

- All eight named native tests passed once, zero ignored/measured/filtered.
- Thirty runner checks and ten selected-source checks passed.
- Cold compilation produced actual rustc witnesses for client-core, mods, client-lib, renderer and the fixture test crate. The compile phase took 196.602 seconds.
- All 14 phases exited 0. The final receipt has passed=true, verified=true and error=null, with all five required gates.
- The official lock, exact three local patches, 439 resolved package identities and all actual local manifest paths were validated. Source/ignored-file/link-target, fixture/lock/metadata and generated OUT_DIR seals stayed unchanged.
- Minimum available space was 88,549,400,576 bytes, above the 10 GiB +128 MiB floor.

## What passed

The original six cases verify actual native selected-pack order, a supplied server pack above selected packs, filtering with and without a self replacement, definitely invalid input, and English/exact-selected-locale behavior. They also show that a later Mod file still replaces the whole prepared locale and loses absent lower-only keys.

The two latest cases verify that a #comment language pack is retained through the native lenient fallback, then still replaced by a higher Mod file; and that the public language reader converts numbers/booleans to strings while strict pack JSON preserves scalar types before Language loading. The lenient pack path uses string values. Literal placeholders remain unchanged, and invalid input does not clear an existing caller-owned map.

## What did not pass by implication

This is not a priority repair, Game/UI result, full installed Pluto/TConstruct composition, production asset proof or world-fluid activation. It does not execute Mod resource_root policy transition/pruning, prove a cache hit, install locale settings, refresh retained Guide/LAN state or supply contact-effect actor authority. Later repair gates remain in [the unimplemented design](language-overlay-latest-design-20261106.md).

The old 624c overlay success and its first failure stay separate. Old Pack6 `55e4d1d0` remains unexecuted; the new execution does not rewrite its history. Complete per-phase logs were validated inside the runner before bounded console summaries. Their recorded hashes do not imply raw ephemeral files can be recovered later. See the companion JSON for exact receipt, names, phase hashes and compiler witnesses.
