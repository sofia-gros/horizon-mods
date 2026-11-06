# Latest official language and fluid delta audit

Source-only comparison of official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` → `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`, 2026-11-06. Latest Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57` is a separate central audit. No fork access, official edits, native run, CI trigger or compatibility claim.

## Important changes

1. **Pack language fallback changed.** [Native read_lang_file](https://github.com/horizon-rust/horizon/blob/f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd/crates/horizon-client-core/src/lang.rs) is public, with unchanged parser body. [Pack merge](https://github.com/horizon-rust/horizon/blob/f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd/crates/horizon-client-render/src/packs.rs) now falls back to it for Lang after strict object parsing fails. A language layer with # comments is no longer dropped. Strict object values and lenient string conversion remain separate paths. The old 624c strict-pack/direct-lenient distinction must not be presented as the latest behavior.
2. **Mod whole-file overlay remains.** [mod_content](https://github.com/horizon-rust/horizon/blob/f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd/crates/horizon-client/src/mod_content.rs) changes resource_root, not build/Overlay::write. Its root is now cache/mod-content/<first 16 stamp characters>; a hit refreshes the stamp, and later builds prune other stale roots. live_switch distinguishes live asset/data changes from registry/worldgen changes. A key-retention repair and cache identity change are still proposals, not implemented fixes.
3. **Game reload has a new applied/live content owner.** Reload uses applied_mods and calls mods_reloaded after apply. Existing load_language and startup selected→server order are unchanged in reviewed diffs. This does not prove retained Text/Guide rerender, remote server authority or live locale activation.
4. **Motion/contact order changed.** on_climbable is new mutable state; climbable updates it. EnterBlock now happens before the damage_only continue; powder-snow melt is an event. Yet inside/step/fall/contact/consume forwarding functions are byte-identical and the inside hook still lacks a Game borrow. New source does not supply the missing synchronous effect authority.
5. **Tank acceptance is a new official extension.** native_block_entity_accepts_fluid filters insertion and destination room before movement drains. This is a possible reusable tank seam, not an immersion/contact effect hook. No Pluto activation was made.
6. Potion consumption adds stat/criterion calls; thrown harming potion handles Enderman teleport separately. Asset/motion JSON loading also changed. Old whole-file guards and exact-order assumptions require fresh review rather than hash replacement.

## Coverage and evidence

The complete official recursive tree contained 5956 entries and was not truncated. 33 selected files were compared with the old official Git objects: 13 changed and 20 were blob-identical. Full changed bytes were checked against those tree blob IDs; SHA256 and source links are in the companion JSON. The generic GitHub compare response stopped at 300 files and was not treated as exhaustive.

Core Text; renderer fluid mesher/pack cache; world fluid, flow, occupancy, shape, registry, level and entity-fluid kernels; native item-fluid helper are among the identical files. Identical bytes do not prove compatibility through changed callers. Remote.rs review was limited to relevant changed hunks, and native_hooks review to selected contact/consume/tank functions. This is not a full gameplay or dependency audit.

## Preserved cutoffs and next bounded proposal

- Prepared [Pack6 source 55e4](https://github.com/sofia-gros/horizon-mods/commit/55e4d1d0ef3b4021472223a9e196260d771a5327) stays unchanged, **unexecuted and held**. Its trigger has not been created.
- [Overlay8 success](https://github.com/sofia-gros/horizon-mods/actions/runs/42195249104) remains 624c evidence only. It proves actual Package/Content→mod_content::build→Language fixture semantics, not selected-pack merge, repair, latest host or Game/UI.
- A later latest-target Pack6 successor should keep the six baseline cases and add an actual lenient-comment layer plus strict/lenient value cases. It must pin latest dependencies/lock/local patches and use the real public merger; no parser clone or silent priority change.
- Reassess latest resource_root/cache pruning and live_switch before any writer repair. No mutation of existing cache directories or override of native lifecycle authority is proposed.
- Central latest plan-only diagnosis comes first. No runtime/guard/source migration or new CI execution was performed for this audit.
