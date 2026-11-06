# Exact integrated playtest: full plan diagnostic

Complete plan accepted; all12 unchanged guards diagnosed:2 pass,10 refuse. Compatibility false; apply/build not executed.

[Run42231588813](https://github.com/sofia-gros/horizon-mods/actions/runs/42231588813), job127938548202. Controller `11794d5f90b274caf46e97b44330b41424ad98de`; integration `9cc1a01632913f01df1f0bcbb0b74b8bb9e80525`, tree `200cb8fcf0f22b16d82866bb67ea402d525489c9`.

- Run started: `2026-11-06T14:10:40Z`. FINAL_RECEIPT console timestamp: `2026-11-06T14:23:28.8777319Z`. GitHub terminal run updated_at: `2026-11-06T14:23:33Z` (not a separately measured job-end timestamp).
- Official Horizon41f822b6 and Mixins5a64af93; exact source/lock/CLI inputs retained in JSON.
- Actual plan accepted in570.20926098seconds; observed outer phase570.714seconds.322injections,321modules,111states,3crates,3bridges,1data entry.
- All12 guard processes completed:2 passed and10 refused. Protection and station-ingredient passed. All failures are retained below without changing guard code.
- Terrain collector observed2hooks/6modules with no competitors. This is selected identity evidence, not rendering type/runtime success.
- Receipt diagnostic_complete/source_verified/probe_completed=true, compatibility=false, error=null, apply_executed/native_type_verified=false.
-13gates/36phases; minimum free disk90,521,255,936B, minimum observed available memory13,473,435,648B.

## Exact guard results

- `check_protection_plan.py`: exit0. Protection ordering verified for the supplied complete plan.
- `check_advancement_plan.py`: exit1. advancement composition rejected: pristine official advancement source drifted: crates/horizon-client/src/game/advancements.rs
- `check_damage_equipment_plan.py`: exit1. damage equipment adapter rejected: official source drifted: crates/horizon-client/src/game/creatures.rs
- `check_client_payload_plan.py`: exit1. client payload observer rejected: native observer resolved location changed
- `check_recipe_placement_plan.py`: exit1. Recipe placement composition rejected: placement veto requires exclusive native target: horizon_client::game::recipe_book::Game<'_>::place_recipe
- `check_current_book_plan.py`: exit1. current-book adapter rejected: plan does not use reviewed current-book source: crates/horizon-client/src/game/books.rs
- `check_wire_snapshot_plan.py`: exit1. Wire snapshot guard rejected: Official private Wire declaration/source differs from the reviewed target
- `check_crafting_match_plan.py`: exit1. Crafting-match composition rejected: crafting-match veto requires exclusive native target: horizon_client_lib::crafting::Recipes::assemble_if_matches
- `check_terrain_binding_plan.py`: exit1. Terrain binding preparation rejected: Assembly function changed
- `check_cloud_insertion_plan.py`: exit1. Cloud insertion composition rejected: Official cloud authority/publication source drift: crates/horizon-client/src/game/lan_actions.rs
- `check_station_ingredient_plan.py`: exit0. Station-ingredient parser ownership, native fingerprint and helper source verified.
- `check_terrain_cell_plan.py`: exit1. Terrain cell adapter rejected: terrain cell plan input drift: crates/horizon-client-render/src/streaming.rs

## Package and plan evidence

- tconstruct_bundle: 8,187,083B archive, 503members, 11,534,126B unpacked, SHA256 `9bc1073220496d24435a013ece84ea01532e8e79c1a0e15b7b7395a96cb30c5d`.
- pluto_bundle: 270,764B archive, 148members, 776,500B unpacked, SHA256 `8adbd7e3f87d746582deceb604dbc953137eb7a87395359105a0f93d0e399a2e`.
- Combined:649members, 12,310,222B, canonical member-map SHA256 `65987a65817d2e11b5064a274d44b190abd8b0ce7a064d1040e9981e4595c5f1`.
- Plan:6,383,021B, SHA256 `7139236d6484c9cf5a862d3e8358d60b2b4e0bd3a8c4c0968c6d313735670f8b`. The complete raw plan is unavailable: post-terminal artifacts query returned `[]`; this hash is not a substitute for its contents.

## Preservation and limits

Full fetched console:240,383B, SHA256 `63a08676b4379806cf836b11b6a8a6e5dbec71186796c17ab9fa638a43c03e81`. Adjacent JSON embeds gzip/base64 console bytes and the complete receipt parsed in Python to preserve large integer fields. CPU/tool logs were reconstructed and checked against exact recorded byte counts/SHA256.

- The sole real plan accepted all649 composed payload files from the exact integration9cc1 packages; no apply, native type check, Game build or launch occurred.
- GitHub job success means diagnostic completion. Ten unchanged safety guards refused; this does not authorize apply or main integration.
- Guard wrapper phase exit0 encloses an independently retained actual guard exit0 or exit1; wrapper success is not guard success.
- The complete raw plan was not uploaded. The post-terminal artifact API returned an empty list. Its6383021 bytes are unavailable; digest/size cannot reconstruct the plan.
- Terrain selected2hooks/6modules are retained. Other unrecorded native node/fingerprint identities require additional explicitly authorized observation.
- Full package and combined member SHA maps are retained, but the source bytes/archive bytes are not embedded here. Nested support was compared in full and retained as count/bytes/map SHA, not every nested member.
- The full fetched console and exact receipt are retained. Raw verbose phase files remain unavailable; their hashes are not those files.
- Tool argv records witness invocation starts, not completion.115 CPU samples describe only the known direct CLI PID, not children totals or internal stages.
- Source is fixed9cc1, not later integration heads. Old624c/f4dc proofs are not promoted to41f; CLI Rust1.95 is not the Windows1.99.1 delivery proof.
- Chest pick color inheritance alone was deferred with user approval. Saved chest colors, schema and helper bytes were preserved.
- This docs-only closure does not alter main/integration/trigger refs or authorize an automatic retry.
